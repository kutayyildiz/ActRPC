use crate::action::ActionKind;

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum ActionCodecError {
    #[error("unknown action kind: {kind}")]
    UnknownActionKind { kind: String },

    #[error("action kind mismatch: expected {expected}, got {actual}")]
    KindMismatch {
        expected: ActionKind,
        actual: ActionKind,
    },

    #[error("invalid params for action {action}: {source}")]
    InvalidParams {
        action: ActionKind,
        #[source]
        source: serde_json::Error,
    },

    #[error("invalid result for action {action}: {source}")]
    InvalidResult {
        action: ActionKind,
        #[source]
        source: serde_json::Error,
    },
}
