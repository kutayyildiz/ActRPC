use crate::{
    error::{MethodCallError, MethodCatalogError},
    method::{MethodProvider, MethodSourceConfig, ProviderName},
};
use actrpc_core::json_rpc::{JsonRpcMessage, JsonRpcParams};
use actrpc_transport::{JsonRpcClient, JsonRpcClientProvider, TransportError};
use std::{collections::HashMap, sync::Arc};

use crate::method::MethodName;

pub struct MethodCatalog {
    providers: HashMap<ProviderName, Arc<dyn MethodProvider>>,
}

impl MethodCatalog {
    pub fn new(providers: HashMap<ProviderName, Arc<dyn MethodProvider>>) -> Self {
        Self { providers }
    }

    pub async fn from_configs<P>(
        configs: Vec<MethodSourceConfig>,
        client_provider: &P,
    ) -> Result<Self, MethodCatalogError>
    where
        P: JsonRpcClientProvider<
                Client = Arc<dyn JsonRpcClient<Error = TransportError>>,
                Error = TransportError,
            > + Send
            + Sync,
    {
        let mut providers = HashMap::new();

        for config in configs {
            let provider_name = config.name().clone();

            if providers.contains_key(&provider_name) {
                return Err(MethodCatalogError::DuplicateProvider {
                    provider: provider_name,
                });
            }

            let provider = config
                .build_provider(client_provider)
                .await
                .map_err(|source| MethodCatalogError::ProviderBuild {
                    provider: provider_name.clone(),
                    source,
                })?;

            providers.insert(provider_name, provider);
        }

        Ok(Self { providers })
    }

    pub fn provider(&self, name: &ProviderName) -> Option<&dyn MethodProvider> {
        self.providers.get(name).map(|provider| provider.as_ref())
    }

    pub fn providers(&self) -> impl Iterator<Item = &dyn MethodProvider> {
        self.providers.values().map(|provider| provider.as_ref())
    }

    pub fn request_message(
        &self,
        provider: &ProviderName,
        method: &MethodName,
        params: Option<JsonRpcParams>,
    ) -> Result<JsonRpcMessage, MethodCallError> {
        let provider_ref =
            self.providers
                .get(provider)
                .ok_or_else(|| MethodCallError::ProviderNotFound {
                    provider: provider.clone(),
                })?;

        provider_ref.request_message(method, params)
    }

    pub async fn send_message(
        &self,
        provider: &ProviderName,
        method: &MethodName,
        message: JsonRpcMessage,
    ) -> Result<JsonRpcMessage, MethodCallError> {
        let provider_ref =
            self.providers
                .get(provider)
                .ok_or_else(|| MethodCallError::ProviderNotFound {
                    provider: provider.clone(),
                })?;

        provider_ref.send_message(method, message).await
    }

    pub async fn call(
        &self,
        provider: &ProviderName,
        method: &MethodName,
        params: Option<JsonRpcParams>,
    ) -> Result<serde_json::Value, MethodCallError> {
        let provider_ref =
            self.providers
                .get(provider)
                .ok_or_else(|| MethodCallError::ProviderNotFound {
                    provider: provider.clone(),
                })?;

        provider_ref.call(method, params).await
    }
}
