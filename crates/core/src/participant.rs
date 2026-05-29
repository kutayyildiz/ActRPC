use serde::{Deserialize, Serialize};
use std::fmt;
use strum::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ParticipantType {
    User,
    Interceptor,
    Orchestrator,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Participant {
    pub kind: ParticipantType,
    pub id: String,
}

impl fmt::Display for Participant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.kind, self.id)
    }
}
