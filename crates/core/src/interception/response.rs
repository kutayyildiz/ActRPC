use crate::action::RequestedActionRecord;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterceptorContinuation {
    /// Rerun the same interceptor after orchestrator action execution.
    Reinvoke,
    /// Proceed to the next interceptor.
    Stop,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterceptionResponse {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<RequestedActionRecord>,
    pub continuation: InterceptorContinuation,
}

impl InterceptionResponse {
    pub fn has_actions(&self) -> bool {
        !self.actions.is_empty()
    }

    pub fn should_reinvoke(&self) -> bool {
        matches!(self.continuation, InterceptorContinuation::Reinvoke)
    }

    pub fn should_stop(&self) -> bool {
        matches!(self.continuation, InterceptorContinuation::Stop)
    }
}
