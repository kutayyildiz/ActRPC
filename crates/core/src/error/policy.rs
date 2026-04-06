use serde::{Deserialize, Serialize};

use crate::{action::ActionKind, interception::InterceptionPhase};

#[non_exhaustive]
#[derive(Debug, thiserror::Error, Clone, Deserialize, Serialize, PartialEq)]
pub enum PolicyError {
    #[error("forbidden action: {action}")]
    ForbiddenAction { action: ActionKind },

    #[error("action {action} is not allowed during phase {phase}")]
    InvalidPhaseUsage {
        action: ActionKind,
        phase: InterceptionPhase,
    },

    #[error("pipeline violation: {reason}")]
    PipelineViolation { reason: String },

    #[error("interceptor not found in policy scope: {name}")]
    InterceptorNotFound { name: String },

    #[error("attempted to expand or reorder interceptor pipeline")]
    PipelineMutationNotAllowed,
}
