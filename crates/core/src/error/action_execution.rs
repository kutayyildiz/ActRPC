use serde::{Deserialize, Serialize};

use super::policy::PolicyError;
use crate::action::ActionKind;

#[non_exhaustive]
#[derive(Debug, thiserror::Error, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionExecutionError {
    #[error("invalid execution parameters for action {action}")]
    InvalidParams { action: ActionKind },

    #[error("target not found: {target}")]
    TargetNotFound { target: String },

    #[error("external method call failed for {method}: {message}")]
    ExternalMethodFailed { method: String, message: String },

    #[error("interceptor not found: {name}")]
    InterceptorNotFound { name: String },

    #[error(transparent)]
    Policy(#[from] PolicyError),

    #[error("internal execution error: {message}")]
    Internal { message: String },
}
