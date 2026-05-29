use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[derive(Clone, Debug, thiserror::Error, PartialEq, Serialize, Deserialize)]
pub enum CodecError {
    #[error("serialization failed: {0}")]
    Serialize(String),

    #[error("deserialization failed: {0}")]
    Deserialize(String),

    #[error("invalid JSON-RPC structure")]
    InvalidJsonRpcStructure,

    #[error("missing required field: {field}")]
    MissingField { field: String },

    #[error("invalid field type for: {field}")]
    InvalidFieldType { field: String },
}
