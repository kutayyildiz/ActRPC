use crate::{
    error::MethodProviderBuildError,
    method::{
        MethodProvider, ProviderName,
        providers::{
            mcp::{McpMethodProvider, McpMethodSourceConfig},
            native::{NativeMethodProvider, NativeMethodSourceConfig},
        },
    },
};
use actrpc_transport::{JsonRpcClient, JsonRpcClientProvider, TransportError};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MethodSourceConfig {
    Native(NativeMethodSourceConfig),
    Mcp(McpMethodSourceConfig),
}

impl MethodSourceConfig {
    pub fn name(&self) -> &ProviderName {
        match self {
            Self::Native(config) => &config.name,
            Self::Mcp(config) => &config.name,
        }
    }

    pub async fn build_provider<P>(
        self,
        client_provider: &P,
    ) -> Result<Arc<dyn MethodProvider>, MethodProviderBuildError>
    where
        P: JsonRpcClientProvider<
                Client = Arc<dyn JsonRpcClient<Error = TransportError>>,
                Error = TransportError,
            > + Send
            + Sync,
    {
        match self {
            Self::Native(config) => {
                let provider = NativeMethodProvider::from_config(config, client_provider).await?;
                Ok(Arc::new(provider) as Arc<dyn MethodProvider>)
            }

            Self::Mcp(config) => {
                let provider = McpMethodProvider::from_config(config, client_provider).await?;
                Ok(Arc::new(provider) as Arc<dyn MethodProvider>)
            }
        }
    }
}
