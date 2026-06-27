use crate::{
    endpoint::EndpointCatalog,
    error::MethodProviderBuildError,
    method::{
        MethodProvider, ProviderName,
        providers::{
            json_rpc::{JsonRpcMethodProvider, JsonRpcMethodSourceConfig},
            mcp::{McpMethodProvider, McpMethodSourceConfig},
            rest::{RestMethodProvider, RestMethodSourceConfig},
        },
    },
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MethodSourceConfig {
    JsonRpc(JsonRpcMethodSourceConfig),
    Mcp(McpMethodSourceConfig),
    Rest(RestMethodSourceConfig),
}

impl MethodSourceConfig {
    pub fn name(&self) -> &ProviderName {
        match self {
            Self::JsonRpc(config) => &config.provider,
            Self::Mcp(config) => &config.name,
            Self::Rest(config) => &config.provider,
        }
    }

    pub async fn build_provider(
        self,
        endpoint_catalog: &EndpointCatalog,
    ) -> Result<Arc<dyn MethodProvider>, MethodProviderBuildError> {
        match self {
            Self::JsonRpc(config) => {
                let provider = JsonRpcMethodProvider::from_config(config, endpoint_catalog).await?;
                Ok(Arc::new(provider) as Arc<dyn MethodProvider>)
            }
            Self::Mcp(config) => {
                let provider = McpMethodProvider::from_config(config, endpoint_catalog).await?;
                Ok(Arc::new(provider) as Arc<dyn MethodProvider>)
            }
            Self::Rest(config) => {
                let provider = RestMethodProvider::from_config(config, endpoint_catalog)?;
                Ok(Arc::new(provider) as Arc<dyn MethodProvider>)
            }
        }
    }
}
