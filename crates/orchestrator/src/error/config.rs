use crate::external_method::MethodName;
use std::path::PathBuf;

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("no config paths were provided")]
    NoConfigPaths,

    #[error("failed to read config file {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("unsupported config file format for {path}")]
    UnsupportedFormat { path: PathBuf },

    #[error("failed to deserialize TOML config {path}: {source}")]
    DeserializeToml {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("failed to deserialize YAML config {path}: {source}")]
    DeserializeYaml {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },

    #[error("duplicate external method config: {name}")]
    DuplicateExternalMethod { name: MethodName },

    #[error("duplicate interceptor config: {name}")]
    DuplicateInterceptor { name: String },
}
