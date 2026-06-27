use crate::interceptors::dynamic_policy::scope::ScopeId;
use std::{io, path::PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum DynamicPolicyError {
    #[error("invalid params: {message}")]
    InvalidParams { message: String },

    #[error("scope not found: {scope_id}")]
    ScopeNotFound { scope_id: ScopeId },

    #[error("invalid config: {message}")]
    InvalidConfig { message: String },

    #[error("failed to read config at {path}: {source}")]
    ConfigRead {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to deserialize TOML config at {path}: {source}")]
    ConfigDeserializeToml {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("failed to deserialize YAML config at {path}: {source}")]
    ConfigDeserializeYaml {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },

    #[error("unsupported config format for {path}")]
    UnsupportedConfigFormat { path: PathBuf },

    #[error("action encoding failed: {source}")]
    ActionEncoding {
        #[from]
        source: serde_json::Error,
    },

    #[error("invalid query result: {message}")]
    InvalidQueryResult { message: String },

    #[error("invalid review result: {message}")]
    InvalidReviewResult { message: String },

    #[error("invalid dynamic policy context: {message}")]
    InvalidContext { message: String },

    #[error("internal store error: {message}")]
    Store { message: String },
}

impl DynamicPolicyError {
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::InvalidParams {
            message: message.into(),
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
