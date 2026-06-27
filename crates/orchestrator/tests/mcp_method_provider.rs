use actrpc_core::json_rpc::{
    JsonRpcId, JsonRpcMessage, JsonRpcParams, JsonRpcRequest, JsonRpcResponse,
    JsonRpcSingleMessage, JsonRpcSuccessResponse, JsonRpcVersion,
};
use actrpc_orchestrator::{
    EndpointCatalog, EndpointConfig, EndpointName,
    method::{McpMethodProvider, McpMethodSourceConfig, MethodName, MethodProvider, ProviderName},
};
use actrpc_transport::{
    HeaderPairs, JsonRpcClient, JsonRpcClientFuture, JsonRpcClientProvider,
    JsonRpcClientProviderFuture, JsonRpcSessionProvider, JsonRpcSessionProviderFuture,
    TransportError, TransportTarget, target::HttpTarget,
};
use serde_json::{Map, Value, json};
use std::sync::{Arc, Mutex};

struct RecordingJsonRpcClient {
    last_request: Arc<Mutex<Option<JsonRpcRequest>>>,
}

impl JsonRpcClient for RecordingJsonRpcClient {
    type Error = TransportError;

    fn send<'a>(
        &'a self,
        message: JsonRpcMessage,
    ) -> JsonRpcClientFuture<'a, Result<JsonRpcMessage, Self::Error>> {
        let last_request = Arc::clone(&self.last_request);
        Box::pin(async move {
            let JsonRpcMessage::Single(JsonRpcSingleMessage::Request(request)) = message else {
                return Err(TransportError::Internal {
                    message: "expected request".to_owned(),
                });
            };

            let response = if request.method == "tools/list" {
                JsonRpcResponse::Success(JsonRpcSuccessResponse {
                    jsonrpc: JsonRpcVersion::V2_0,
                    id: request.id.clone(),
                    result: json!({
                        "tools": [
                            {
                                "name": "tool_a",
                                "description": "tool a",
                                "inputSchema": {"type": "object"}
                            }
                        ]
                    }),
                })
            } else {
                *last_request.lock().unwrap() = Some(request.clone());
                JsonRpcResponse::Success(JsonRpcSuccessResponse {
                    jsonrpc: JsonRpcVersion::V2_0,
                    id: request.id,
                    result: json!({"ok": true}),
                })
            };

            Ok(JsonRpcMessage::Single(JsonRpcSingleMessage::Response(
                response,
            )))
        })
    }
}

struct StaticClientProvider {
    client: Arc<dyn JsonRpcClient<Error = TransportError>>,
}

impl JsonRpcClientProvider for StaticClientProvider {
    type Error = TransportError;
    type Client = Arc<dyn JsonRpcClient<Error = TransportError>>;

    fn get_client<'a>(
        &'a self,
        _target: &'a TransportTarget,
    ) -> JsonRpcClientProviderFuture<'a, Result<Self::Client, Self::Error>> {
        let client = self.client.clone();
        Box::pin(async move { Ok(client) })
    }
}

struct NoopSessionProvider;

impl JsonRpcSessionProvider for NoopSessionProvider {
    type Error = TransportError;
    type Session = Arc<dyn actrpc_transport::JsonRpcSession<Error = TransportError>>;

    fn get_session<'a>(
        &'a self,
        _target: &'a TransportTarget,
    ) -> JsonRpcSessionProviderFuture<'a, Result<Self::Session, Self::Error>> {
        Box::pin(async move {
            Err(TransportError::Internal {
                message: "session not used".to_owned(),
            })
        })
    }
}

async fn provider_with_recording() -> (Arc<dyn MethodProvider>, Arc<RecordingJsonRpcClient>) {
    let client = Arc::new(RecordingJsonRpcClient {
        last_request: Arc::new(Mutex::new(None)),
    });

    let endpoint_catalog = EndpointCatalog::from_configs(
        vec![EndpointConfig::legacy(
            EndpointName::from("mcp"),
            TransportTarget::Http(HttpTarget {
                url: "http://example.invalid".to_owned(),
                headers: HeaderPairs::default(),
                timeout_ms: 1000,
            }),
        )],
        &[],
        &[],
        &StaticClientProvider {
            client: client.clone() as Arc<dyn JsonRpcClient<Error = TransportError>>,
        },
        &NoopSessionProvider,
    )
    .await
    .unwrap();

    let provider = Arc::new(
        McpMethodProvider::from_config(
            McpMethodSourceConfig {
                name: ProviderName::from("mcp_tools"),
                description: None,
                endpoint: EndpointName::from("mcp"),
                info: json!({"custom": "value"}),
                include_tools: vec![],
                exclude_tools: vec![],
            },
            &endpoint_catalog,
        )
        .await
        .unwrap(),
    ) as Arc<dyn MethodProvider>;

    (provider, client)
}

#[tokio::test]
async fn send_message_uses_selected_tool_and_tools_call_method() {
    let (provider, client) = provider_with_recording().await;

    let mut call_params = Map::new();
    call_params.insert("name".to_owned(), Value::String("evil.method".to_owned()));
    call_params.insert("arguments".to_owned(), json!({"x": 1}));

    let request = JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
        jsonrpc: JsonRpcVersion::V2_0,
        id: JsonRpcId::String("internal-1".to_owned()),
        method: "evil.method".to_owned(),
        params: Some(JsonRpcParams::Object(call_params)),
    }));

    provider
        .send_message(&MethodName::from("tool_a"), request)
        .await
        .unwrap();

    let external = client
        .last_request
        .lock()
        .unwrap()
        .clone()
        .expect("tools/call sent");
    assert_eq!(external.method, "tools/call");

    let params = external.params.expect("params");
    let JsonRpcParams::Object(map) = params else {
        panic!("expected object params");
    };
    assert_eq!(map.get("name").and_then(Value::as_str), Some("tool_a"));
    assert_eq!(map.get("arguments"), Some(&json!({"x": 1})));
}

#[tokio::test]
async fn snapshot_info_contains_kind_and_tools_list() {
    let (provider, _) = provider_with_recording().await;

    let snapshot = provider.snapshot();
    assert_eq!(
        snapshot.info.get("kind").and_then(Value::as_str),
        Some("mcp")
    );
    assert!(snapshot.info.get("tools_list").is_some());
    assert_eq!(
        snapshot.info.get("custom").and_then(Value::as_str),
        Some("value")
    );
}
