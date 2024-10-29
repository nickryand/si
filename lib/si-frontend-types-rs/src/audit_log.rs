use serde::{Deserialize, Serialize};
use si_events::{audit_log::AuditLogKind, Actor, ChangeSetId, WorkspacePk};

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuditLog {
    pub actor: Actor,
    pub kind: AuditLogKind,
    pub timestamp: String,

    pub workspace_id: WorkspacePk,
    pub change_set_id: Option<ChangeSetId>,

    pub actor_name: Option<String>,
    pub actor_email: Option<String>,
    pub origin_ip_address: Option<String>,
    pub workspace_name: Option<String>,
    pub change_set_name: Option<String>,
}
