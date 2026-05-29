use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[derive(Debug, thiserror::Error, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProtocolError {
    #[error("expected method {expected}, got {actual}")]
    UnexpectedMethod { expected: String, actual: String },

    #[error("invalid request params")]
    InvalidRequestParams,

    #[error("mixed JSON-RPC batch is invalid")]
    MixedBatch,

    #[error("invalid message direction: {reason}")]
    InvalidMessageDirection { reason: String },
}
