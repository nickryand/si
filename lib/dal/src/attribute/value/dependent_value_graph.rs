use petgraph::prelude::*;
use si_events::ulid::Ulid;
use std::collections::HashSet;
use std::collections::{hash_map::Entry, HashMap, VecDeque};
use std::{fs::File, io::Write};
use telemetry::prelude::*;

use crate::component::ControllingFuncData;
use crate::workspace_snapshot::node_weight::NodeWeightDiscriminants;
use crate::{
    attribute::{
        prototype::{argument::AttributePrototypeArgument, AttributePrototype},
        value::ValueIsFor,
    },
    dependency_graph::DependencyGraph,
    workspace_snapshot::edge_weight::EdgeWeightKindDiscriminants,
    Component, DalContext, Secret,
};
use crate::{ComponentError, ComponentId, Prop, PropKind};

use super::{AttributeValue, AttributeValueError, AttributeValueId, AttributeValueResult};

#[derive(Debug, Clone)]
pub struct DependentValueGraph {
    inner: DependencyGraph<AttributeValueId>,
    values_that_need_to_execute_from_prototype_function: HashSet<AttributeValueId>,
}

// We specifically need to track if the value is one of the child values we
// added to the graph in order to discover if a dynamically set object's
// children are inputs to a function. The other two parts of this enum are not
// used now, but may be useful information when debugging
enum WorkQueueValue {
    Initial(AttributeValueId),
    ObjectChild(AttributeValueId),
    Discovered(AttributeValueId),
}

impl WorkQueueValue {
    fn id(&self) -> AttributeValueId {
        match self {
            WorkQueueValue::Initial(id)
            | WorkQueueValue::ObjectChild(id)
            | WorkQueueValue::Discovered(id) => *id,
        }
    }
}

impl DependentValueGraph {
    /// Construct a [`DependentValueGraph`] of all the [`AttributeValueIds`](AttributeValue) who are
    /// dependent on the initial ids provided as well as all descending dependencies.
    pub async fn new(
        ctx: &DalContext,
        initial_ids: Vec<impl Into<Ulid>>,
    ) -> AttributeValueResult<Self> {
        let mut dependent_value_graph = Self {
            inner: DependencyGraph::new(),
            values_that_need_to_execute_from_prototype_function: HashSet::new(),
        };

        let values = dependent_value_graph
            .parse_initial_ids(ctx, initial_ids)
            .await?;
        dependent_value_graph
            .populate_for_values(ctx, values)
            .await?;

        Ok(dependent_value_graph)
    }

    /// Parse the set of initial ids in order to construct the list of [`values`](WorkQueueValue).
    async fn parse_initial_ids(
        &mut self,
        ctx: &DalContext,
        initial_ids: Vec<impl Into<Ulid>>,
    ) -> AttributeValueResult<Vec<WorkQueueValue>> {
        let workspace_snapshot = ctx.workspace_snapshot()?;

        let mut values = Vec::new();
        for initial_id in initial_ids {
            let initial_id = initial_id.into();

            // It's possible that one or more of the initial ids provided by the enqueued
            // DependentValuesUpdate job may have been removed from the snapshot between when the
            // DVU job was created and when we're processing things now. This could happen if there
            // are other modifications to the snapshot before the DVU job starts executing, as the
            // job always operates on the current state of the change set's snapshot, not the state
            // at the time the job was created.
            if workspace_snapshot
                .try_get_node_index_by_id(initial_id)
                .await?
                .is_none()
            {
                debug!(%initial_id, "missing node, skipping it in DependentValueGraph");
                continue;
            }

            let node_weight = workspace_snapshot.get_node_weight_by_id(initial_id).await?;

            match node_weight.into() {
                NodeWeightDiscriminants::AttributeValue => {
                    let initial_attribute_value_id: AttributeValueId = initial_id.into();
                    if AttributeValue::is_set_by_dependent_function(ctx, initial_attribute_value_id)
                        .await?
                    {
                        self.values_that_need_to_execute_from_prototype_function
                            .insert(initial_attribute_value_id);
                    }

                    values.push(WorkQueueValue::Initial(initial_attribute_value_id));
                }
                NodeWeightDiscriminants::Secret => {
                    // If we are processing a secret, we don't want to add the secret itself. We
                    // only want to add the direct dependent attribute values. We also need to mark
                    // these values as needing to be processed because DVU will skip processing the
                    // first set of independent values. In most cases, we want that to happen.
                    // However, since the first set of independent values all have a secret as their
                    // parent and not an attribute value, they need to be processed in DVU.
                    let direct_dependents =
                        Secret::direct_dependent_attribute_values(ctx, initial_id.into())
                            .await
                            .map_err(Box::new)?;
                    self.values_that_need_to_execute_from_prototype_function
                        .extend(direct_dependents.clone());
                    values.extend(
                        direct_dependents
                            .iter()
                            .map(|d| WorkQueueValue::Initial(*d)),
                    );
                }
                discrim => {
                    warn!(%discrim, %initial_id, "skipping dependent value graph generation for unsupported node weight");
                }
            };
        }

        Ok(values)
    }

    /// Populate the [`DependentValueGraph`] using the provided [`values`](WorkQueueValue). This
    /// includes the entire parent tree of each value discovered, up to the root for every value's
    /// component, as well as any dependencies of values discovered while walking the graph
    /// (e.g. if a value's prototype takes one of the passed values as an input, we also need to
    /// find the values for the other inputs to the prototype, etc.). The graph also includes any
    /// inferred dependencies based on parentage, for example if a component gets its inputs from a
    /// parent frame, and that frame's output sockets change, we add those downstream input sockets
    /// to the graph.
    async fn populate_for_values(
        &mut self,
        ctx: &DalContext,
        values: Vec<WorkQueueValue>,
    ) -> AttributeValueResult<()> {
        let workspace_snapshot = ctx.workspace_snapshot()?;

        let mut controlling_funcs_for_component = HashMap::new();
        let mut work_queue = VecDeque::from_iter(values);
        let mut seen_list = HashSet::new();

        while let Some(current_attribute_value) = work_queue.pop_front() {
            let mut found_deps = false;

            if seen_list.contains(&current_attribute_value.id()) {
                continue;
            }
            seen_list.insert(current_attribute_value.id());

            let current_component_id =
                AttributeValue::component_id(ctx, current_attribute_value.id()).await?;

            // We need to be sure to only construct the graph out of
            // "controlling" values. However, controlled values can still be
            // inputs to functions, so we need to find the prototypes that
            // depend on them!
            let current_attribute_value_controlling_value_id =
                Self::get_controlling_attribute_value_id(
                    ctx,
                    current_component_id,
                    current_attribute_value.id(),
                    &mut controlling_funcs_for_component,
                )
                .await?;

            let value_is_for = AttributeValue::is_for(ctx, current_attribute_value.id()).await?;

            // Gather the Attribute Prototype Arguments that take the thing the
            // current value is for (prop, or socket) as an input
            let relevant_apas = {
                let attribute_prototype_argument_idxs = workspace_snapshot
                    .incoming_sources_for_edge_weight_kind(
                        value_is_for,
                        EdgeWeightKindDiscriminants::PrototypeArgumentValue,
                    )
                    .await?;

                let mut relevant_apas = vec![];
                for apa_idx in attribute_prototype_argument_idxs {
                    let apa = workspace_snapshot
                        .get_node_weight(apa_idx)
                        .await?
                        .get_attribute_prototype_argument_node_weight()?;

                    match apa.targets() {
                        // If there are no targets, this is a schema-level attribute prototype argument
                        None => relevant_apas.push(apa),
                        Some(targets) => {
                            if targets.source_component_id == current_component_id {
                                // Both "deleted" and not deleted Components can feed data into
                                // "deleted" Components. **ONLY** not deleted Components can feed
                                // data into not deleted Components.
                                if Component::should_data_flow_between_components(
                                    ctx,
                                    targets.destination_component_id,
                                    targets.source_component_id,
                                )
                                .await
                                .map_err(|e| AttributeValueError::Component(Box::new(e)))?
                                {
                                    relevant_apas.push(apa)
                                }
                            }
                        }
                    }
                }
                relevant_apas
            };

            match value_is_for {
                ValueIsFor::Prop(prop_id) => {
                    let prop = Prop::get_by_id_or_error(ctx, prop_id).await?;
                    if prop.kind == PropKind::Object {
                        // The children of an object might themselves be the
                        // input to another function, so we have to add them to
                        // the calculation of the graph, as we encounter them.
                        // We use `seen_list` to ensure we don't reprocess these
                        // values or the parents of these values.
                        for child_value_id in AttributeValue::get_child_av_ids_for_ordered_parent(
                            ctx,
                            current_attribute_value.id(),
                        )
                        .await?
                        {
                            if !seen_list.contains(&child_value_id) {
                                work_queue.push_back(WorkQueueValue::ObjectChild(child_value_id));
                            }
                        }
                    }
                }
                // Check if this value is an output socket as the attribute
                // value might have implicit dependendcies based on the ancestry
                // (aka frames/nested frames) note: we filter out non-deleted
                // targets if the source component is set to be deleted
                ValueIsFor::OutputSocket(_) => {
                    let maybe_values_depend_on =
                        match Component::find_inferred_values_using_this_output_socket(
                            ctx,
                            current_attribute_value.id(),
                        )
                        .await
                        {
                            Ok(values) => values,
                            // When we first run dvu, the component type might not be set yet.
                            // In this case, we can assume there aren't downstream inputs that need to
                            // be queued up.
                            Err(ComponentError::ComponentMissingTypeValueMaterializedView(_)) => {
                                vec![]
                            }
                            Err(err) => return Err(AttributeValueError::Component(Box::new(err))),
                        };

                    for input_socket_match in maybe_values_depend_on {
                        // Both "deleted" and not deleted Components can feed data into
                        // "deleted" Components. **ONLY** not deleted Components can feed
                        // data into not deleted Components.
                        let destination_component_id =
                            AttributeValue::component_id(ctx, current_attribute_value.id()).await?;
                        if Component::should_data_flow_between_components(
                            ctx,
                            destination_component_id,
                            input_socket_match.component_id,
                        )
                        .await
                        .map_err(|e| AttributeValueError::Component(Box::new(e)))?
                        {
                            work_queue.push_back(WorkQueueValue::Discovered(
                                input_socket_match.attribute_value_id,
                            ));
                            self.value_depends_on(
                                input_socket_match.attribute_value_id,
                                current_attribute_value.id(),
                            );

                            found_deps = true;
                        }
                    }
                }
                _ => {}
            }

            // Find the values that are set by the prototype for the relevant
            // AttributePrototypeArguments, and declare that these values depend
            // on the value of the current value
            for apa in relevant_apas {
                let prototype_id =
                    AttributePrototypeArgument::prototype_id_for_argument_id(ctx, apa.id().into())
                        .await?;

                let attribute_value_ids =
                    AttributePrototype::attribute_value_ids(ctx, prototype_id).await?;

                for attribute_value_id in attribute_value_ids {
                    let filter_component_id = match apa.targets() {
                        None => current_component_id,
                        Some(targets) => targets.destination_component_id,
                    };
                    let component_id =
                        AttributeValue::component_id(ctx, attribute_value_id).await?;

                    if component_id != filter_component_id {
                        continue;
                    }

                    // If the input to this function is a value that is a child of another dynamic
                    // function, we should just depend on the controlling value in the dependency
                    // graph, since we can't guarantee that the controlled value won't be destroyed
                    // when "populating" nested value
                    let controlling_attribute_value_id = Self::get_controlling_attribute_value_id(
                        ctx,
                        component_id,
                        attribute_value_id,
                        &mut controlling_funcs_for_component,
                    )
                    .await?;

                    work_queue
                        .push_back(WorkQueueValue::Discovered(controlling_attribute_value_id));
                    self.inner.id_depends_on(
                        controlling_attribute_value_id,
                        current_attribute_value_controlling_value_id,
                    );

                    found_deps = true;
                }
            }

            // Parent props always depend on their children, even if those parents do not have
            // "dependent" functions. Adding them to the graph ensures we execute the entire set of
            // dependent values here (suppose for example an output socket depends on the root
            // prop, we have to be sure we add that output socket to the graph if a child of root
            // changes, because if a child of root has changed, then the view of root to the leaves
            // will also change)
            if let Some(parent_attribute_value_id) = AttributeValue::parent_attribute_value_id(
                ctx,
                current_attribute_value_controlling_value_id,
            )
            .await?
            {
                // If this is one of child values we added speculatively we
                // should only walk the parent tree if we have actually found a
                // dep for this value.  Otherwise we will add unnecessary values
                // to the graph. Normally this is harmless, but it means we'll
                // do more work than necessary
                if !found_deps && matches!(current_attribute_value, WorkQueueValue::ObjectChild(_))
                {
                    continue;
                }

                work_queue.push_back(WorkQueueValue::Discovered(parent_attribute_value_id));
                self.inner.id_depends_on(
                    parent_attribute_value_id,
                    current_attribute_value_controlling_value_id,
                );
            }
        }

        Ok(())
    }

    pub async fn debug_dot(&self, ctx: &DalContext, suffix: Option<&str>) {
        let mut is_for_map = HashMap::new();

        for attribute_value_id in self.inner.id_to_index_map().keys() {
            let is_for = AttributeValue::is_for(ctx, *attribute_value_id)
                .await
                .expect("able to get value is for")
                .debug_info(ctx)
                .await
                .expect("able to get info for value is for");
            is_for_map.insert(*attribute_value_id, is_for);
        }

        let label_value_fn =
            move |_: &StableDiGraph<AttributeValueId, ()>,
                  (_, attribute_value_id): (NodeIndex, &AttributeValueId)| {
                let attribute_value_id = *attribute_value_id;
                let is_for = is_for_map.clone();

                let is_for_string = is_for
                    .clone()
                    .get(&attribute_value_id)
                    .map(ToOwned::to_owned)
                    .expect("is for exists for every value");

                format!("label = \"{}\n{}\"", attribute_value_id, is_for_string)
            };

        let dot = petgraph::dot::Dot::with_attr_getters(
            self.inner.graph(),
            &[
                petgraph::dot::Config::NodeNoLabel,
                petgraph::dot::Config::EdgeNoLabel,
            ],
            &|_, _| "label = \"\"".to_string(),
            &label_value_fn,
        );

        let filename_no_extension = format!("{}-{}", Ulid::new(), suffix.unwrap_or("depgraph"));
        let mut file = File::create(format!("/home/zacharyhamm/{filename_no_extension}.txt"))
            .expect("could not create file");

        file.write_all(format!("{dot:?}").as_bytes())
            .expect("could not write file");
        println!("dot output stored in file (filename without extension: {filename_no_extension})");
    }

    async fn get_controlling_attribute_value_id(
        ctx: &DalContext,
        current_component_id: ComponentId,
        current_attribute_value_id: AttributeValueId,
        controlling_funcs_for_component: &mut HashMap<
            ComponentId,
            HashMap<AttributeValueId, ControllingFuncData>,
        >,
    ) -> AttributeValueResult<AttributeValueId> {
        Ok(
            match controlling_funcs_for_component.entry(current_component_id) {
                Entry::Vacant(entry) => {
                    let controlling_func_data =
                        Component::list_av_controlling_func_ids_for_id(ctx, current_component_id)
                            .await
                            .map_err(Box::new)?;
                    let data = controlling_func_data
                        .get(&current_attribute_value_id)
                        .copied();

                    entry.insert(controlling_func_data);

                    data
                }
                Entry::Occupied(entry) => entry.get().get(&current_attribute_value_id).copied(),
            }
            .map(|func_data| func_data.av_id)
            // If nothing controls us, we control ourselves
            .unwrap_or(current_attribute_value_id),
        )
    }

    pub fn value_depends_on(
        &mut self,
        value_id: AttributeValueId,
        depends_on_id: AttributeValueId,
    ) {
        self.inner.id_depends_on(value_id, depends_on_id);
    }

    pub fn contains_value(&self, value_id: AttributeValueId) -> bool {
        self.inner.contains_id(value_id)
    }

    pub fn direct_dependencies_of(&self, value_id: AttributeValueId) -> Vec<AttributeValueId> {
        self.inner.direct_dependencies_of(value_id)
    }

    pub fn remove_value(&mut self, value_id: AttributeValueId) {
        self.inner.remove_id(value_id);
    }

    pub fn cycle_on_self(&mut self, value_id: AttributeValueId) {
        self.inner.cycle_on_self(value_id);
    }

    pub fn independent_values(&self) -> Vec<AttributeValueId> {
        self.inner.independent_ids()
    }

    pub fn all_value_ids(&self) -> Vec<AttributeValueId> {
        self.inner.all_ids()
    }

    /// Indicates whether the value needs to be processed. This is useful for determining when to
    /// filter or de-duplicate values when executing from their prototype functions. If the value is
    /// marked as needing to be processed, it likely needs to execute from its prototype function.
    pub fn values_needs_to_execute_from_prototype_function(
        &self,
        value_id: AttributeValueId,
    ) -> bool {
        self.values_that_need_to_execute_from_prototype_function
            .contains(&value_id)
    }
}
