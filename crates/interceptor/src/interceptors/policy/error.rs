use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    #[error("failed to read policy config file {path}: {source}")]
    ConfigRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("unsupported policy config file format for {path}")]
    UnsupportedConfigFormat { path: PathBuf },

    #[error("failed to deserialize TOML policy config {path}: {source}")]
    ConfigDeserializeToml {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("failed to deserialize YAML policy config {path}: {source}")]
    ConfigDeserializeYaml {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },

    #[error("invalid policy config: {message}")]
    InvalidConfig { message: String },

    #[error("duplicate policy rule name: {name}")]
    DuplicateRuleName { name: String },

    #[error("invalid regex in rule {rule}: {source}")]
    InvalidRegex {
        rule: String,
        #[source]
        source: regex::Error,
    },

    #[error("invalid glob in rule {rule}: {source}")]
    InvalidGlob {
        rule: String,
        #[source]
        source: globset::Error,
    },

    #[error("failed to encode policy effect action: {source}")]
    ActionEncoding {
        #[source]
        source: serde_json::Error,
    },

    #[error("invalid review result: {message}")]
    InvalidReviewResult { message: String },
}
