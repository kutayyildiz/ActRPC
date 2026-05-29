use crate::interceptors::policy::{config::PolicyConfig, error::PolicyError};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyConfigFormat {
    Toml,
    Yaml,
}

impl PolicyConfig {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, PolicyError> {
        let path = path.as_ref();

        let format = PolicyConfigFormat::from_path(path)?;

        let text = std::fs::read_to_string(path).map_err(|source| PolicyError::ConfigRead {
            path: path.to_path_buf(),
            source,
        })?;

        Self::from_str_with_format(&text, format, path)
    }

    pub fn from_str_with_format(
        text: &str,
        format: PolicyConfigFormat,
        path_for_errors: impl AsRef<Path>,
    ) -> Result<Self, PolicyError> {
        let path = path_for_errors.as_ref().to_path_buf();

        match format {
            PolicyConfigFormat::Toml => toml::from_str(text)
                .map_err(|source| PolicyError::ConfigDeserializeToml { path, source }),
            PolicyConfigFormat::Yaml => serde_yaml::from_str(text)
                .map_err(|source| PolicyError::ConfigDeserializeYaml { path, source }),
        }
    }
}

impl PolicyConfigFormat {
    pub fn from_path(path: &Path) -> Result<Self, PolicyError> {
        match path.extension().and_then(OsStr::to_str) {
            Some("toml") => Ok(Self::Toml),
            Some("yaml") | Some("yml") => Ok(Self::Yaml),
            _ => Err(PolicyError::UnsupportedConfigFormat {
                path: PathBuf::from(path),
            }),
        }
    }
}
