use crate::interceptors::call_request::error::CallRequestError;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    ffi::OsStr,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallRequestConfigFormat {
    Toml,
    Yaml,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstructorConfig {
    pub version: u32,
    pub rules: Vec<PromptInjectionRule>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptInjectionRule {
    pub name: String,
    pub provider: String,
    pub method: String,
    pub prompt_field: String,

    #[serde(default, skip_serializing_if = "PromptInjection::is_empty")]
    pub injection: PromptInjection,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptInjection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prepend: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub append: Option<String>,
}

impl PromptInjection {
    pub fn is_empty(&self) -> bool {
        self.prepend.is_none() && self.append.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutorConfig {
    pub version: u32,

    #[serde(default = "default_call_requests_field")]
    pub call_requests_field: String,

    #[serde(default = "default_results_field")]
    pub results_field: String,
}

fn default_call_requests_field() -> String {
    "_actrpc_call_requests".to_owned()
}

fn default_results_field() -> String {
    "_actrpc_call_results".to_owned()
}

impl InstructorConfig {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, CallRequestError> {
        let path = path.as_ref();
        let format = CallRequestConfigFormat::from_path(path)?;
        let text = std::fs::read_to_string(path).map_err(|source| CallRequestError::ConfigRead {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_str_with_format(&text, format, path)
    }

    pub fn from_str_with_format(
        text: &str,
        format: CallRequestConfigFormat,
        path_for_errors: impl AsRef<Path>,
    ) -> Result<Self, CallRequestError> {
        let path = path_for_errors.as_ref().to_path_buf();
        let config: Self = match format {
            CallRequestConfigFormat::Toml => toml::from_str(text)
                .map_err(|source| CallRequestError::ConfigDeserializeToml { path, source })?,
            CallRequestConfigFormat::Yaml => serde_yaml::from_str(text)
                .map_err(|source| CallRequestError::ConfigDeserializeYaml { path, source })?,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), CallRequestError> {
        if self.version != 1 {
            return Err(CallRequestError::InvalidConfig {
                message: format!("unsupported instructor config version: {}", self.version),
            });
        }

        if self.rules.is_empty() {
            return Err(CallRequestError::InvalidConfig {
                message: "instructor config rules must not be empty".to_owned(),
            });
        }

        let mut seen_names = HashSet::new();
        for rule in &self.rules {
            if rule.name.trim().is_empty() {
                return Err(CallRequestError::InvalidConfig {
                    message: "rule.name must not be empty".to_owned(),
                });
            }
            if rule.provider.trim().is_empty() {
                return Err(CallRequestError::InvalidConfig {
                    message: format!("rule {} provider must not be empty", rule.name),
                });
            }
            if rule.method.trim().is_empty() {
                return Err(CallRequestError::InvalidConfig {
                    message: format!("rule {} method must not be empty", rule.name),
                });
            }
            if rule.prompt_field.trim().is_empty() {
                return Err(CallRequestError::InvalidConfig {
                    message: format!("rule {} prompt_field must not be empty", rule.name),
                });
            }
            if rule.injection.is_empty() {
                return Err(CallRequestError::InvalidConfig {
                    message: format!(
                        "rule {} injection must include prepend or append",
                        rule.name
                    ),
                });
            }
            if !seen_names.insert(rule.name.clone()) {
                return Err(CallRequestError::InvalidConfig {
                    message: format!("duplicate rule name: {}", rule.name),
                });
            }
        }

        Ok(())
    }
}

impl ExecutorConfig {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, CallRequestError> {
        let path = path.as_ref();
        let format = CallRequestConfigFormat::from_path(path)?;
        let text = std::fs::read_to_string(path).map_err(|source| CallRequestError::ConfigRead {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_str_with_format(&text, format, path)
    }

    pub fn from_str_with_format(
        text: &str,
        format: CallRequestConfigFormat,
        path_for_errors: impl AsRef<Path>,
    ) -> Result<Self, CallRequestError> {
        let path = path_for_errors.as_ref().to_path_buf();
        let config: Self = match format {
            CallRequestConfigFormat::Toml => toml::from_str(text)
                .map_err(|source| CallRequestError::ConfigDeserializeToml { path, source })?,
            CallRequestConfigFormat::Yaml => serde_yaml::from_str(text)
                .map_err(|source| CallRequestError::ConfigDeserializeYaml { path, source })?,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), CallRequestError> {
        if self.version != 1 {
            return Err(CallRequestError::InvalidConfig {
                message: format!("unsupported executor config version: {}", self.version),
            });
        }
        Ok(())
    }
}

impl CallRequestConfigFormat {
    pub fn from_path(path: &Path) -> Result<Self, CallRequestError> {
        match path.extension().and_then(OsStr::to_str) {
            Some("toml") => Ok(Self::Toml),
            Some("yaml") | Some("yml") => Ok(Self::Yaml),
            _ => Err(CallRequestError::UnsupportedConfigFormat {
                path: PathBuf::from(path),
            }),
        }
    }
}