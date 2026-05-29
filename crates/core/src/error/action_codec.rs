use crate::action::ActionKind;

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum ActionCodecError {
    /// thrown when requested/resolved action andn requested/resolved action record types does not
    /// match
    #[error("action kind mismatch: expected {expected}, got {actual}")]
    KindMismatch {
        expected: ActionKind,
        actual: ActionKind,
    },
    /// thrown when requested action record params doesnt match with the requested action type
    #[error("invalid params for action {action}: {source}")]
    InvalidParams {
        action: ActionKind,
        #[source]
        source: serde_json::Error,
    },
    /// thrown when resolved action record result cannot be decoded to resolved action result
    #[error("invalid result for action {action}: {source}")]
    InvalidResult {
        action: ActionKind,
        #[source]
        source: serde_json::Error,
    },
    /// thrown when requested action record does not have params but requested action does
    #[error("missing params for action {action}")]
    MissingParams { action: ActionKind },
    /// thrown when resolved action record does not have params but resolved action does
    #[error("missing ok result for action {action}")]
    MissingOkResult { action: ActionKind },
}
