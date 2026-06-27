use crate::{
    endpoint::{EndpointCatalog, RestHttpEndpoint},
    error::{MethodCallError, MethodProviderBuildError},
    method::{
        MethodInfo, MethodName, MethodProvider, MethodProviderFuture, MethodProviderSnapshot,
        ProviderName,
        rpc_bridge::{
            invalid_request_message_response, logical_error_response, request_internal_id,
        },
    },
};
use actrpc_core::json_rpc::{
    JsonRpcMessage, JsonRpcParams, JsonRpcRequest, JsonRpcResponse, JsonRpcSingleMessage,
    JsonRpcVersion,
};
use actrpc_transport::{
    HeaderPairs, RestHttpExecuteRequest, validate_http_method, validate_rest_path,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashSet,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestMethodSourceConfig {
    pub provider: ProviderName,
    pub endpoint: crate::endpoint::config::EndpointName,
    pub methods: Vec<RestMethodDefinition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestMethodDefinition {
    pub name: MethodName,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default)]
    pub params_schema: Option<Value>,

    #[serde(default)]
    pub result_schema: Option<Value>,

    pub request: RestRequestMapping,
    pub response: RestResponseMapping,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestRequestMapping {
    pub method: String,
    pub path: String,

    #[serde(default)]
    pub headers: HeaderPairs,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestResponseMapping {
    pub success_status: u16,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
}

pub struct RestMethodProvider {
    provider: ProviderName,
    endpoint: Arc<dyn RestHttpEndpoint>,
    endpoint_name: crate::endpoint::config::EndpointName,
    methods: Vec<MethodInfo>,
    definitions: std::collections::HashMap<MethodName, RestMethodDefinition>,
    next_id: AtomicU64,
}

impl RestMethodProvider {
    pub fn from_config(
        config: RestMethodSourceConfig,
        endpoint_catalog: &EndpointCatalog,
    ) -> Result<Self, MethodProviderBuildError> {
        let endpoint = endpoint_catalog
            .get_rest_http(&config.endpoint)
            .map_err(|source| MethodProviderBuildError::InvalidConfig {
                provider: config.provider.clone(),
                message: source.to_string(),
            })?;
        let endpoint_name = endpoint.endpoint_name().clone();

        let mut seen = HashSet::new();
        let mut methods = Vec::new();
        let mut definitions = std::collections::HashMap::new();

        for definition in config.methods {
            if !seen.insert(definition.name.clone()) {
                return Err(MethodProviderBuildError::DuplicateMethod {
                    provider: config.provider.clone(),
                    method: definition.name.clone(),
                });
            }

            validate_rest_method_definition(&config.provider, &definition)?;

            methods.push(MethodInfo {
                name: definition.name.clone(),
                description: definition.description.clone(),
                params_schema: definition.params_schema.clone(),
                result_schema: definition.result_schema.clone(),
                info: serde_json::Value::Null,
            });
            definitions.insert(definition.name.clone(), definition);
        }

        Ok(Self {
            provider: config.provider,
            endpoint,
            endpoint_name,
            methods,
            definitions,
            next_id: AtomicU64::new(1),
        })
    }

    fn build_http_body(
        &self,
        definition: &RestMethodDefinition,
        params: &Option<JsonRpcParams>,
    ) -> Result<Option<Vec<u8>>, MethodCallError> {
        let Some(body_template) = definition.request.body.as_deref() else {
            return Ok(None);
        };

        if body_template != "$params" {
            return Err(MethodCallError::InvalidParams {
                provider: self.provider.clone(),
                method: definition.name.clone(),
                message: format!("unsupported REST body template '{body_template}'"),
            });
        }

        let value = params
            .as_ref()
            .map(json_params_to_value)
            .unwrap_or(Value::Null);

        serde_json::to_vec(&value)
            .map_err(|source| MethodCallError::InvalidParams {
                provider: self.provider.clone(),
                method: definition.name.clone(),
                message: source.to_string(),
            })
            .map(Some)
    }

    fn map_http_response(
        &self,
        definition: &RestMethodDefinition,
        internal_id: actrpc_core::json_rpc::JsonRpcId,
        status: u16,
        body: &[u8],
    ) -> JsonRpcMessage {
        if status != definition.response.success_status {
            return logical_error_response(
                internal_id,
                format!("HTTP request failed with status {status}"),
            );
        }

        let result_template = definition.response.result.as_deref().unwrap_or("$body");

        if result_template != "$body" {
            return logical_error_response(
                internal_id,
                format!("unsupported REST result template '{result_template}'"),
            );
        }

        let result = if body.is_empty() {
            Value::Null
        } else {
            match serde_json::from_slice::<Value>(body) {
                Ok(value) => value,
                Err(source) => {
                    return logical_error_response(
                        internal_id,
                        format!("failed to parse REST response body as JSON: {source}"),
                    );
                }
            }
        };

        JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Success(
            actrpc_core::json_rpc::JsonRpcSuccessResponse {
                jsonrpc: JsonRpcVersion::V2_0,
                id: internal_id,
                result,
            },
        )))
    }
}

fn validate_rest_method_definition(
    provider: &ProviderName,
    definition: &RestMethodDefinition,
) -> Result<(), MethodProviderBuildError> {
    validate_http_method(&definition.request.method).map_err(|message| {
        MethodProviderBuildError::InvalidConfig {
            provider: provider.clone(),
            message,
        }
    })?;

    validate_rest_path(&definition.request.path).map_err(|message| {
        MethodProviderBuildError::InvalidConfig {
            provider: provider.clone(),
            message,
        }
    })?;

    if let Some(body) = definition.request.body.as_deref() {
        if body != "$params" {
            return Err(MethodProviderBuildError::InvalidConfig {
                provider: provider.clone(),
                message: format!("unsupported REST body template '{body}'"),
            });
        }
    }

    if let Some(result) = definition.response.result.as_deref() {
        if result != "$body" {
            return Err(MethodProviderBuildError::InvalidConfig {
                provider: provider.clone(),
                message: format!("unsupported REST result template '{result}'"),
            });
        }
    }

    let status = definition.response.success_status;
    if !(100..=599).contains(&status) {
        return Err(MethodProviderBuildError::InvalidConfig {
            provider: provider.clone(),
            message: format!("invalid HTTP success_status {status}"),
        });
    }

    Ok(())
}

fn json_params_to_value(params: &JsonRpcParams) -> Value {
    match params {
        JsonRpcParams::Array(values) => Value::Array(values.clone()),
        JsonRpcParams::Object(map) => Value::Object(map.clone()),
    }
}

impl MethodProvider for RestMethodProvider {
    fn name(&self) -> &ProviderName {
        &self.provider
    }

    fn endpoint(&self) -> Option<&crate::endpoint::EndpointName> {
        Some(&self.endpoint_name)
    }

    fn snapshot(&self) -> MethodProviderSnapshot {
        MethodProviderSnapshot {
            provider: self.provider.clone(),
            version: None,
            description: None,
            methods: self.methods.clone(),
            info: serde_json::Value::Null,
        }
    }

    fn refresh<'a>(
        &'a self,
    ) -> MethodProviderFuture<
        'a,
        Result<MethodProviderSnapshot, crate::error::MethodProviderRefreshError>,
    > {
        let provider = self.provider.clone();
        Box::pin(
            async move { Err(crate::error::MethodProviderRefreshError::Unsupported { provider }) },
        )
    }

    fn request_message(
        &self,
        method: &MethodName,
        params: Option<JsonRpcParams>,
    ) -> Result<JsonRpcMessage, MethodCallError> {
        if !self.definitions.contains_key(method) {
            return Err(MethodCallError::MethodNotFound {
                provider: self.provider.clone(),
                method: method.clone(),
            });
        }

        Ok(JsonRpcMessage::Single(JsonRpcSingleMessage::Request(
            JsonRpcRequest {
                jsonrpc: JsonRpcVersion::V2_0,
                id: actrpc_core::json_rpc::JsonRpcId::Number(
                    self.next_id.fetch_add(1, Ordering::Relaxed).into(),
                ),
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

            let Some(definition) = self.definitions.get(method) else {
                if let Some(id) = request_internal_id(&request.id) {
                    return Ok(logical_error_response(id, "unknown REST method"));
                }
                return Err(MethodCallError::MethodNotFound {
                    provider: self.provider.clone(),
                    method: method.clone(),
                });
            };

            let body = self.build_http_body(definition, &request.params)?;
            let http_request = RestHttpExecuteRequest {
                method: definition.request.method.clone(),
                path: definition.request.path.clone(),
                headers: definition.request.headers.clone(),
                body,
            };

            let response = self
                .endpoint
                .execute(http_request)
                .await
                .map_err(|source| MethodCallError::Transport {
                    provider: self.provider.clone(),
                    method: method.clone(),
                    source,
                })?;

            Ok(self.map_http_response(definition, internal_id, response.status, &response.body))
        })
    }
}
