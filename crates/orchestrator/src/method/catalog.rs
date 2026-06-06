use crate::{
    endpoint::{EndpointCatalog, config::EndpointName},
    error::{MethodCallError, MethodCatalogError},
    method::{MethodProvider, MethodProviderSnapshot, MethodSourceConfig, ProviderName},
};
use actrpc_core::json_rpc::{JsonRpcMessage, JsonRpcParams};
use std::{collections::HashMap, sync::Arc};

use crate::method::MethodName;

pub struct MethodCatalog {
    providers: HashMap<ProviderName, Arc<dyn MethodProvider>>,
}

impl MethodCatalog {
    pub fn new(providers: HashMap<ProviderName, Arc<dyn MethodProvider>>) -> Self {
        Self { providers }
    }

    pub async fn from_configs(
        configs: Vec<MethodSourceConfig>,
        endpoint_catalog: &EndpointCatalog,
    ) -> Result<Self, MethodCatalogError> {
        let mut providers = HashMap::new();
        for config in configs {
            let provider_name = config.name().clone();
            if providers.contains_key(&provider_name) {
                return Err(MethodCatalogError::DuplicateProvider {
                    provider: provider_name,
                });
            }
            let provider = config
                .build_provider(endpoint_catalog)
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

    pub async fn refresh_provider(
        &self,
        provider: &ProviderName,
    ) -> Result<MethodProviderSnapshot, MethodCatalogError> {
        let provider_ref =
            self.providers
                .get(provider)
                .ok_or_else(|| MethodCatalogError::UnknownProvider {
                    provider: provider.clone(),
                })?;
        provider_ref
            .refresh()
            .await
            .map_err(|source| MethodCatalogError::ProviderRefresh {
                provider: provider.clone(),
                source,
            })
    }

    pub async fn handle_method_provider_changed(
        &self,
        endpoint: &EndpointName,
        provider: &ProviderName,
        _version: Option<String>,
    ) -> Result<(), MethodCatalogError> {
        let provider_ref =
            self.providers
                .get(provider)
                .ok_or_else(|| MethodCatalogError::UnknownProvider {
                    provider: provider.clone(),
                })?;

        match provider_ref.endpoint() {
            Some(ep) if ep == endpoint => {}
            _ => {
                return Err(MethodCatalogError::ProviderEndpointMismatch {
                    endpoint: endpoint.clone(),
                    provider: provider.clone(),
                });
            }
        }

        if !provider_ref.is_watchable() {
            return Err(MethodCatalogError::ProviderNotWatchable {
                provider: provider.clone(),
            });
        }

        self.refresh_provider(provider).await?;
        Ok(())
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
