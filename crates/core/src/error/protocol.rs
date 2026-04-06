use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[derive(Debug, thiserror::Error, Clone, Deserialize, Serialize, PartialEq)]
pub enum ProtocolError {
    #[error("expected: {expected}, actual: {actual}")]
    UnexpectedMethod { expected: String, actual: String },

    #[error("invalid request params")]
    InvalidRequestParams,
}
