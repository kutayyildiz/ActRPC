use crate::{
    endpoint::{EndpointCatalog, EndpointCatalogError, JsonRpc2RequestEndpoint},
    error::{MethodCallError, MethodProviderBuildError, MethodProviderRefreshError},
    method::{
        MethodInfo, MethodName, MethodProvider, MethodProviderFuture, MethodProviderSnapshot,
        ProviderName,
        rpc_bridge::{
            invalid_request_message_response, logical_error_response, remap_json_rpc_response,
            request_internal_id,
        },
    },
};
use actrpc_core::json_rpc::{
    JsonRpcId, JsonRpcMessage, JsonRpcParams, JsonRpcRequest, JsonRpcResponse,
    JsonRpcSingleMessage, JsonRpcVersion,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    sync::{
        Arc, RwLock,
        atomic::{AtomicU64, Ordering},
    },
};

fn default_initialize_method() -> String {
    actrpc_core::ACTRPC_METHOD_PROVIDER_INITIALIZE_METHOD.to_owned()
}

fn default_refresh_method() -> String {
    actrpc_core::ACTRPC_METHOD_PROVIDER_REFRESH_METHOD.to_owned()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum JsonRpcMethodDiscoveryConfig {
    Static {
        methods: Vec<MethodInfo>,
    },
    Initialize {
        #[serde(default = "default_initialize_method")]
        method: String,
    },
    Refreshable {
        #[serde(default = "default_initialize_method")]
        initialize_method: String,
        #[serde(default = "default_refresh_method")]
        refresh_method: String,
    },
    Watchable {
        #[serde(default = "default_initialize_method")]
        initialize_method: String,
        #[serde(default = "default_refresh_method")]
        refresh_method: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonRpcMethodSourceConfig {
    pub provider: ProviderName,
    pub endpoint: crate::endpoint::config::EndpointName,
    pub discovery: JsonRpcMethodDiscoveryConfig,
}

pub struct JsonRpcMethodProvider {
    provider: ProviderName,
    endpoint: Arc<dyn JsonRpc2RequestEndpoint>,
    endpoint_name: crate::endpoint::config::EndpointName,
    snapshot: RwLock<MethodProviderSnapshot>,
    discovery: JsonRpcMethodDiscoveryConfig,
    next_id: AtomicU64,
}

fn json_rpc_id(next_id: &AtomicU64) -> JsonRpcId {
    JsonRpcId::Number(next_id.fetch_add(1, Ordering::Relaxed).into())
}

impl JsonRpcMethodProvider {
    pub async fn from_config(
        config: JsonRpcMethodSourceConfig,
        endpoint_catalog: &EndpointCatalog,
    ) -> Result<Self, MethodProviderBuildError> {
        let watchable = matches!(
            config.discovery,
            JsonRpcMethodDiscoveryConfig::Watchable { .. }
        );

        if watchable {
            endpoint_catalog
                .get_json_rpc2_session(&config.endpoint)
                .map_err(map_catalog_error(&config.provider))?;
        }

        let endpoint = endpoint_catalog
            .get_json_rpc2(&config.endpoint)
            .map_err(map_catalog_error(&config.provider))?;

        let endpoint_name = endpoint.endpoint_name().clone();
        let next_id = AtomicU64::new(1);

        let initial_snapshot = match &config.discovery {
            JsonRpcMethodDiscoveryConfig::Static { methods } => {
                let mut seen = HashSet::new();
                for m in methods {
                    if !seen.insert(m.name.clone()) {
                        return Err(MethodProviderBuildError::DuplicateMethod {
                            provider: config.provider.clone(),
                            method: m.name.clone(),
                        });
                    }
                }
                MethodProviderSnapshot {
                    provider: config.provider.clone(),
                    version: None,
                    description: None,
                    methods: methods.clone(),
                    info: serde_json::Value::Null,
                }
            }
            JsonRpcMethodDiscoveryConfig::Initialize { method } => {
                let snap =
                    Self::call_initialize(&endpoint, method, &config.provider, &next_id).await?;
                Self::validate_snapshot(&snap, &config.provider)?;
                snap
            }
            JsonRpcMethodDiscoveryConfig::Refreshable {
                initialize_method, ..
            }
            | JsonRpcMethodDiscoveryConfig::Watchable {
                initialize_method, ..
            } => {
                let snap =
                    Self::call_initialize(&endpoint, initialize_method, &config.provider, &next_id)
                        .await?;
                Self::validate_snapshot(&snap, &config.provider)?;
                snap
            }
        };

        Ok(Self {
            provider: config.provider,
            endpoint,
            endpoint_name,
            snapshot: RwLock::new(initial_snapshot),
            discovery: config.discovery,
            next_id,
        })
    }

    async fn call_initialize(
        endpoint: &Arc<dyn JsonRpc2RequestEndpoint>,
        method: &str,
        provider: &ProviderName,
        next_id: &AtomicU64,
    ) -> Result<MethodProviderSnapshot, MethodProviderBuildError> {
        let external_id = json_rpc_id(next_id);
        let req = JsonRpcRequest {
            jsonrpc: JsonRpcVersion::V2_0,
            id: external_id.clone(),
            method: method.to_owned(),
            params: None,
        };
        let resp = endpoint.request(req).await.map_err(|source| {
            MethodProviderBuildError::DiscoveryTransport {
                provider: provider.clone(),
                source,
            }
        })?;
        let remapped =
            remap_json_rpc_response(external_id.clone(), external_id, resp).map_err(|message| {
                MethodProviderBuildError::DiscoveryFailed {
                    provider: provider.clone(),
                    message,
                }
            })?;
        let JsonRpcResponse::Success(success) = remapped else {
            return Err(MethodProviderBuildError::DiscoveryFailed {
                provider: provider.clone(),
                message: "initialize returned error response".to_owned(),
            });
        };
        serde_json::from_value(success.result).map_err(|source| {
            MethodProviderBuildError::DiscoveryFailed {
                provider: provider.clone(),
                message: format!("failed to decode snapshot: {source}"),
            }
        })
    }

    fn validate_snapshot(
        snap: &MethodProviderSnapshot,
        expected: &ProviderName,
    ) -> Result<(), MethodProviderBuildError> {
        if &snap.provider != expected {
            return Err(MethodProviderBuildError::InvalidConfig {
                provider: expected.clone(),
                message: format!(
                    "snapshot provider mismatch: expected {}, got {}",
                    expected.as_str(),
                    snap.provider.as_str()
                ),
            });
        }
        let mut seen = HashSet::new();
        for m in &snap.methods {
            if !seen.insert(m.name.clone()) {
                return Err(MethodProviderBuildError::DuplicateMethod {
                    provider: expected.clone(),
                    method: m.name.clone(),
                });
            }
        }
        Ok(())
    }

    fn next_external_id(&self) -> JsonRpcId {
        json_rpc_id(&self.next_id)
    }
}

fn map_catalog_error(
    provider: &ProviderName,
) -> impl FnOnce(EndpointCatalogError) -> MethodProviderBuildError + '_ {
    move |source| MethodProviderBuildError::InvalidConfig {
        provider: provider.clone(),
        message: source.to_string(),
    }
}

impl MethodProvider for JsonRpcMethodProvider {
    fn name(&self) -> &ProviderName {
        &self.provider
    }

    fn endpoint(&self) -> Option<&crate::endpoint::EndpointName> {
        Some(&self.endpoint_name)
    }

    fn is_watchable(&self) -> bool {
        matches!(
            self.discovery,
            JsonRpcMethodDiscoveryConfig::Watchable { .. }
        )
    }

    fn snapshot(&self) -> MethodProviderSnapshot {
        self.snapshot
            .read()
            .expect("poisoned snapshot lock")
            .clone()
    }

    fn request_message(
        &self,
        method: &MethodName,
        params: Option<JsonRpcParams>,
    ) -> Result<JsonRpcMessage, MethodCallError> {
        let snap = self.snapshot();
        if !snap.methods.iter().any(|m| &m.name == method) {
            return Err(MethodCallError::MethodNotFound {
                provider: self.provider.clone(),
                method: method.clone(),
            });
        }
        Ok(JsonRpcMessage::Single(JsonRpcSingleMessage::Request(
            JsonRpcRequest {
                jsonrpc: JsonRpcVersion::V2_0,
                id: self.next_external_id(),
                method: method.as_str().to_owned(),
                params,
            },
        )))
    }

    fn send_message<'a>(
        &'a self,
        method: &'a MethodName,
        message: JsonRpcMessage,
    ) -> MethodProviderFuture<'a, Result<JsonRpcMessage, MethodCallError>> {
        Box::pin(async move {
            let request = match message {
                JsonRpcMessage::Single(JsonRpcSingleMessage::Request(request)) => request,
                other => {
                    return invalid_request_message_response(&self.provider, method, other);
                }
            };

            let Some(internal_id) = request_internal_id(&request.id) else {
                return Err(MethodCallError::InvalidResponse {
                    provider: self.provider.clone(),
                    method: method.clone(),
                    message: "provider send_message expected a request with id".to_owned(),
                });
            };

            let external_id = self.next_external_id();
            let external_req = JsonRpcRequest {
                jsonrpc: request.jsonrpc,
                id: external_id.clone(),
                method: method.as_str().to_owned(),
                params: request.params,
            };

            let resp = self
                .endpoint
                .request(external_req)
                .await
                .map_err(|source| MethodCallError::Transport {
                    provider: self.provider.clone(),
                    method: method.clone(),
                    source,
                })?;

            let remapped = match remap_json_rpc_response(internal_id.clone(), external_id, resp) {
                Ok(response) => response,
                Err(message) => {
                    return Ok(logical_error_response(internal_id, message));
                }
            };

            Ok(JsonRpcMessage::Single(JsonRpcSingleMessage::Response(
                remapped,
            )))
        })
    }

    fn refresh<'a>(
        &'a self,
    ) -> MethodProviderFuture<'a, Result<MethodProviderSnapshot, MethodProviderRefreshError>> {
        let refresh_method = match &self.discovery {
            JsonRpcMethodDiscoveryConfig::Refreshable { refresh_method, .. }
            | JsonRpcMethodDiscoveryConfig::Watchable { refresh_method, .. } => {
                refresh_method.clone()
            }
            _ => {
                let p = self.provider.clone();
                return Box::pin(async move {
                    Err(MethodProviderRefreshError::Unsupported { provider: p })
                });
            }
        };
        let provider = self.provider.clone();
        let endpoint = self.endpoint.clone();
        Box::pin(async move {
            let external_id = json_rpc_id(&self.next_id);
            let req = JsonRpcRequest {
                jsonrpc: JsonRpcVersion::V2_0,
                id: external_id.clone(),
                method: refresh_method,
                params: None,
            };
            let resp = endpoint.request(req).await.map_err(|source| {
                MethodProviderRefreshError::Transport {
                    provider: provider.clone(),
                    source,
                }
            })?;
            let remapped = remap_json_rpc_response(external_id.clone(), external_id, resp)
                .map_err(|message| MethodProviderRefreshError::Decode {
                    provider: provider.clone(),
                    message,
                })?;
            let JsonRpcResponse::Success(success) = remapped else {
                return Err(MethodProviderRefreshError::Decode {
                    provider: provider.clone(),
                    message: "refresh returned error".to_owned(),
                });
            };
            let snap: MethodProviderSnapshot =
                serde_json::from_value(success.result).map_err(|source| {
                    MethodProviderRefreshError::Decode {
                        provider: provider.clone(),
                        message: source.to_string(),
                    }
                })?;
            if &snap.provider != &provider {
                return Err(MethodProviderRefreshError::SnapshotMismatch {
                    provider: provider.clone(),
                    expected: provider.clone(),
                    actual: snap.provider.clone(),
                });
            }
            let mut seen = HashSet::new();
            for m in &snap.methods {
                if !seen.insert(m.name.clone()) {
                    return Err(MethodProviderRefreshError::DuplicateMethod {
                        provider: provider.clone(),
                        method: m.name.clone(),
                    });
                }
            }
            {
                let mut guard = self.snapshot.write().expect("poisoned");
                *guard = snap.clone();
            }
            Ok(snap)
        })
    }
}
