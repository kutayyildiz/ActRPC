use crate::{config::RuntimeConfig, error::ConfigError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct InterceptorRuntimeLimitsOverride {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_interception_reinvokes: Option<usize>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interception_request_timeout_ms: Option<u64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_actions_per_interception: Option<usize>,
}

impl InterceptorRuntimeLimitsOverride {
    pub fn validate(&self, interceptor_name: &str) -> Result<(), ConfigError> {
        if self.interception_request_timeout_ms == Some(0) {
            return Err(ConfigError::InvalidRuntimeLimit {
                config_path: format!(
                    "interceptors[name=\"{interceptor_name}\"].runtime.interception_request_timeout_ms"
                ),
                value: 0,
            });
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeLimitSource {
    Global,
    Interceptor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedInterceptorRuntimeLimits {
    pub max_interception_reinvokes: usize,
    pub max_interception_reinvokes_source: RuntimeLimitSource,
    pub interception_request_timeout_ms: u64,
    pub interception_request_timeout_ms_source: RuntimeLimitSource,
    pub max_actions_per_interception: usize,
    pub max_actions_per_interception_source: RuntimeLimitSource,
}

impl ResolvedInterceptorRuntimeLimits {
    pub fn resolve(
        global: &RuntimeConfig,
        overrides: Option<&InterceptorRuntimeLimitsOverride>,
    ) -> Self {
        let max_interception_reinvokes = overrides
            .and_then(|o| o.max_interception_reinvokes)
            .unwrap_or(global.max_interception_reinvokes);
        let max_interception_reinvokes_source = if overrides
            .and_then(|o| o.max_interception_reinvokes)
            .is_some()
        {
            RuntimeLimitSource::Interceptor
        } else {
            RuntimeLimitSource::Global
        };

        let interception_request_timeout_ms = overrides
            .and_then(|o| o.interception_request_timeout_ms)
            .unwrap_or(global.interception_request_timeout_ms);
        let interception_request_timeout_ms_source = if overrides
            .and_then(|o| o.interception_request_timeout_ms)
            .is_some()
        {
            RuntimeLimitSource::Interceptor
        } else {
            RuntimeLimitSource::Global
        };

        let max_actions_per_interception = overrides
            .and_then(|o| o.max_actions_per_interception)
            .unwrap_or(global.max_actions_per_interception);
        let max_actions_per_interception_source = if overrides
            .and_then(|o| o.max_actions_per_interception)
            .is_some()
        {
            RuntimeLimitSource::Interceptor
        } else {
            RuntimeLimitSource::Global
        };

        Self {
            max_interception_reinvokes,
            max_interception_reinvokes_source,
            interception_request_timeout_ms,
            interception_request_timeout_ms_source,
            max_actions_per_interception,
            max_actions_per_interception_source,
        }
    }

    pub fn reinvokes_config_hint(&self, interceptor: &str) -> String {
        config_hint(
            interceptor,
            self.max_interception_reinvokes_source,
            "max_interception_reinvokes",
        )
    }

    pub fn timeout_config_hint(&self, interceptor: &str) -> String {
        config_hint(
            interceptor,
            self.interception_request_timeout_ms_source,
            "interception_request_timeout_ms",
        )
    }

    pub fn actions_config_hint(&self, interceptor: &str) -> String {
        config_hint(
            interceptor,
            self.max_actions_per_interception_source,
            "max_actions_per_interception",
        )
    }
}

fn config_hint(interceptor: &str, source: RuntimeLimitSource, field: &str) -> String {
    match source {
        RuntimeLimitSource::Global => format!("runtime.{field}"),
        RuntimeLimitSource::Interceptor => {
            format!("interceptors[name=\"{interceptor}\"].runtime.{field}")
        }
    }
}
