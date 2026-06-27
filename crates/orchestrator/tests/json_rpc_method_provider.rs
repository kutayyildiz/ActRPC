use actrpc_core::json_rpc::{
    JsonRpcId, JsonRpcMessage, JsonRpcParams, JsonRpcRequest, JsonRpcResponse,
    JsonRpcSingleMessage, JsonRpcSuccessResponse, JsonRpcVersion,
};
use actrpc_orchestrator::{
    EndpointCatalog, EndpointConfig, EndpointName,
    method::{
        JsonRpcMethodDiscoveryConfig, JsonRpcMethodProvider, JsonRpcMethodSourceConfig, MethodInfo,
        MethodName, MethodProvider, ProviderName,
    },
};
use actrpc_transport::{
    HeaderPairs, JsonRpcClient, JsonRpcClientFuture, JsonRpcClientProvider,
    JsonRpcClientProviderFuture, JsonRpcSessionProvider, JsonRpcSessionProviderFuture,
    TransportError, TransportTarget, target::HttpTarget,
};
use serde_json::json;
use std::sync::{Arc, Mutex};

struct RecordingJsonRpcClient {
    last_request_id: Arc<Mutex<Option<JsonRpcId>>>,
    last_request_method: Arc<Mutex<Option<String>>>,
    mismatch_id: bool,
}

impl JsonRpcClient for RecordingJsonRpcClient {
    type Error = TransportError;

    fn send<'a>(
        &'a self,
        message: JsonRpcMessage,
    ) -> JsonRpcClientFuture<'a, Result<JsonRpcMessage, Self::Error>> {
        let mismatch = self.mismatch_id;
        let last_request_id = Arc::clone(&self.last_request_id);
        let last_request_method = Arc::clone(&self.last_request_method);
        Box::pin(async move {
            let JsonRpcMessage::Single(JsonRpcSingleMessage::Request(request)) = message else {
                return Err(TransportError::Internal {
                    message: "expected request".to_owned(),
                });
            };
            *last_request_id.lock().unwrap() = Some(request.id.clone());
            *last_request_method.lock().unwrap() = Some(request.method.clone());

            let response_id = if mismatch {
                JsonRpcId::Number(999.into())
            } else {
                request.id
            };

            Ok(JsonRpcMessage::Single(JsonRpcSingleMessage::Response(
                JsonRpcResponse::Success(JsonRpcSuccessResponse {
                    jsonrpc: JsonRpcVersion::V2_0,
                    id: response_id,
                    result: json!(3),
                }),
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

async fn provider_with_response(
    mismatch_id: bool,
) -> (Arc<dyn MethodProvider>, Arc<RecordingJsonRpcClient>) {
    let client = Arc::new(RecordingJsonRpcClient {
        last_request_id: Arc::new(Mutex::new(None)),
        last_request_method: Arc::new(Mutex::new(None)),
        mismatch_id,
    });

    let endpoint_catalog = EndpointCatalog::from_configs(
        vec![EndpointConfig::legacy(
            EndpointName::from("ep"),
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
        JsonRpcMethodProvider::from_config(
            JsonRpcMethodSourceConfig {
                provider: ProviderName::from("math"),
                endpoint: EndpointName::from("ep"),
                discovery: JsonRpcMethodDiscoveryConfig::Static {
                    methods: vec![MethodInfo {
                        name: MethodName::from("sum"),
                        description: None,
                        params_schema: None,
                        result_schema: None,
                        info: json!({}),
                    }],
                },
            },
            &endpoint_catalog,
        )
        .await
        .unwrap(),
    ) as Arc<dyn MethodProvider>;

    (provider, client)
}

#[tokio::test]
async fn preserves_internal_id_and_verifies_external_id() {
    let (provider, client) = provider_with_response(false).await;

    let internal_id = JsonRpcId::String("internal-42".to_owned());
    let request = JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
        jsonrpc: JsonRpcVersion::V2_0,
        id: internal_id.clone(),
        method: "sum".to_owned(),
        params: Some(JsonRpcParams::Array(vec![json!(1), json!(2)])),
    }));

    let response = provider
        .send_message(&MethodName::from("sum"), request)
        .await
        .unwrap();

    let JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Success(success))) =
        response
    else {
        panic!("expected success response");
    };

    assert_eq!(success.id, internal_id);
    assert_eq!(success.result, json!(3));

    let external_id = client
        .last_request_id
        .lock()
        .unwrap()
        .clone()
        .expect("sent");
    assert_ne!(external_id, internal_id);
}

#[tokio::test]
async fn mismatched_external_response_id_returns_logical_error() {
    let (provider, _) = provider_with_response(true).await;

    let internal_id = JsonRpcId::String("internal-7".to_owned());
    let request = JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
        jsonrpc: JsonRpcVersion::V2_0,
        id: internal_id.clone(),
        method: "sum".to_owned(),
        params: None,
    }));

    let response = provider
        .send_message(&MethodName::from("sum"), request)
        .await
        .unwrap();

    let JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Error(error))) =
        response
    else {
        panic!("expected logical error response");
    };

    assert_eq!(error.id, internal_id);
    assert!(error.error.message.contains("id mismatch"));
}

#[tokio::test]
async fn send_message_uses_selected_method_not_logical_request_method() {
    let (provider, client) = provider_with_response(false).await;

    let request = JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
        jsonrpc: JsonRpcVersion::V2_0,
        id: JsonRpcId::String("internal-1".to_owned()),
        method: "evil.method".to_owned(),
        params: Some(JsonRpcParams::Array(vec![json!(1), json!(2)])),
    }));

    provider
        .send_message(&MethodName::from("sum"), request)
        .await
        .unwrap();

    let external_method = client
        .last_request_method
        .lock()
        .unwrap()
        .clone()
        .expect("sent");
    assert_eq!(external_method, "sum");
}

#[tokio::test]
async fn rejects_batch_messages_with_provider_error() {
    let (provider, _) = provider_with_response(false).await;

    let err = provider
        .send_message(
            &MethodName::from("sum"),
            JsonRpcMessage::Batch(actrpc_core::json_rpc::JsonRpcBatch(vec![])),
        )
        .await
        .unwrap_err();

    assert!(
        err.to_string()
            .contains("expected a single JSON-RPC request")
    );
}
