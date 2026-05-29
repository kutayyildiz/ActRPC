use crate::{
    error::{MethodCallError, MethodProviderBuildError},
    method::{MethodInfo, MethodName, MethodProvider, MethodProviderFuture, ProviderName},
};
use actrpc_core::json_rpc::{
    JsonRpcId, JsonRpcMessage, JsonRpcParams, JsonRpcRequest, JsonRpcSingleMessage, JsonRpcVersion,
};
use actrpc_transport::{JsonRpcClient, JsonRpcClientProvider, TransportError, TransportTarget};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeMethodSourceConfig {
    pub name: ProviderName,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    pub target: TransportTarget,

    #[serde(default)]
    pub info: serde_json::Value,

    #[serde(default)]
    pub methods: Vec<NativeMethodConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeMethodConfig {
    pub name: MethodName,

    pub remote_method: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default)]
    pub info: serde_json::Value,
}

pub struct NativeMethodProvider {
    name: ProviderName,
    description: Option<String>,
    info: serde_json::Value,
    client: Arc<dyn JsonRpcClient<Error = TransportError>>,
    methods: Vec<MethodInfo>,
    remote_methods: HashMap<MethodName, String>,
    next_id: AtomicU64,
}

impl NativeMethodProvider {
    pub async fn from_config<P>(
        config: NativeMethodSourceConfig,
        client_provider: &P,
    ) -> Result<Self, MethodProviderBuildError>
    where
        P: JsonRpcClientProvider<
                Client = Arc<dyn JsonRpcClient<Error = TransportError>>,
                Error = TransportError,
            > + Send
            + Sync,
    {
        let client = client_provider
            .get_client(&config.target)
            .await
            .map_err(|source| MethodProviderBuildError::ClientCreate {
                provider: config.name.clone(),
                source,
            })?;

        let mut methods = Vec::with_capacity(config.methods.len());
        let mut remote_methods = HashMap::new();

        for method in config.methods {
            if remote_methods.contains_key(&method.name) {
                return Err(MethodProviderBuildError::DuplicateMethod {
                    provider: config.name.clone(),
                    method: method.name,
                });
            }

            methods.push(MethodInfo {
                name: method.name.clone(),
                description: method.description,
                info: method.info,
            });

            remote_methods.insert(method.name, method.remote_method);
        }

        Ok(Self {
            name: config.name,
            description: config.description,
            info: config.info,
            client,
            methods,
            remote_methods,
            next_id: AtomicU64::new(1),
        })
    }

    fn next_id(&self) -> JsonRpcId {
        JsonRpcId::Number(self.next_id.fetch_add(1, Ordering::Relaxed).into())
    }
}

impl MethodProvider for NativeMethodProvider {
    fn name(&self) -> &ProviderName {
        &self.name
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    fn info(&self) -> &serde_json::Value {
        &self.info
    }

    fn methods(&self) -> &[MethodInfo] {
        &self.methods
    }

    fn request_message(
        &self,
        method: &MethodName,
        params: Option<JsonRpcParams>,
    ) -> Result<JsonRpcMessage, MethodCallError> {
        let remote_method =
            self.remote_methods
                .get(method)
                .ok_or_else(|| MethodCallError::MethodNotFound {
                    provider: self.name.clone(),
                    method: method.clone(),
                })?;

        Ok(JsonRpcMessage::Single(JsonRpcSingleMessage::Request(
            JsonRpcRequest {
                jsonrpc: JsonRpcVersion::V2_0,
                id: self.next_id(),
                method: remote_method.clone(),
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
            self.client
                .send(message)
                .await
                .map_err(|source| MethodCallError::Transport {
                    provider: self.name.clone(),
                    method: method.clone(),
                    source,
                })
        })
    }
}
