use crate::{
    executed_action::ExecutedAction, json_rpc::JsonRpcMessage, participant::Participant,
    phase::Phase,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterceptionRequest {
    pub origin: Participant,
    pub message: JsonRpcMessage, // ← see below
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executed_actions: Option<Vec<ExecutedAction>>,
}
impl InterceptionRequest {
    pub fn has_previous_actions(&self) -> bool {
        match &self.executed_actions {
            Some(actions) => !actions.is_empty(),
            None => false,
        }
    }

    pub fn phase(&self) -> Phase {
        if matches!(self.message, JsonRpcMessage::Response(_)) {
            Phase::Inbound
        } else {
            Phase::Outbound
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterceptionDecision {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<String>>,
    pub is_final: bool,
}
