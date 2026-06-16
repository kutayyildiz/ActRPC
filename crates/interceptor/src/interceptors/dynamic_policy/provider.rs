use crate::interceptors::dynamic_policy::{
    error::DynamicPolicyError,
    scope::{
        CreateScopeParams, GetScopeParams, ListScopesParams, PROVIDER_NAME, ReleaseScopeParams,
    },
    store::DynamicPolicyStore,
};
use actrpc_core::json_rpc::{
    JsonRpcError, JsonRpcMessage, JsonRpcParams, JsonRpcRequest, JsonRpcResponse,
    JsonRpcSingleMessage, JsonRpcVersion,
};
use actrpc_orchestrator::{
    error::MethodCallError,
    method::{
        MethodInfo, MethodName, MethodProvider, MethodProviderFuture, MethodProviderSnapshot,
        ProviderName,
    },
};
use serde_json::Value;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

const METHOD_CREATE_SCOPE: &str = "create_scope";
const METHOD_RELEASE_SCOPE: &str = "release_scope";
const METHOD_GET_SCOPE: &str = "get_scope";
const METHOD_LIST_SCOPES: &str = "list_scopes";

pub struct DynamicPolicyMethodProvider {
    store: Arc<DynamicPolicyStore>,
    next_id: AtomicU64,
}

impl DynamicPolicyMethodProvider {
    pub fn new(store: Arc<DynamicPolicyStore>) -> Self {
        Self {
            store,
            next_id: AtomicU64::new(1),
        }
    }

    pub fn handle_request(
        &self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, DynamicPolicyError> {
        match method {
            METHOD_CREATE_SCOPE => {
                let params: CreateScopeParams = decode_params(params)?;
                Ok(serde_json::to_value(self.store.create_scope(params)?)?)
            }
            METHOD_RELEASE_SCOPE => {
                let params: ReleaseScopeParams = decode_params(params)?;
                Ok(serde_json::to_value(self.store.release_scope(params)?)?)
            }
            METHOD_GET_SCOPE => {
                let params: GetScopeParams = decode_params(params)?;
                Ok(serde_json::to_value(self.store.get_scope(params)?)?)
            }
            METHOD_LIST_SCOPES => {
                let params = match params {
                    None | Some(Value::Null) => ListScopesParams::default(),
                    Some(value) => serde_json::from_value(value)
                        .map_err(|source| DynamicPolicyError::invalid_params(source.to_string()))?,
                };
                Ok(serde_json::to_value(self.store.list_scopes(params)?)?)
            }
            other => Err(DynamicPolicyError::MethodNotFound {
                method: other.to_owned(),
            }),
        }
    }

    fn next_json_rpc_id(&self) -> actrpc_core::json_rpc::JsonRpcId {
        actrpc_core::json_rpc::JsonRpcId::Number(
            self.next_id.fetch_add(1, Ordering::Relaxed).into(),
        )
    }
}

fn decode_params<T: serde::de::DeserializeOwned>(
    params: Option<Value>,
) -> Result<T, DynamicPolicyError> {
    let Some(value) = params else {
        return Err(DynamicPolicyError::invalid_params("missing params"));
    };

    serde_json::from_value(value)
        .map_err(|source| DynamicPolicyError::invalid_params(source.to_string()))
}

fn method_call_error(
    provider: ProviderName,
    method: MethodName,
    error: DynamicPolicyError,
) -> MethodCallError {
    match &error {
        DynamicPolicyError::InvalidParams { .. } | DynamicPolicyError::InvalidGlob { .. } => {
            MethodCallError::InvalidParams {
                provider,
                method,
                message: error.to_string(),
            }
        }
        DynamicPolicyError::MethodNotFound { .. } => {
            MethodCallError::MethodNotFound { provider, method }
        }
        _ => MethodCallError::RemoteError {
            provider,
            method,
            error: JsonRpcError {
                code: error.json_rpc_code(),
                message: error.to_string(),
                data: None,
            },
        },
    }
}

pub fn method_snapshot() -> MethodProviderSnapshot {
    MethodProviderSnapshot {
        provider: ProviderName::from(PROVIDER_NAME),
        version: None,
        description: Some("Runtime dynamic policy scope management".to_owned()),
        methods: vec![
            MethodInfo {
                name: MethodName::from(METHOD_CREATE_SCOPE),
                description: Some("Create a dynamic policy scope".to_owned()),
                info: serde_json::Value::Null,
            },
            MethodInfo {
                name: MethodName::from(METHOD_RELEASE_SCOPE),
                description: Some("Release a dynamic policy scope".to_owned()),
                info: serde_json::Value::Null,
            },
            MethodInfo {
                name: MethodName::from(METHOD_GET_SCOPE),
                description: Some("Get a dynamic policy scope".to_owned()),
                info: serde_json::Value::Null,
            },
            MethodInfo {
                name: MethodName::from(METHOD_LIST_SCOPES),
                description: Some("List dynamic policy scopes".to_owned()),
                info: serde_json::Value::Null,
            },
        ],
        info: serde_json::Value::Null,
    }
}

impl MethodProvider for DynamicPolicyMethodProvider {
    fn name(&self) -> &ProviderName {
        static NAME: std::sync::OnceLock<ProviderName> = std::sync::OnceLock::new();
        NAME.get_or_init(|| ProviderName::from(PROVIDER_NAME))
    }

    fn snapshot(&self) -> MethodProviderSnapshot {
        method_snapshot()
    }

    fn request_message(
        &self,
        method: &MethodName,
        params: Option<JsonRpcParams>,
    ) -> Result<JsonRpcMessage, MethodCallError> {
        let snap = method_snapshot();
        if !snap.methods.iter().any(|m| &m.name == method) {
            return Err(MethodCallError::MethodNotFound {
                provider: self.name().clone(),
                method: method.clone(),
            });
        }

        Ok(JsonRpcMessage::Single(JsonRpcSingleMessage::Request(
            JsonRpcRequest {
                jsonrpc: JsonRpcVersion::V2_0,
                id: self.next_json_rpc_id(),
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
            let JsonRpcMessage::Single(JsonRpcSingleMessage::Request(request)) = message else {
                return Err(MethodCallError::InvalidResponse {
                    provider: self.name().clone(),
                    method: method.clone(),
                    message: "expected JSON-RPC request".to_owned(),
                });
            };

            let params = request
                .params
                .map(|p| serde_json::to_value(p))
                .transpose()
                .map_err(|source| MethodCallError::InvalidParams {
                    provider: self.name().clone(),
                    method: method.clone(),
                    message: source.to_string(),
                })?;

            let result = self
                .handle_request(&request.method, params)
                .map_err(|error| method_call_error(self.name().clone(), method.clone(), error))?;

            Ok(JsonRpcMessage::Single(JsonRpcSingleMessage::Response(
                JsonRpcResponse::Success(actrpc_core::json_rpc::JsonRpcSuccessResponse {
                    jsonrpc: JsonRpcVersion::V2_0,
                    id: request.id,
                    result,
                }),
            )))
        })
    }

    fn refresh<'a>(
        &'a self,
    ) -> MethodProviderFuture<
        'a,
        Result<MethodProviderSnapshot, actrpc_orchestrator::error::MethodProviderRefreshError>,
    > {
        let provider = self.name().clone();
        Box::pin(async move {
            Err(actrpc_orchestrator::error::MethodProviderRefreshError::Unsupported { provider })
        })
    }
}

/// In-process async helper used by tests when calling provider methods directly.
pub async fn call_method(
    provider: &DynamicPolicyMethodProvider,
    method: &str,
    params: Option<Value>,
) -> Result<Value, DynamicPolicyError> {
    provider.handle_request(method, params)
}
