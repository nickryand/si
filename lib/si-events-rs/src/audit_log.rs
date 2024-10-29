use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumIter, EnumString};

use crate::{Actor, ChangeSetId, WorkspacePk};

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub enum AuditLog {
    V2(AuditLogV2),
    V1(AuditLogV1),
}

impl AuditLog {
    pub fn new(
        actor: Actor,
        kind: AuditLogKind,
        timestamp: DateTime<Utc>,
        workspace_id: WorkspacePk,
        change_set_id: ChangeSetId,
    ) -> Self {
        Self::V2(AuditLogV2 {
            actor,
            kind,
            timestamp: timestamp.to_rfc3339(),
            workspace_id,
            change_set_id: Some(change_set_id),
        })
    }
}

pub type AuditLogKind = AuditLogKindV2;

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct AuditLogV2 {
    pub actor: Actor,
    pub kind: AuditLogKindV2,
    pub timestamp: String,
    pub workspace_id: WorkspacePk,
    pub change_set_id: Option<ChangeSetId>,
}

type AuditLogKindV2 = AuditLogKindV1;

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct AuditLogV1 {
    pub actor: Actor,
    pub kind: AuditLogKindV1,
    pub timestamp: String,
}

#[remain::sorted]
#[derive(
    Clone,
    Debug,
    Deserialize,
    Serialize,
    Eq,
    PartialEq,
    Hash,
    AsRefStr,
    Display,
    EnumIter,
    EnumString,
)]
pub enum AuditLogKindV1 {
    CreateComponent,
    DeleteComponent,
    PerformRebase,
    RunAction,
    RunComputeValidations,
    RunDependentValuesUpdate,
    UpdatePropertyEditorValue,
}
