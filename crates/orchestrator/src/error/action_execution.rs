use actrpc_core::{action::ActionKind, interception::InterceptionPhase};

#[non_exhaustive]
#[derive(Debug, thiserror::Error, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum ActionExecutionError {
    #[error("invalid execution parameters for action {action}")]
    InvalidParams { action: ActionKind },

    #[error("referenced resource not found: {target}")]
    NotFound { target: String },

    #[error("dependency failure in {dependency}: {message}")]
    DependencyFailed { dependency: String, message: String },

    #[error("forbidden action: {action}")]
    ForbiddenAction { action: ActionKind },

    #[error("action {action} is not allowed during phase {phase}")]
    InvalidPhaseUsage {
        action: ActionKind,
        phase: InterceptionPhase,
    },

    #[error("invalid runtime state: {message}")]
    InvalidState { message: String },

    #[error("internal execution error: {message}")]
    Internal { message: String },
}
