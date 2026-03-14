use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Outbound,
    Inbound,
}

impl Phase {
    pub fn is_outbound(self) -> bool {
        matches!(self, Self::Outbound)
    }
    pub fn is_inbound(self) -> bool {
        matches!(self, Self::Inbound)
    }
}
