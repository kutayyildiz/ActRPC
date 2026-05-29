use crate::error::ActionHandlerError;
use actrpc_core::{action::ActionKind, interception::InterceptionPhase};

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum ActionError {
    #[error("no registered action handler for action {action}")]
    HandlerNotFound { action: ActionKind },

    #[error("action handler failed for interceptor {interceptor}, action {action}: {source}")]
    HandlerFailed {
        interceptor: String,
        action: ActionKind,
        #[source]
        source: ActionHandlerError,
    },

    #[error("duplicate action registration for kind {kind}")]
    DuplicateRegistration { kind: ActionKind },

    #[error(
        "interceptor {interceptor} is not allowed to execute action {action} during phase {phase}"
    )]
    ForbiddenByPolicy {
        interceptor: String,
        action: ActionKind,
        phase: InterceptionPhase,
    },
}
