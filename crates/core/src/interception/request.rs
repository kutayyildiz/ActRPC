use crate::{
    action::ResolvedActionRecord, error::ProtocolError, interception::InterceptionPhase,
    json_rpc::JsonRpcMessage, participant::Participant,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterceptionRequest {
    pub origin: Participant,
    pub message: JsonRpcMessage,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resolved_action_history: Vec<Vec<ResolvedActionRecord>>,
}

impl InterceptionRequest {
    pub fn has_resolved_actions(&self) -> bool {
        self.resolved_action_history
            .iter()
            .any(|actions| !actions.is_empty())
    }

    pub fn has_prior_actions(&self) -> bool {
        self.has_resolved_actions()
    }

    pub fn get_last_resolved_actions(&self) -> Option<&[ResolvedActionRecord]> {
        self.resolved_action_history.last().map(Vec::as_slice)
    }

    pub fn iter_resolved_action_rounds(&self) -> impl Iterator<Item = &[ResolvedActionRecord]> {
        self.resolved_action_history.iter().map(Vec::as_slice)
    }

    pub fn phase(&self) -> Result<InterceptionPhase, ProtocolError> {
        self.message.phase()
    }
}
