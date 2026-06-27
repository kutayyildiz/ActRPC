use std::{io, path::PathBuf};

use actrpc_core::error::ProtocolError;

#[derive(Debug, thiserror::Error)]
pub enum CallRequestError {
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
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

    #[error("invalid config: {message}")]
    InvalidConfig { message: String },

    #[error("invalid call request: {message}")]
    InvalidCallRequest { message: String },

    #[error("action encoding failed: {source}")]
    ActionEncoding {
        #[from]
        source: serde_json::Error,
    },
}

impl From<CallRequestError> for actrpc_orchestrator::error::InterceptorRuntimeError {
    fn from(error: CallRequestError) -> Self {
        Self::Request {
            message: error.to_string(),
        }
    }
}
