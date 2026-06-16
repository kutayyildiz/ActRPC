use crate::interceptors::dynamic_policy::scope::ScopeId;
use globset::Error as GlobError;

pub const JSON_RPC_INVALID_PARAMS: i32 = -32602;
pub const JSON_RPC_METHOD_NOT_FOUND: i32 = -32601;
pub const JSON_RPC_SERVER_ERROR: i32 = -32000;
pub const SCOPE_NOT_FOUND_CODE: i32 = -32012;
pub const CREATOR_MISMATCH_CODE: i32 = -32013;

#[derive(Debug, thiserror::Error)]
pub enum DynamicPolicyError {
    #[error("invalid params: {message}")]
    InvalidParams { message: String },

    #[error("scope not found: {scope_id}")]
    ScopeNotFound { scope_id: ScopeId },

    #[error("creator mismatch for scope {scope_id}")]
    CreatorMismatch { scope_id: ScopeId },

    #[error("invalid glob in target selector: {source}")]
    InvalidGlob { source: GlobError },

    #[error("unknown method: {method}")]
    MethodNotFound { method: String },

    #[error("action encoding failed: {source}")]
    ActionEncoding {
        #[from]
        source: serde_json::Error,
    },

    #[error("invalid query result: {message}")]
    InvalidQueryResult { message: String },

    #[error("internal store error: {message}")]
    Store { message: String },
}

impl DynamicPolicyError {
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::InvalidParams {
            message: message.into(),
        }
    }

    pub fn json_rpc_code(&self) -> i32 {
        match self {
            Self::InvalidParams { .. } | Self::InvalidGlob { .. } => JSON_RPC_INVALID_PARAMS,
            Self::ScopeNotFound { .. } => SCOPE_NOT_FOUND_CODE,
            Self::CreatorMismatch { .. } => CREATOR_MISMATCH_CODE,
            Self::MethodNotFound { .. } => JSON_RPC_METHOD_NOT_FOUND,
            Self::ActionEncoding { .. } | Self::InvalidQueryResult { .. } | Self::Store { .. } => {
                JSON_RPC_SERVER_ERROR
            }
        }
    }
}

impl From<DynamicPolicyError> for actrpc_orchestrator::error::InterceptorRuntimeError {
    fn from(error: DynamicPolicyError) -> Self {
        Self::Request {
            message: error.to_string(),
        }
    }
}
