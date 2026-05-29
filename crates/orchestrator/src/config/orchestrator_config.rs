use crate::{config::PipelineConfig, interceptor::InterceptorConfig, method::MethodSourceConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct OrchestratorConfig {
    #[serde(default)]
    pub methods: Vec<MethodSourceConfig>,

    #[serde(default)]
    pub interceptors: Vec<InterceptorConfig>,

    #[serde(default)]
    pub pipelines: PipelineConfig,
}
