use crate::{
    config::PipelineConfig, external_method::ExternalMethodConfig, interceptor::InterceptorConfig,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct OrchestratorConfig {
    #[serde(default)]
    pub external_methods: Vec<ExternalMethodConfig>,

    #[serde(default)]
    pub interceptors: Vec<InterceptorConfig>,

    #[serde(default)]
    pub pipelines: PipelineConfig,
}
