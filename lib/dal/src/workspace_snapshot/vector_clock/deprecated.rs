use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use si_events::DeprecatedVectorClockId;

use crate::workspace_snapshot::lamport_clock::LamportClock;

#[derive(Default, Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct DeprecatedVectorClock {
    pub entries: HashMap<DeprecatedVectorClockId, LamportClock>,
}
