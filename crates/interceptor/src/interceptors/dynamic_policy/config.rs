use crate::interceptors::dynamic_policy::error::DynamicPolicyError;
use actrpc_core::MethodTarget;
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicPolicyConfigFormat {
    Toml,
    Yaml,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DynamicPolicyConfig {
    #[serde(default)]
    pub unscoped_policy: UnscopedPolicy,
}

impl Default for DynamicPolicyConfig {
    fn default() -> Self {
        Self {
            unscoped_policy: UnscopedPolicy::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnscopedPolicy {
    #[serde(default)]
    pub on_unscoped: UnscopedBehavior,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_method_targets: Vec<MethodTarget>,
}

impl Default for UnscopedPolicy {
    fn default() -> Self {
        Self {
            on_unscoped: UnscopedBehavior::Allow,
            allowed_method_targets: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum UnscopedBehavior {
    #[default]
    Allow,
    Reject,
    #[serde(rename = "scope")]
    ScopeRoot,
}

impl DynamicPolicyConfig {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, DynamicPolicyError> {
        let path = path.as_ref();
        let format = DynamicPolicyConfigFormat::from_path(path)?;
        let text =
            std::fs::read_to_string(path).map_err(|source| DynamicPolicyError::ConfigRead {
                path: path.to_path_buf(),
                source,
            })?;
        Self::from_str_with_format(&text, format, path)
    }

    pub fn from_str_with_format(
        text: &str,
        format: DynamicPolicyConfigFormat,
        path_for_errors: impl AsRef<Path>,
    ) -> Result<Self, DynamicPolicyError> {
        let path = path_for_errors.as_ref().to_path_buf();
        let config: Self = match format {
            DynamicPolicyConfigFormat::Toml => toml::from_str(text)
                .map_err(|source| DynamicPolicyError::ConfigDeserializeToml { path, source })?,
            DynamicPolicyConfigFormat::Yaml => serde_yaml::from_str(text)
                .map_err(|source| DynamicPolicyError::ConfigDeserializeYaml { path, source })?,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), DynamicPolicyError> {
        if self.unscoped_policy.on_unscoped == UnscopedBehavior::ScopeRoot
            && self.unscoped_policy.allowed_method_targets.is_empty()
        {
            return Err(DynamicPolicyError::InvalidConfig {
                message:
                    "unscoped_policy.on_unscoped = scope requires non-empty allowed_method_targets"
                        .to_owned(),
            });
        }
        Ok(())
    }
}

impl DynamicPolicyConfigFormat {
    pub fn from_path(path: &Path) -> Result<Self, DynamicPolicyError> {
        match path.extension().and_then(OsStr::to_str) {
            Some("toml") => Ok(Self::Toml),
            Some("yaml") | Some("yml") => Ok(Self::Yaml),
            _ => Err(DynamicPolicyError::UnsupportedConfigFormat {
                path: PathBuf::from(path),
            }),
        }
    }
}
