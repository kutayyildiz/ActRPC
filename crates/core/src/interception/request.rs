use crate::{
    action::ResolvedActionRecord, interception::InterceptionPhase, json_rpc::JsonRpcMessage,
    participant::Participant,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterceptionRequest {
    pub origin: Participant,
    pub message: JsonRpcMessage,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prior_actions: Vec<ResolvedActionRecord>,
}

impl InterceptionRequest {
    pub fn has_prior_actions(&self) -> bool {
        !self.prior_actions.is_empty()
    }

    pub fn phase(&self) -> Result<InterceptionPhase, &'static str> {
        self.message.phase()
    }
}
