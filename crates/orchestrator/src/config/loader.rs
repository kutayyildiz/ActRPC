use crate::{config::OrchestratorConfig, error::ConfigError};
use std::{
    collections::HashSet,
    ffi::OsStr,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    Toml,
    Yaml,
}

impl OrchestratorConfig {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();

        let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;

        let format = ConfigFormat::from_path(path)?;

        Self::from_str_with_format(&text, format, path)
    }

    pub fn from_paths<I, P>(paths: I) -> Result<Self, ConfigError>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let mut paths = paths.into_iter().peekable();

        if paths.peek().is_none() {
            return Err(ConfigError::NoConfigPaths);
        }

        let mut merged = OrchestratorConfig::default();

        for path in paths {
            let config = Self::from_path(path)?;
            merged.merge_append(config)?;
        }

        Ok(merged)
    }

    pub fn from_str_with_format(
        text: &str,
        format: ConfigFormat,
        path_for_errors: impl AsRef<Path>,
    ) -> Result<Self, ConfigError> {
        let path = path_for_errors.as_ref().to_path_buf();

        match format {
            ConfigFormat::Toml => {
                toml::from_str(text).map_err(|source| ConfigError::DeserializeToml { path, source })
            }
            ConfigFormat::Yaml => serde_yaml::from_str(text)
                .map_err(|source| ConfigError::DeserializeYaml { path, source }),
        }
    }

    fn merge_append(&mut self, other: OrchestratorConfig) -> Result<(), ConfigError> {
        self.ensure_no_duplicate_endpoints(&other)?;
        self.ensure_no_duplicate_method_sources(&other)?;
        self.ensure_no_duplicate_interceptors(&other)?;

        self.endpoints.extend(other.endpoints);
        self.methods.extend(other.methods);
        self.interceptors.extend(other.interceptors);
        self.pipelines.outbound.extend(other.pipelines.outbound);
        self.pipelines.inbound.extend(other.pipelines.inbound);

        if let Some(runtime) = other.runtime {
            self.runtime = Some(runtime);
        }

        Ok(())
    }

    fn ensure_no_duplicate_method_sources(
        &self,
        other: &OrchestratorConfig,
    ) -> Result<(), ConfigError> {
        let mut names = HashSet::new();

        for config in &self.methods {
            names.insert(config.name().clone());
        }

        for config in &other.methods {
            let name = config.name().clone();

            if !names.insert(name.clone()) {
                return Err(ConfigError::DuplicateMethodProvider { name });
            }
        }

        Ok(())
    }

    fn ensure_no_duplicate_interceptors(
        &self,
        other: &OrchestratorConfig,
    ) -> Result<(), ConfigError> {
        let mut names = HashSet::new();

        for config in &self.interceptors {
            names.insert(config.name.clone());
        }

        for config in &other.interceptors {
            if !names.insert(config.name.clone()) {
                return Err(ConfigError::DuplicateInterceptor {
                    name: config.name.clone(),
                });
            }
        }

        Ok(())
    }

    fn ensure_no_duplicate_endpoints(&self, other: &OrchestratorConfig) -> Result<(), ConfigError> {
        let mut names = std::collections::HashSet::new();

        for config in &self.endpoints {
            names.insert(config.name.clone());
        }

        for config in &other.endpoints {
            let name = config.name.clone();

            if !names.insert(name.clone()) {
                return Err(ConfigError::DuplicateEndpoint { name });
            }
        }

        Ok(())
    }
}

impl ConfigFormat {
    pub fn from_path(path: &Path) -> Result<Self, ConfigError> {
        match path.extension().and_then(OsStr::to_str) {
            Some("toml") => Ok(Self::Toml),
            Some("yaml") | Some("yml") => Ok(Self::Yaml),
            _ => Err(ConfigError::UnsupportedFormat {
                path: PathBuf::from(path),
            }),
        }
    }
}
