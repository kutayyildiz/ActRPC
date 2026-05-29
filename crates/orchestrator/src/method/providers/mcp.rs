use crate::{
    error::{MethodCallError, MethodProviderBuildError},
    method::{MethodInfo, MethodName, MethodProvider, MethodProviderFuture, ProviderName},
};
use actrpc_core::json_rpc::{
    JsonRpcId, JsonRpcMessage, JsonRpcParams, JsonRpcRequest, JsonRpcResponse,
    JsonRpcSingleMessage, JsonRpcVersion,
};
use actrpc_transport::{JsonRpcClient, JsonRpcClientProvider, TransportError, TransportTarget};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    collections::HashSet,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpMethodSourceConfig {
    pub name: ProviderName,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    pub target: TransportTarget,

    #[serde(default)]
    pub info: serde_json::Value,

    #[serde(default)]
    pub include_tools: Vec<String>,

    #[serde(default)]
    pub exclude_tools: Vec<String>,
}

pub struct McpMethodProvider {
    name: ProviderName,
    description: Option<String>,
    info: serde_json::Value,
    client: Arc<dyn JsonRpcClient<Error = TransportError>>,
    methods: Vec<MethodInfo>,
    tool_names: HashSet<MethodName>,
    next_id: AtomicU64,
}

impl McpMethodProvider {
    pub async fn from_config<P>(
        config: McpMethodSourceConfig,
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

        let tools_list = list_tools(config.name.clone(), client.clone()).await?;

        let include_tools: HashSet<String> = config.include_tools.into_iter().collect();
        let exclude_tools: HashSet<String> = config.exclude_tools.into_iter().collect();

        let tools = tools_list
            .get("tools")
            .and_then(Value::as_array)
            .ok_or_else(|| MethodProviderBuildError::DiscoveryFailed {
                provider: config.name.clone(),
                message: "MCP tools/list result did not contain a tools array".to_owned(),
            })?;

        let mut methods = Vec::new();
        let mut tool_names = HashSet::new();

        for tool in tools {
            let Some(tool_name) = tool.get("name").and_then(Value::as_str) else {
                continue;
            };

            if !include_tools.is_empty() && !include_tools.contains(tool_name) {
                continue;
            }

            if exclude_tools.contains(tool_name) {
                continue;
            }

            let method_name = MethodName::new(tool_name);

            if !tool_names.insert(method_name.clone()) {
                return Err(MethodProviderBuildError::DuplicateMethod {
                    provider: config.name.clone(),
                    method: method_name,
                });
            }

            let description = tool
                .get("description")
                .and_then(Value::as_str)
                .map(str::to_owned);

            methods.push(MethodInfo {
                name: method_name,
                description,
                info: tool.clone(),
            });
        }

        let mut provider_info = Map::new();
        provider_info.insert("kind".to_owned(), Value::String("mcp".to_owned()));
        provider_info.insert("tools_list".to_owned(), tools_list);

        if let Value::Object(extra) = config.info {
            for (key, value) in extra {
                provider_info.insert(key, value);
            }
        }

        Ok(Self {
            name: config.name,
            description: config.description,
            info: Value::Object(provider_info),
            client,
            methods,
            tool_names,
            next_id: AtomicU64::new(1),
        })
    }

    fn next_id(&self) -> JsonRpcId {
        JsonRpcId::Number(self.next_id.fetch_add(1, Ordering::Relaxed).into())
    }
}

impl MethodProvider for McpMethodProvider {
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
        if !self.tool_names.contains(method) {
            return Err(MethodCallError::MethodNotFound {
                provider: self.name.clone(),
                method: method.clone(),
            });
        }

        let arguments = match params {
            Some(JsonRpcParams::Object(map)) => Value::Object(map),
            Some(JsonRpcParams::Array(_)) => {
                return Err(MethodCallError::InvalidParams {
                    provider: self.name.clone(),
                    method: method.clone(),
                    message: "MCP tool arguments must be JSON object params".to_owned(),
                });
            }
            None => Value::Object(Map::new()),
        };

        let mut call_params = Map::new();
        call_params.insert("name".to_owned(), Value::String(method.as_str().to_owned()));
        call_params.insert("arguments".to_owned(), arguments);

        Ok(JsonRpcMessage::Single(JsonRpcSingleMessage::Request(
            JsonRpcRequest {
                jsonrpc: JsonRpcVersion::V2_0,
                id: self.next_id(),
                method: "tools/call".to_owned(),
                params: Some(JsonRpcParams::Object(call_params)),
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

async fn list_tools(
    provider: ProviderName,
    client: Arc<dyn JsonRpcClient<Error = TransportError>>,
) -> Result<Value, MethodProviderBuildError> {
    let id = JsonRpcId::Number(1_u64.into());

    let request = JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
        jsonrpc: JsonRpcVersion::V2_0,
        id: id.clone(),
        method: "tools/list".to_owned(),
        params: None,
    }));

    let response = client.send(request).await.map_err(|source| {
        MethodProviderBuildError::DiscoveryTransport {
            provider: provider.clone(),
            source,
        }
    })?;

    let JsonRpcMessage::Single(JsonRpcSingleMessage::Response(response)) = response else {
        return Err(MethodProviderBuildError::DiscoveryFailed {
            provider,
            message: "MCP tools/list returned a non-response JSON-RPC message".to_owned(),
        });
    };

    match response {
        JsonRpcResponse::Success(success) => {
            if success.id != id {
                return Err(MethodProviderBuildError::DiscoveryFailed {
                    provider,
                    message: "MCP tools/list response id mismatch".to_owned(),
                });
            }

            Ok(success.result)
        }

        JsonRpcResponse::Error(error) => Err(MethodProviderBuildError::DiscoveryFailed {
            provider,
            message: format!(
                "MCP tools/list returned JSON-RPC error {}: {}",
                error.error.code, error.error.message
            ),
        }),
    }
}
