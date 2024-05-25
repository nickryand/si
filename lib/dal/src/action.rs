use std::collections::{HashSet, VecDeque};

use petgraph::prelude::*;
use postgres_types::{FromSql, ToSql};
use serde::{Deserialize, Serialize};
use si_events::{ulid::Ulid, ActionResultState};
use si_layer_cache::LayerDbError;
use strum::{AsRefStr, Display, EnumDiscriminants, EnumIter, EnumString};
use telemetry::prelude::*;
use thiserror::Error;
use veritech_client::{ActionRunResultSuccess, ResourceStatus};

use crate::{
    action::{
        dependency_graph::ActionDependencyGraph,
        prototype::{ActionKind, ActionPrototype, ActionPrototypeError},
    },
    attribute::value::{AttributeValueError, DependentValueGraph},
    component::resource::ResourceData,
    func::FuncExecutionPk,
    id, implement_add_edge_to,
    job::definition::ActionJob,
    workspace_snapshot::node_weight::{
        category_node_weight::CategoryNodeKind, ActionNodeWeight, NodeWeight, NodeWeightError,
    },
    AttributeValue, ChangeSetError, ChangeSetId, Component, ComponentError, ComponentId,
    DalContext, EdgeWeightError, EdgeWeightKind, EdgeWeightKindDiscriminants, HelperError,
    TransactionsError, WorkspaceSnapshotError, WsEvent, WsEventError, WsEventResult, WsPayload,
};

pub mod dependency_graph;
pub mod prototype;

#[remain::sorted]
#[derive(Debug, Error)]
pub enum ActionError {
    #[error("action prototype error: {0}")]
    ActionPrototype(#[from] ActionPrototypeError),
    #[error("AttributeValue error: {0}")]
    AttributeValue(#[from] AttributeValueError),
    #[error("Change Set error: {0}")]
    ChangeSet(#[from] ChangeSetError),
    #[error("Component error: {0}")]
    Component(#[from] ComponentError),
    #[error("component not found for action: {0}")]
    ComponentNotFoundForAction(ActionId),
    #[error("Edge Weight error: {0}")]
    EdgeWeight(#[from] EdgeWeightError),
    #[error("Helper error: {0}")]
    Helper(#[from] HelperError),
    #[error("Layer DB error: {0}")]
    LayerDb(#[from] LayerDbError),
    #[error("Node Weight error: {0}")]
    NodeWeight(#[from] NodeWeightError),
    #[error("prototype not found for action: {0}")]
    PrototypeNotFoundForAction(ActionId),
    #[error("Transactions error: {0}")]
    Transactions(#[from] TransactionsError),
    #[error("Unable to determine kind for action: {0}")]
    UnableToGetKind(ActionId),
    #[error("Workspace Snapshot error: {0}")]
    WorkspaceSnapshot(#[from] WorkspaceSnapshotError),
    #[error("ws event error: {0}")]
    WsEvent(#[from] WsEventError),
}

pub type ActionResult<T> = Result<T, ActionError>;

id!(ActionId);

impl From<ActionId> for si_events::ActionId {
    fn from(value: ActionId) -> Self {
        value.into_inner().into()
    }
}

id!(ActionPrototypeId);

impl From<ActionPrototypeId> for si_events::ActionPrototypeId {
    fn from(value: ActionPrototypeId) -> Self {
        value.into_inner().into()
    }
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize, EnumDiscriminants, PartialEq, Eq)]
#[strum_discriminants(derive(strum::Display, Serialize, Deserialize))]
pub enum ActionState {
    /// Action has been determined to be eligible to run, and has had its job sent to the job
    /// queue.
    Dispatched,
    /// Action failed during execution. See the job history for details.
    Failed,
    /// Action is "queued", but should not be considered as eligible to run, until moved to the
    /// `Queued` state.
    OnHold,
    /// Action is available to be dispatched once all of its prerequisites have succeeded, and been
    /// removed from the graph.
    Queued,
    /// Action has been dispatched, and started execution in the job system. See the job history
    /// for details.
    Running,
}

/// The completion status of a [`ActionRunner`]
///
/// NOTE: This type is only here for backwards comppatibility
/// TODO(fnichol): delete this when it's time
#[remain::sorted]
#[derive(
    Deserialize,
    Serialize,
    AsRefStr,
    Display,
    EnumIter,
    EnumString,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    ToSql,
    FromSql,
)]
#[serde(rename_all = "camelCase")]
#[strum(serialize_all = "camelCase")]
pub enum ActionCompletionStatus {
    Error,
    Failure,
    Success,
    Unstarted,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Action {
    id: ActionId,
    state: ActionState,
    originating_changeset_id: ChangeSetId,
    // DEPRECATED
    func_execution_pk: Option<FuncExecutionPk>,
}

impl From<ActionNodeWeight> for Action {
    fn from(value: ActionNodeWeight) -> Self {
        Self {
            id: value.id().into(),
            state: value.state(),
            originating_changeset_id: value.originating_changeset_id(),
            func_execution_pk: None,
        }
    }
}

impl Action {
    pub fn id(&self) -> ActionId {
        self.id
    }

    pub fn state(&self) -> ActionState {
        self.state
    }

    pub fn originating_changeset_id(&self) -> ChangeSetId {
        self.originating_changeset_id
    }

    implement_add_edge_to!(
        source_id: ActionId,
        destination_id: ComponentId,
        add_fn: add_edge_to_component,
        discriminant: EdgeWeightKindDiscriminants::Use,
        result: ActionResult,
    );
    implement_add_edge_to!(
        source_id: ActionId,
        destination_id: ActionPrototypeId,
        add_fn: add_edge_to_action_prototype,
        discriminant: EdgeWeightKindDiscriminants::Use,
        result: ActionResult,
    );

    // Even though we're using `implement_add_edge_to`, we're not creating an edge from Self *TO*
    // the Category node. We're adding an edge *FROM* the Category node to Self.
    implement_add_edge_to!(
        source_id: Ulid,
        destination_id: ActionId,
        add_fn: add_incoming_category_edge,
        discriminant: EdgeWeightKindDiscriminants::Use,
        result: ActionResult,
    );

    pub async fn find_for_component_id(
        ctx: &DalContext,
        component_id: ComponentId,
    ) -> ActionResult<Vec<ActionId>> {
        let mut actions = vec![];
        let snap = ctx.workspace_snapshot()?;
        let action_category_id = snap
            .get_category_node_or_err(None, CategoryNodeKind::Action)
            .await?;

        for action_idx in snap
            .outgoing_targets_for_edge_weight_kind(
                action_category_id,
                EdgeWeightKindDiscriminants::Use,
            )
            .await?
        {
            let action_id: ActionId = snap
                .get_node_weight(action_idx)
                .await?
                .get_action_node_weight()?
                .id()
                .into();

            if Self::component_id(ctx, action_id).await? == Some(component_id) {
                actions.push(action_id);
            }
        }
        Ok(actions)
    }

    #[instrument(level = "info", skip_all)]
    pub async fn remove_all_for_component_id(
        ctx: &DalContext,
        component_id: ComponentId,
    ) -> ActionResult<()> {
        let actions_to_remove = Self::find_for_component_id(ctx, component_id).await?;
        for action_id in actions_to_remove {
            Self::remove_by_id(ctx, action_id).await?;
        }
        Ok(())
    }

    pub async fn find_for_states_and_component_id(
        ctx: &DalContext,
        component_id: ComponentId,
        action_states: Vec<ActionState>,
    ) -> ActionResult<Vec<ActionId>> {
        let mut actions = vec![];
        let actions_for_component = Self::find_for_component_id(ctx, component_id).await?;
        for action_id in actions_for_component {
            let action = Self::get_by_id(ctx, action_id).await?;
            if action_states.contains(&action.state()) {
                actions.push(action_id);
            }
        }
        Ok(actions)
    }

    pub async fn find_for_kind_and_component_id(
        ctx: &DalContext,
        component_id: ComponentId,
        action_kind: ActionKind,
    ) -> ActionResult<Vec<ActionId>> {
        let actions_for_component = Self::find_for_component_id(ctx, component_id).await?;
        let mut actions = vec![];
        for action_id in actions_for_component {
            let action_prototype_id = Self::prototype_id(ctx, action_id).await?;
            let action_prototype = ActionPrototype::get_by_id(ctx, action_prototype_id).await?;
            if action_prototype.kind == action_kind {
                actions.push(action_id);
            }
        }
        Ok(actions)
    }

    pub async fn find_equivalent(
        ctx: &DalContext,
        action_prototype_id: ActionPrototypeId,
        maybe_component_id: Option<ComponentId>,
    ) -> ActionResult<Option<ActionId>> {
        let snap = ctx.workspace_snapshot()?;
        let action_category_id = snap
            .get_category_node_or_err(None, CategoryNodeKind::Action)
            .await?;

        for action_idx in snap
            .outgoing_targets_for_edge_weight_kind(
                action_category_id,
                EdgeWeightKindDiscriminants::Use,
            )
            .await?
        {
            let action_id: ActionId = snap
                .get_node_weight(action_idx)
                .await?
                .get_action_node_weight()?
                .id()
                .into();

            if Self::component_id(ctx, action_id).await? == maybe_component_id
                && Self::prototype_id(ctx, action_id).await? == action_prototype_id
            {
                // we found the equivalent!
                return Ok(Some(action_id));
            }
        }
        Ok(None)
    }

    pub async fn get_by_id(ctx: &DalContext, id: ActionId) -> ActionResult<Self> {
        let action: Self = ctx
            .workspace_snapshot()?
            .get_node_weight_by_id(id)
            .await?
            .get_action_node_weight()?
            .into();
        Ok(action)
    }

    pub async fn set_state(ctx: &DalContext, id: ActionId, state: ActionState) -> ActionResult<()> {
        let idx = ctx.workspace_snapshot()?.get_node_index_by_id(id).await?;
        let node_weight = ctx
            .workspace_snapshot()?
            .get_node_weight(idx)
            .await?
            .get_action_node_weight()?;
        let mut new_node_weight =
            node_weight.new_with_incremented_vector_clock(ctx.change_set()?)?;
        new_node_weight.set_state(state);
        ctx.workspace_snapshot()?
            .add_node(NodeWeight::Action(new_node_weight))
            .await?;
        ctx.workspace_snapshot()?.replace_references(idx).await?;
        Ok(())
    }

    #[deprecated(note = "no longer tracking this")]
    pub async fn set_func_execution_pk(
        _ctx: &DalContext,
        _id: ActionId,
        _pk: Option<FuncExecutionPk>,
    ) -> ActionResult<()> {
        unimplemented!("You should never be setting func_execution_pk; bug!");
    }

    #[instrument(level = "info", skip(ctx))]
    pub async fn new(
        ctx: &DalContext,
        action_prototype_id: ActionPrototypeId,
        maybe_component_id: Option<ComponentId>,
    ) -> ActionResult<Self> {
        let change_set = ctx.change_set()?;
        let new_id: ActionId = change_set.generate_ulid()?.into();
        let originating_change_set_id = ctx.change_set_id();
        let node_weight =
            NodeWeight::new_action(change_set, originating_change_set_id, new_id.into())?;
        ctx.workspace_snapshot()?.add_node(node_weight).await?;

        let action_category_id = ctx
            .workspace_snapshot()?
            .get_category_node_or_err(None, CategoryNodeKind::Action)
            .await?;
        Self::add_incoming_category_edge(
            ctx,
            action_category_id,
            new_id,
            EdgeWeightKind::new_use(),
        )
        .await?;

        Self::add_edge_to_action_prototype(
            ctx,
            new_id,
            action_prototype_id,
            EdgeWeightKind::new_use(),
        )
        .await?;

        if let Some(component_id) = maybe_component_id {
            Self::add_edge_to_component(ctx, new_id, component_id, EdgeWeightKind::new_use())
                .await?;
        }

        let new_action: Self = ctx
            .workspace_snapshot()?
            .get_node_weight_by_id(new_id)
            .await?
            .get_action_node_weight()?
            .into();
        WsEvent::action_list_updated(ctx)
            .await?
            .publish_on_commit(ctx)
            .await?;
        Ok(new_action)
    }

    pub async fn remove_by_id(ctx: &DalContext, action_id: ActionId) -> ActionResult<()> {
        ctx.workspace_snapshot()?
            .remove_node_by_id(ctx.change_set()?, action_id)
            .await?;
        Ok(())
    }

    pub async fn remove_by_prototype_and_component(
        ctx: &DalContext,
        action_prototype_id: ActionPrototypeId,
        maybe_component_id: Option<ComponentId>,
    ) -> ActionResult<()> {
        let change_set = ctx.change_set()?;
        let snap = ctx.workspace_snapshot()?;

        if let Some(action_id) =
            Self::find_equivalent(ctx, action_prototype_id, maybe_component_id).await?
        {
            snap.remove_node_by_id(change_set, action_id).await?;
        }
        Ok(())
    }

    /// Sort the dependency graph of [`Actions`][Action] topologically, breaking ties by listing
    /// [`Actions`][Action] sorted by their ID (oldest first thanks to ULID sorting).
    #[instrument(level = "info", skip_all)]
    pub async fn list_topologically(ctx: &DalContext) -> ActionResult<Vec<ActionId>> {
        // TODO: Grab all "running" & "failed" Actions to list first?
        let mut result = Vec::new();

        let mut action_dependency_graph = ActionDependencyGraph::for_workspace(ctx).await?;

        loop {
            let mut independent_actions = action_dependency_graph.independent_actions();
            if independent_actions.is_empty() {
                break;
            }

            independent_actions.sort();
            for action_id in independent_actions {
                action_dependency_graph.remove_action(action_id);
                result.push(action_id);
            }
        }

        // If there is a cycle in the dependencies for some reason, we still want to know that the
        // actions exist, even though they'll never start executing.
        let mut actions_in_cycle = action_dependency_graph.remaining_actions();
        actions_in_cycle.sort();
        result.extend(&actions_in_cycle);

        Ok(result)
    }

    pub async fn prototype_id(
        ctx: &DalContext,
        action_id: ActionId,
    ) -> ActionResult<ActionPrototypeId> {
        for (_, _tail_node_idx, head_node_idx) in ctx
            .workspace_snapshot()?
            .edges_directed_for_edge_weight_kind(
                action_id,
                Outgoing,
                EdgeWeightKindDiscriminants::Use,
            )
            .await?
        {
            if let NodeWeight::ActionPrototype(node_weight) = ctx
                .workspace_snapshot()?
                .get_node_weight(head_node_idx)
                .await?
            {
                return Ok(node_weight.id().into());
            }
        }

        Err(ActionError::PrototypeNotFoundForAction(action_id))
    }

    pub async fn component_id(
        ctx: &DalContext,
        action_id: ActionId,
    ) -> ActionResult<Option<ComponentId>> {
        for (_, _tail_node_idx, head_node_idx) in ctx
            .workspace_snapshot()?
            .edges_directed_for_edge_weight_kind(
                action_id,
                Outgoing,
                EdgeWeightKindDiscriminants::Use,
            )
            .await?
        {
            if let NodeWeight::Component(component_node_weight) = ctx
                .workspace_snapshot()?
                .get_node_weight(head_node_idx)
                .await?
            {
                return Ok(Some(component_node_weight.id().into()));
            }
        }

        Ok(None)
    }

    pub async fn all_ids(ctx: &DalContext) -> ActionResult<Vec<ActionId>> {
        let mut result = Vec::new();

        let action_category_node_index = ctx
            .workspace_snapshot()?
            .get_category_node_or_err(None, CategoryNodeKind::Action)
            .await?;
        for (_, _, action_node_index) in ctx
            .workspace_snapshot()?
            .edges_directed(action_category_node_index, Outgoing)
            .await?
        {
            let action_id = ctx
                .workspace_snapshot()?
                .get_node_weight(action_node_index)
                .await?
                .id();
            result.push(action_id.into());
        }

        Ok(result)
    }

    /// Gets all actions this action is dependent on
    pub async fn get_dependent_actions_by_id(
        ctx: &DalContext,
        action_id: ActionId,
    ) -> ActionResult<Vec<ActionId>> {
        let action_dependency_graph: ActionDependencyGraph =
            ActionDependencyGraph::for_workspace(ctx).await?;
        Ok(action_dependency_graph.direct_dependencies_of(action_id))
    }

    // Gets all actions that are dependent on this action (the entire subgraph)
    #[instrument(level = "info", skip(ctx))]
    pub async fn get_all_dependencies(&self, ctx: &DalContext) -> ActionResult<Vec<ActionId>> {
        let action_dependency_graph: ActionDependencyGraph =
            ActionDependencyGraph::for_workspace(ctx).await?;
        Ok(action_dependency_graph.get_all_dependencies(self.id()))
    }

    #[instrument(level = "info", skip(ctx))]
    pub async fn get_hold_status_influenced_by(
        &self,
        ctx: &DalContext,
    ) -> ActionResult<Vec<ActionId>> {
        let mut reasons_for_hold = vec![];
        let mut work_queue =
            VecDeque::from(Self::get_dependent_actions_by_id(ctx, self.id()).await?);
        while let Some(action_id) = work_queue.pop_front() {
            let act = Self::get_by_id(ctx, action_id).await?;
            match act.state() {
                ActionState::Failed | ActionState::OnHold => reasons_for_hold.push(act.id()),
                _ => (),
            }
            work_queue.extend(Self::get_dependent_actions_by_id(ctx, action_id).await?);
        }
        Ok(reasons_for_hold)
    }

    pub async fn run(
        ctx: &DalContext,
        id: ActionId,
    ) -> ActionResult<Option<ActionRunResultSuccess>> {
        let component_id = Action::component_id(ctx, id)
            .await?
            .ok_or(ActionError::ComponentNotFoundForAction(id))?;

        let prototype_id = Action::prototype_id(ctx, id).await?;

        let (func_run_value, resource) =
            ActionPrototype::run(ctx, prototype_id, component_id).await?;

        if matches!(
            resource.as_ref().map(|r| r.status),
            Some(ResourceStatus::Ok)
        ) {
            ctx.workspace_snapshot()?
                .remove_node_by_id(ctx.change_set()?, id)
                .await?;

            ctx.layer_db()
                .func_run()
                .set_action_result_state(
                    func_run_value.func_run_id(),
                    ActionResultState::Success,
                    ctx.events_tenancy(),
                    ctx.events_actor(),
                )
                .await?;

            let component = Component::get_by_id(ctx, component_id).await?;

            if component.to_delete()
                && matches!(resource.as_ref().map(|r| r.payload.is_none()), Some(true))
            {
                Component::remove(ctx, component.id()).await?;
            }
        } else {
            Action::set_state(ctx, id, ActionState::Failed).await?;
            ctx.layer_db()
                .func_run()
                .set_action_result_state(
                    func_run_value.func_run_id(),
                    ActionResultState::Failure,
                    ctx.events_tenancy(),
                    ctx.events_actor(),
                )
                .await?;
        }
        WsEvent::action_list_updated(ctx)
            .await?
            .publish_on_commit(ctx)
            .await?;

        Ok(resource)
    }

    /// An Action is dispatchable if all of the following are true:
    ///   * The action is in the state [`ActionState::Queued`](ActionState)
    ///   * The graph of values for `DependentValuesUpdate` does *NOT* include
    ///     *ANY* [`AttributeValue`s](AttributeValue) for the same
    ///     [`Component`](crate::Component) as the [`Action`].
    ///
    /// This method **DOES NOT** check the `DependentValuesUpdate` graph. That
    /// is done as part of [`Self::eligible_to_dispatch()`]
    pub fn is_eligible_to_dispatch(&self) -> bool {
        // Only Actions in the ActionState::Queued state are dispatchable.
        self.state() == ActionState::Queued
    }

    /// An Action is dispatchable if all of the following are true:
    ///   * The action is in the state [`ActionState::Queued`](ActionState)
    ///   * The graph of values for `DependentValuesUpdate` does *NOT* include
    ///     *ANY* [`AttributeValue`s](AttributeValue) for the same
    ///     [`Component`](crate::Component) as the [`Action`].
    pub async fn eligible_to_dispatch(ctx: &DalContext) -> ActionResult<Vec<ActionId>> {
        let action_dependency_graph = ActionDependencyGraph::for_workspace(ctx).await?;
        let mut result = Vec::new();
        let dependent_value_graph = DependentValueGraph::new(
            ctx,
            ctx.workspace_snapshot()?
                .list_dependent_value_value_ids()
                .await?,
        )
        .await?;

        // Find the ComponentIds for all AttributeValues in the full dependency graph for the
        // queued/running DependentValuesUpdate. We'll want to hold off on dispatching any Actions
        // that would be operating on the same Component until after the DependentValuesUpdate has
        // finished working with that Component.
        let mut dvu_component_ids: HashSet<ComponentId> = HashSet::new();
        for av_id in &dependent_value_graph.all_value_ids() {
            dvu_component_ids.insert(AttributeValue::component_id(ctx, *av_id).await?);
        }

        for possible_action_id in action_dependency_graph.independent_actions() {
            let action = Action::get_by_id(ctx, possible_action_id).await?;

            if action.is_eligible_to_dispatch() {
                if let Some(action_component_id) = Action::component_id(ctx, action.id()).await? {
                    if dvu_component_ids.contains(&action_component_id) {
                        // This action is for a Component that currently involved in the queued
                        // DependentValuesUpdate graph. We don't want to dispatch the Action until
                        // the DependentValuesUpdate job has completely finished
                        // processing/populating values that the Action might need to work with.
                        continue;
                    }
                }
                result.push(possible_action_id);
            }
        }

        Ok(result)
    }

    pub async fn dispatch_action(ctx: &DalContext, action_id: ActionId) -> ActionResult<()> {
        Action::set_state(ctx, action_id, ActionState::Dispatched).await?;

        ctx.enqueue_action(ActionJob::new(ctx, action_id)).await?;

        Ok(())
    }
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActionReturn {
    id: ActionId,
    component_id: ComponentId,
    kind: ActionKind,
    resource: Option<ResourceData>,
}

impl WsEvent {
    pub async fn action_return(
        ctx: &DalContext,
        id: ActionId,
        kind: ActionKind,
        component_id: ComponentId,
        resource: Option<ResourceData>,
    ) -> WsEventResult<Self> {
        WsEvent::new(
            ctx,
            WsPayload::ActionReturn(ActionReturn {
                id,
                kind,
                component_id,
                resource,
            }),
        )
        .await
    }
    pub async fn action_list_updated(ctx: &DalContext) -> WsEventResult<Self> {
        WsEvent::new(ctx, WsPayload::ActionsListUpdated(ctx.change_set_id())).await
    }
}
