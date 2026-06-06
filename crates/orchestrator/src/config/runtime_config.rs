use crate::error::ConfigError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeConfig {
    #[serde(default = "default_max_call_depth")]
    pub max_call_depth: usize,

    #[serde(default = "default_max_interception_reinvokes")]
    pub max_interception_reinvokes: usize,

    #[serde(default = "default_interception_request_timeout_ms")]
    pub interception_request_timeout_ms: u64,

    #[serde(default = "default_max_actions_per_interception")]
    pub max_actions_per_interception: usize,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_call_depth: default_max_call_depth(),
            max_interception_reinvokes: default_max_interception_reinvokes(),
            interception_request_timeout_ms: default_interception_request_timeout_ms(),
            max_actions_per_interception: default_max_actions_per_interception(),
        }
    }
}

fn default_max_call_depth() -> usize {
    8
}

fn default_max_interception_reinvokes() -> usize {
    8
}

fn default_interception_request_timeout_ms() -> u64 {
    30_000
}

fn default_max_actions_per_interception() -> usize {
    64
}

impl RuntimeConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.interception_request_timeout_ms == 0 {
            return Err(ConfigError::InvalidRuntimeLimit {
                config_path: "runtime.interception_request_timeout_ms".to_owned(),
                value: 0,
            });
        }

        Ok(())
    }
}
