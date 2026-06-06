use crate::{
    endpoint::JsonRpcEndpoint,
    error::{MethodCallError, MethodProviderBuildError, MethodProviderRefreshError},
    method::{
        MethodInfo, MethodName, MethodProvider, MethodProviderFuture, MethodProviderSnapshot,
        ProviderName,
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
    endpoint: Arc<JsonRpcEndpoint>,
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
        endpoint_catalog: &crate::endpoint::EndpointCatalog,
    ) -> Result<Self, MethodProviderBuildError> {
        let endpoint = endpoint_catalog
            .get(&config.endpoint)
            .ok_or_else(|| MethodProviderBuildError::InvalidConfig {
                provider: config.provider.clone(),
                message: format!("unknown endpoint '{}'", config.endpoint.as_str()),
            })?
            .clone();

        if matches!(
            config.discovery,
            JsonRpcMethodDiscoveryConfig::Watchable { .. }
        ) && !endpoint.session_capable()
        {
            return Err(MethodProviderBuildError::EndpointDoesNotSupportSession {
                endpoint: config.endpoint.clone(),
                provider: config.provider.clone(),
            });
        }

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
            snapshot: RwLock::new(initial_snapshot),
            discovery: config.discovery,
            next_id,
        })
    }

    async fn call_initialize(
        endpoint: &JsonRpcEndpoint,
        method: &str,
        provider: &ProviderName,
        next_id: &AtomicU64,
    ) -> Result<MethodProviderSnapshot, MethodProviderBuildError> {
        let req = JsonRpcRequest {
            jsonrpc: JsonRpcVersion::V2_0,
            id: json_rpc_id(next_id),
            method: method.to_owned(),
            params: None,
        };
        let resp = endpoint.request(req).await.map_err(|source| {
            MethodProviderBuildError::DiscoveryTransport {
                provider: provider.clone(),
                source,
            }
        })?;
        let JsonRpcResponse::Success(success) = resp else {
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

    fn next_id(&self) -> JsonRpcId {
        json_rpc_id(&self.next_id)
    }
}

impl MethodProvider for JsonRpcMethodProvider {
    fn name(&self) -> &ProviderName {
        &self.provider
    }

    fn endpoint(&self) -> Option<&crate::endpoint::EndpointName> {
        Some(&self.endpoint.name)
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
                id: self.next_id(),
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
            let JsonRpcMessage::Single(JsonRpcSingleMessage::Request(r)) = message else {
                return Err(MethodCallError::InvalidResponse {
                    provider: self.provider.clone(),
                    method: method.clone(),
                    message: "provider send_message expected a request message".to_owned(),
                });
            };
            let resp =
                self.endpoint
                    .request(r)
                    .await
                    .map_err(|source| MethodCallError::Transport {
                        provider: self.provider.clone(),
                        method: method.clone(),
                        source,
                    })?;
            Ok(JsonRpcMessage::Single(JsonRpcSingleMessage::Response(resp)))
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
            let req = JsonRpcRequest {
                jsonrpc: JsonRpcVersion::V2_0,
                id: self.next_id(),
                method: refresh_method,
                params: None,
            };
            let resp = endpoint.request(req).await.map_err(|source| {
                MethodProviderRefreshError::Transport {
                    provider: provider.clone(),
                    source,
                }
            })?;
            let JsonRpcResponse::Success(success) = resp else {
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
