//! This module provides audit logging functionality to the rest of the crate.

use audit_logs::AuditLogsError;
use audit_logs::AuditLogsStream;
use chrono::Utc;
use futures::StreamExt;
use pending_events::PendingEventsError;
use pending_events::PendingEventsStream;
use shuttle_server::Shuttle;
use shuttle_server::ShuttleError;
use si_data_nats::async_nats;
use si_data_nats::async_nats::jetstream::consumer::pull::BatchErrorKind;
use si_data_nats::async_nats::jetstream::stream::ConsumerErrorKind;
use si_events::audit_log::AuditLog;
use si_events::audit_log::AuditLogKind;
use si_frontend_types::AuditLog as FrontendAuditLog;
use telemetry::prelude::*;
use thiserror::Error;
use tokio_util::task::TaskTracker;

use crate::DalContext;
use crate::TenancyError;
use crate::TransactionsError;

#[remain::sorted]
#[derive(Debug, Error)]
pub enum AuditLoggingError {
    #[error("async nats batch error: {0}")]
    AsyncNatsBatch(#[from] async_nats::error::Error<BatchErrorKind>),
    #[error("async nats consumer error: {0}")]
    AsyncNatsConsumer(#[from] async_nats::error::Error<ConsumerErrorKind>),
    #[error("audit logs error: {0}")]
    AuditLogs(#[from] AuditLogsError),
    #[error("pending events error: {0}")]
    PendingEventsError(#[from] PendingEventsError),
    #[error("serde json error: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("shuttle error: {0}")]
    Shuttle(#[from] ShuttleError),
    #[error("transactions error: {0}")]
    Transactions(#[from] Box<TransactionsError>),
}

type Result<T> = std::result::Result<T, AuditLoggingError>;

/// Publishes all pending [`AuditLogs`](AuditLog) to the audit logs stream for the event session.
///
/// Provide the "override" [`EventSessionId`] if you'd like to use a different identifier than
/// the one on [`self`](DalContext).
///
/// _Warning: the subject for the event session must have a [final message](write_final_message)._
#[instrument(
    name = "audit_logging.publish_pending",
    level = "debug",
    skip_all,
    fields(override_event_session_id)
)]
pub(crate) async fn publish_pending(
    ctx: &DalContext,
    tracker: Option<TaskTracker>,
    override_event_session_id: Option<si_events::EventSessionId>,
) -> Result<()> {
    // TODO(nick): nuke this from intergalactic orbit. Then do it again.
    let workspace_id = match ctx.workspace_pk() {
        Ok(workspace_id) => workspace_id,
        Err(TransactionsError::Tenancy(TenancyError::NoWorkspace)) => return Ok(()),
        Err(err) => return Err(AuditLoggingError::Transactions(Box::new(err))),
    };

    let (tracker, provided_tracker) = match tracker {
        Some(provided_tracker) => (provided_tracker, false),
        None => (TaskTracker::new(), true),
    };

    // Get a handle on the source and destination streams.
    let source_stream = PendingEventsStream::get_or_create(ctx.jetstream_context()).await?;
    let destination_stream = AuditLogsStream::get_or_create(ctx.jetstream_context()).await?;

    // Create a shuttle instance for shuttling audit logs from the pending events stream.
    let audit_logs_shuttle = Shuttle::new(
        ctx.nats_conn().to_owned(),
        tracker.to_owned(),
        source_stream.stream().await?,
        source_stream.subject_for_audit_log(
            workspace_id.into(),
            ctx.change_set_id().into(),
            match override_event_session_id {
                Some(override_id) => override_id,
                None => ctx.event_session_id(),
            },
        ),
        destination_stream.subject(workspace_id.into()),
    )
    .await?;

    // Run the audit logs shuttle instance. If a tracker has been provided, we can spawn the
    // shuttle instance using it. If we are using a tracker purely within this function, we cannot
    // reliably use it to run the shuttle instance, so we will close and wait once shuttle exits.
    if provided_tracker {
        tracker.spawn(async move {
            if let Err(err) = audit_logs_shuttle.try_run().await {
                error!(?err, "audit logs shuttle error");
            }
        });
    } else {
        // TODO(nick): this needs a tracker. In fact, func runner does too. We'll need a long term
        // solution for spwaning tasks in the dal.
        tokio::spawn(async move {
            if let Err(err) = audit_logs_shuttle.try_run().await {
                error!(?err, "audit logs shuttle error");
            }
            tracker.close();
            tracker.wait().await;
        });
    }

    Ok(())
}

#[instrument(name = "audit_logging.write", level = "debug", skip_all, fields(kind))]
pub(crate) async fn write(ctx: &DalContext, kind: AuditLogKind) -> Result<()> {
    // TODO(nick): nuke this from intergalactic orbit. Then do it again.
    let workspace_id = match ctx.workspace_pk() {
        Ok(workspace_id) => workspace_id,
        Err(TransactionsError::Tenancy(TenancyError::NoWorkspace)) => return Ok(()),
        Err(err) => return Err(AuditLoggingError::Transactions(Box::new(err))),
    };

    let pending_events_stream = PendingEventsStream::get_or_create(ctx.jetstream_context()).await?;
    pending_events_stream
        .publish_audit_log(
            workspace_id.into(),
            ctx.change_set_id().into(),
            ctx.event_session_id(),
            &AuditLog::new(
                ctx.events_actor(),
                kind,
                Utc::now(),
                workspace_id.into(),
                ctx.change_set_id().into(),
            ),
        )
        .await?;
    Ok(())
}

#[instrument(name = "audit_logging.write_final_message", level = "debug", skip_all)]
pub(crate) async fn write_final_message(ctx: &DalContext) -> Result<()> {
    // TODO(nick): nuke this from intergalactic orbit. Then do it again.
    let workspace_id = match ctx.workspace_pk() {
        Ok(workspace_id) => workspace_id,
        Err(TransactionsError::Tenancy(TenancyError::NoWorkspace)) => return Ok(()),
        Err(err) => return Err(AuditLoggingError::Transactions(Box::new(err))),
    };

    let pending_events_stream = PendingEventsStream::get_or_create(ctx.jetstream_context()).await?;
    pending_events_stream
        .publish_audit_log_final_message(
            workspace_id.into(),
            ctx.change_set_id().into(),
            ctx.event_session_id(),
        )
        .await?;
    Ok(())
}

#[instrument(name = "audit_logging.list", level = "debug", skip_all)]
pub async fn list(ctx: &DalContext) -> Result<Vec<FrontendAuditLog>> {
    // TODO(nick): nuke this from intergalactic orbit. Then do it again.
    let workspace_id = match ctx.workspace_pk() {
        Ok(workspace_id) => workspace_id,
        Err(TransactionsError::Tenancy(TenancyError::NoWorkspace)) => return Ok(Vec::new()),
        Err(err) => return Err(AuditLoggingError::Transactions(Box::new(err))),
    };

    let stream_wrapper = AuditLogsStream::get_or_create(ctx.jetstream_context()).await?;
    let stream = stream_wrapper.stream().await?;
    let consumer = stream
        .create_consumer(async_nats::jetstream::consumer::pull::Config {
            filter_subject: stream_wrapper.subject(workspace_id.into()).to_string(),
            ..Default::default()
        })
        .await?;

    // TODO(nick): remove hard-coded value.
    let mut messages = consumer.fetch().max_messages(200).messages().await?;
    let mut frontend_audit_logs = Vec::new();

    while let Some(Ok(message)) = messages.next().await {
        let audit_log: AuditLog = serde_json::from_slice(&message.payload)?;
        match audit_log {
            AuditLog::V2(inner) => {
                frontend_audit_logs.push(FrontendAuditLog {
                    actor: inner.actor,
                    kind: inner.kind,
                    timestamp: inner.timestamp,
                    workspace_id: inner.workspace_id,
                    change_set_id: inner.change_set_id,
                    actor_name: None,
                    actor_email: None,
                    origin_ip_address: None,
                    workspace_name: None,
                    change_set_name: None,
                });
            }
            AuditLog::V1(_) => {
                trace!("skipping older audit logs in beta...");
            }
        }
    }

    Ok(frontend_audit_logs)
}

pub mod temporary {
    use std::collections::HashSet;

    use si_events::audit_log::AuditLogKind;
    use si_events::Actor;
    use si_events::ChangeSetId;
    use si_events::UserPk;
    use si_frontend_types::AuditLog as FrontendAuditLog;

    use super::Result;

    #[allow(clippy::too_many_arguments)]
    pub fn filter_and_paginate(
        audit_logs: Vec<FrontendAuditLog>,
        page: Option<usize>,
        page_size: Option<usize>,
        sort_timestamp_ascending: Option<bool>,
        exclude_system_user: Option<bool>,
        kind_filter: HashSet<AuditLogKind>,
        change_set_filter: HashSet<ChangeSetId>,
        user_filter: HashSet<UserPk>,
    ) -> Result<(Vec<FrontendAuditLog>, usize)> {
        // First, filter the logs based on our chosen filters. This logic works by processing each
        // audit log and assuming each log is within our desired scope by default. The instant that a
        // log does not meet our scope, we continue!
        let mut filtered_audit_logs = Vec::new();
        for audit_log in audit_logs {
            if !kind_filter.is_empty() && !kind_filter.contains(&audit_log.kind) {
                continue;
            }

            if let Some(change_set_id) = &audit_log.change_set_id {
                if !change_set_filter.is_empty() && !change_set_filter.contains(change_set_id) {
                    continue;
                }
            } else if !change_set_filter.is_empty() {
                continue;
            }

            match &audit_log.actor {
                Actor::User(user_pk) => {
                    if !user_filter.is_empty() && !user_filter.contains(user_pk) {
                        continue;
                    }
                }
                Actor::System => {
                    if let Some(true) = exclude_system_user {
                        continue;
                    }
                }
            }

            filtered_audit_logs.push(audit_log);
        }

        // After filtering, perform the sort.
        if let Some(true) = sort_timestamp_ascending {
            filtered_audit_logs.reverse();
        }

        // Count the number of audit logs after filtering, but before pagination. We need this so that
        // the frontend can know how many pages exists when paginating data.
        let total = filtered_audit_logs.len();

        // Finally, paginate and return.
        Ok((paginate(filtered_audit_logs, page, page_size), total))
    }

    fn paginate(
        logs: Vec<FrontendAuditLog>,
        page: Option<usize>,
        page_size: Option<usize>,
    ) -> Vec<FrontendAuditLog> {
        if let Some(page_size) = page_size {
            let target_page = page.unwrap_or(1);

            let mut current_page = 1;
            for chunk in logs.chunks(page_size) {
                if current_page == target_page {
                    return chunk.to_vec();
                }
                current_page += 1;
            }
            logs
        } else {
            logs
        }
    }
}
