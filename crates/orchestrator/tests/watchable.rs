use actrpc_core::{
    ACTRPC_METHOD_PROVIDER_CHANGED_METHOD, ACTRPC_METHOD_PROVIDER_INITIALIZE_METHOD,
    ACTRPC_METHOD_PROVIDER_REFRESH_METHOD,
    json_rpc::{
        JsonRpcId, JsonRpcNotification, JsonRpcParams, JsonRpcRequest, JsonRpcResponse,
        JsonRpcSuccessResponse, JsonRpcVersion,
    },
};
use actrpc_orchestrator::{
    endpoint::{EndpointCatalog, EndpointConfig, EndpointName},
    error::OrchestratorError,
    method::{
        JsonRpcMethodDiscoveryConfig, JsonRpcMethodSourceConfig, MethodCatalog, MethodInfo,
        MethodName, MethodProviderSnapshot, MethodSourceConfig, ProviderName,
        spawn_watchable_listeners,
    },
};
use actrpc_transport::{
    JsonRpcClient, JsonRpcClientFuture, JsonRpcClientProvider, JsonRpcClientProviderFuture,
    JsonRpcSession, JsonRpcSessionEvent, JsonRpcSessionFuture, JsonRpcSessionProvider,
    JsonRpcSessionProviderFuture, TransportError, TransportTarget, target::HttpTarget,
};
use serde_json::json;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

fn method_a() -> MethodInfo {
    MethodInfo {
        name: MethodName::from("method_a"),
        description: None,
        info: json!({}),
    }
}

fn method_b() -> MethodInfo {
    MethodInfo {
        name: MethodName::from("method_b"),
        description: None,
        info: json!({}),
    }
}

fn snapshot(provider: &str, methods: Vec<MethodInfo>) -> MethodProviderSnapshot {
    MethodProviderSnapshot {
        provider: ProviderName::from(provider),
        version: None,
        description: None,
        methods,
        info: json!({}),
    }
}

fn success_response(result: serde_json::Value) -> JsonRpcResponse {
    JsonRpcResponse::Success(JsonRpcSuccessResponse {
        jsonrpc: JsonRpcVersion::V2_0,
        id: JsonRpcId::Number(1.into()),
        result,
    })
}

struct FakeJsonRpcSession {
    responses: Mutex<HashMap<String, JsonRpcResponse>>,
    events: tokio::sync::broadcast::Sender<JsonRpcSessionEvent>,
}

impl FakeJsonRpcSession {
    fn new() -> Arc<Self> {
        let (events, _) = tokio::sync::broadcast::channel(64);
        Arc::new(Self {
            responses: Mutex::new(HashMap::new()),
            events,
        })
    }

    fn queue_response(self: &Arc<Self>, method: &str, response: JsonRpcResponse) {
        self.responses
            .lock()
            .unwrap()
            .insert(method.to_owned(), response);
    }

    fn inject_notification(self: &Arc<Self>, notification: JsonRpcNotification) {
        let _ = self
            .events
            .send(JsonRpcSessionEvent::Notification(notification));
    }
}

impl JsonRpcSession for FakeJsonRpcSession {
    type Error = TransportError;

    fn request<'a>(
        &'a self,
        request: JsonRpcRequest,
    ) -> JsonRpcSessionFuture<'a, Result<JsonRpcResponse, Self::Error>> {
        let responses = self.responses.lock().unwrap().clone();
        Box::pin(async move {
            responses
                .get(&request.method)
                .cloned()
                .ok_or_else(|| TransportError::Internal {
                    message: format!("no queued response for method '{}'", request.method),
                })
        })
    }

    fn notify<'a>(
        &'a self,
        _notification: actrpc_core::json_rpc::JsonRpcNotification,
    ) -> JsonRpcSessionFuture<'a, Result<(), Self::Error>> {
        Box::pin(async move { Ok(()) })
    }

    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<JsonRpcSessionEvent> {
        self.events.subscribe()
    }
}

struct FakeSessionProvider {
    session: Arc<FakeJsonRpcSession>,
}

impl FakeSessionProvider {
    fn new(session: Arc<FakeJsonRpcSession>) -> Self {
        Self { session }
    }
}

impl JsonRpcSessionProvider for FakeSessionProvider {
    type Error = TransportError;
    type Session = Arc<dyn JsonRpcSession<Error = TransportError>>;

    fn get_session<'a>(
        &'a self,
        _target: &'a TransportTarget,
    ) -> JsonRpcSessionProviderFuture<'a, Result<Self::Session, Self::Error>> {
        let session = self.session.clone() as Arc<dyn JsonRpcSession<Error = TransportError>>;
        Box::pin(async move { Ok(session) })
    }
}

struct QueuedClient {
    response: actrpc_core::json_rpc::JsonRpcMessage,
}

impl JsonRpcClient for QueuedClient {
    type Error = TransportError;

    fn send<'a>(
        &'a self,
        _message: actrpc_core::json_rpc::JsonRpcMessage,
    ) -> JsonRpcClientFuture<'a, Result<actrpc_core::json_rpc::JsonRpcMessage, Self::Error>> {
        let response = self.response.clone();
        Box::pin(async move { Ok(response) })
    }
}

struct QueuedClientProvider {
    response: actrpc_core::json_rpc::JsonRpcMessage,
}

impl QueuedClientProvider {
    fn new(response: actrpc_core::json_rpc::JsonRpcMessage) -> Self {
        Self { response }
    }
}

impl JsonRpcClientProvider for QueuedClientProvider {
    type Error = TransportError;
    type Client = Arc<dyn JsonRpcClient<Error = TransportError>>;

    fn get_client<'a>(
        &'a self,
        _target: &'a TransportTarget,
    ) -> JsonRpcClientProviderFuture<'a, Result<Self::Client, Self::Error>> {
        let response = self.response.clone();
        Box::pin(async move {
            Ok(Arc::new(QueuedClient { response })
                as Arc<dyn JsonRpcClient<Error = TransportError>>)
        })
    }
}

struct NoopClientProvider;

impl JsonRpcClientProvider for NoopClientProvider {
    type Error = TransportError;
    type Client = Arc<dyn JsonRpcClient<Error = TransportError>>;

    fn get_client<'a>(
        &'a self,
        _target: &'a TransportTarget,
    ) -> JsonRpcClientProviderFuture<'a, Result<Self::Client, Self::Error>> {
        Box::pin(async move {
            Ok(Arc::new(QueuedClient {
                response: actrpc_core::json_rpc::JsonRpcMessage::Single(
                    actrpc_core::json_rpc::JsonRpcSingleMessage::Response(success_response(json!(
                        null
                    ))),
                ),
            })
                as Arc<dyn JsonRpcClient<Error = TransportError>>)
        })
    }
}

fn tcp_target() -> TransportTarget {
    serde_json::from_value(json!({
        "tcp": {
            "addr": "127.0.0.1:0",
            "framing": "newline_delimited"
        }
    }))
    .unwrap()
}

fn http_target() -> TransportTarget {
    TransportTarget::Http(HttpTarget {
        url: "http://example.invalid/rpc".to_owned(),
        headers: vec![],
        timeout_ms: 1_000,
    })
}

fn watchable_method_config(provider: &str, endpoint: &str) -> MethodSourceConfig {
    MethodSourceConfig::JsonRpc(JsonRpcMethodSourceConfig {
        provider: ProviderName::from(provider),
        endpoint: EndpointName::from(endpoint),
        discovery: JsonRpcMethodDiscoveryConfig::Watchable {
            initialize_method: ACTRPC_METHOD_PROVIDER_INITIALIZE_METHOD.to_owned(),
            refresh_method: ACTRPC_METHOD_PROVIDER_REFRESH_METHOD.to_owned(),
        },
    })
}

async fn build_watchable_catalog(
    endpoint_name: &str,
    target: TransportTarget,
    provider_name: &str,
    init_methods: Vec<MethodInfo>,
    refresh_methods: Vec<MethodInfo>,
) -> (
    Arc<MethodCatalog>,
    EndpointCatalog,
    Arc<FakeJsonRpcSession>,
    EndpointName,
    ProviderName,
) {
    let session = FakeJsonRpcSession::new();
    session.queue_response(
        ACTRPC_METHOD_PROVIDER_INITIALIZE_METHOD,
        success_response(serde_json::to_value(snapshot(provider_name, init_methods)).unwrap()),
    );
    session.queue_response(
        ACTRPC_METHOD_PROVIDER_REFRESH_METHOD,
        success_response(serde_json::to_value(snapshot(provider_name, refresh_methods)).unwrap()),
    );

    let ep_name = EndpointName::from(endpoint_name);
    let methods = vec![watchable_method_config(provider_name, endpoint_name)];
    let endpoint_catalog = EndpointCatalog::from_configs(
        vec![EndpointConfig {
            name: ep_name.clone(),
            target,
        }],
        &methods,
        &[],
        &NoopClientProvider,
        &FakeSessionProvider::new(session.clone()),
    )
    .await
    .unwrap();

    let catalog = Arc::new(
        MethodCatalog::from_configs(methods, &endpoint_catalog)
            .await
            .unwrap(),
    );
    let provider = ProviderName::from(provider_name);
    (catalog, endpoint_catalog, session, ep_name, provider)
}

fn changed_notification(provider: &str, version: Option<&str>) -> JsonRpcNotification {
    let mut params = serde_json::Map::new();
    params.insert(
        "provider".to_owned(),
        serde_json::Value::String(provider.to_owned()),
    );
    if let Some(version) = version {
        params.insert(
            "version".to_owned(),
            serde_json::Value::String(version.to_owned()),
        );
    }
    JsonRpcNotification {
        jsonrpc: JsonRpcVersion::V2_0,
        method: ACTRPC_METHOD_PROVIDER_CHANGED_METHOD.to_owned(),
        params: Some(JsonRpcParams::Object(params)),
    }
}

#[tokio::test]
async fn watchable_on_http_endpoint_fails_at_build() {
    let methods = vec![watchable_method_config("agent_tools", "agent")];
    let result = EndpointCatalog::from_configs(
        vec![EndpointConfig {
            name: EndpointName::from("agent"),
            target: http_target(),
        }],
        &methods,
        &[],
        &NoopClientProvider,
        &FakeSessionProvider::new(FakeJsonRpcSession::new()),
    )
    .await;

    match result {
        Err(err) => match err {
            OrchestratorError::WatchableUnsupportedEndpoint { endpoint, message } => {
                assert_eq!(endpoint.as_str(), "agent");
                assert!(message.contains("HTTP"));
            }
            other => panic!("unexpected error: {other:?}"),
        },
        Ok(_) => panic!("expected build to fail"),
    }
}

#[tokio::test]
async fn endpoint_catalog_builds_session_for_watchable_endpoint() {
    let methods = vec![watchable_method_config("agent_tools", "agent")];
    let session = FakeJsonRpcSession::new();
    let catalog = EndpointCatalog::from_configs(
        vec![EndpointConfig {
            name: EndpointName::from("agent"),
            target: tcp_target(),
        }],
        &methods,
        &[],
        &NoopClientProvider,
        &FakeSessionProvider::new(session),
    )
    .await
    .unwrap();

    let endpoint = catalog.get(&EndpointName::from("agent")).unwrap();
    assert!(endpoint.session_capable());
}

#[tokio::test]
async fn valid_notification_triggers_refresh() {
    let (catalog, endpoint_catalog, session, _endpoint, provider) = build_watchable_catalog(
        "agent",
        tcp_target(),
        "agent_tools",
        vec![method_a()],
        vec![method_b()],
    )
    .await;

    assert_eq!(
        catalog
            .provider(&provider)
            .unwrap()
            .snapshot()
            .methods
            .iter()
            .map(|m| m.name.as_str())
            .collect::<Vec<_>>(),
        vec!["method_a"]
    );

    let handles = spawn_watchable_listeners(catalog.clone(), &endpoint_catalog).unwrap();
    assert_eq!(handles.len(), 1);

    session.inject_notification(changed_notification("agent_tools", Some("42")));

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let snap = catalog.provider(&provider).unwrap().snapshot();
    let names: Vec<_> = snap.methods.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, vec!["method_b"]);

    for handle in handles {
        handle.abort();
    }
}

#[tokio::test]
async fn wrong_endpoint_notification_is_rejected() {
    let (catalog, _endpoint_catalog, _session, endpoint_a, provider) = build_watchable_catalog(
        "endpoint_a",
        tcp_target(),
        "agent_tools",
        vec![method_a()],
        vec![method_b()],
    )
    .await;

    let err = catalog
        .handle_method_provider_changed(&EndpointName::from("endpoint_b"), &provider, None)
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        actrpc_orchestrator::error::MethodCatalogError::ProviderEndpointMismatch { .. }
    ));

    assert_eq!(
        catalog.provider(&provider).unwrap().snapshot().methods[0]
            .name
            .as_str(),
        "method_a"
    );
    let _ = endpoint_a;
}

#[tokio::test]
async fn non_watchable_provider_does_not_refresh_on_notification() {
    let session = FakeJsonRpcSession::new();
    session.queue_response(
        ACTRPC_METHOD_PROVIDER_INITIALIZE_METHOD,
        success_response(serde_json::to_value(snapshot("refreshable", vec![method_a()])).unwrap()),
    );

    let methods = vec![MethodSourceConfig::JsonRpc(JsonRpcMethodSourceConfig {
        provider: ProviderName::from("refreshable"),
        endpoint: EndpointName::from("agent"),
        discovery: JsonRpcMethodDiscoveryConfig::Refreshable {
            initialize_method: ACTRPC_METHOD_PROVIDER_INITIALIZE_METHOD.to_owned(),
            refresh_method: ACTRPC_METHOD_PROVIDER_REFRESH_METHOD.to_owned(),
        },
    })];

    let init_response = actrpc_core::json_rpc::JsonRpcMessage::Single(
        actrpc_core::json_rpc::JsonRpcSingleMessage::Response(success_response(
            serde_json::to_value(snapshot("refreshable", vec![method_a()])).unwrap(),
        )),
    );
    let endpoint_catalog = EndpointCatalog::from_configs(
        vec![EndpointConfig {
            name: EndpointName::from("agent"),
            target: tcp_target(),
        }],
        &methods,
        &[],
        &QueuedClientProvider::new(init_response),
        &FakeSessionProvider::new(session),
    )
    .await
    .unwrap();

    let catalog = MethodCatalog::from_configs(methods, &endpoint_catalog)
        .await
        .unwrap();
    let provider = ProviderName::from("refreshable");

    let err = catalog
        .handle_method_provider_changed(&EndpointName::from("agent"), &provider, None)
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        actrpc_orchestrator::error::MethodCatalogError::ProviderNotWatchable { .. }
    ));
    assert_eq!(
        catalog.provider(&provider).unwrap().snapshot().methods[0]
            .name
            .as_str(),
        "method_a"
    );
}

#[tokio::test]
async fn malformed_then_valid_notification_still_refreshes() {
    let (catalog, endpoint_catalog, session, _endpoint, provider) = build_watchable_catalog(
        "agent",
        tcp_target(),
        "agent_tools",
        vec![method_a()],
        vec![method_b()],
    )
    .await;

    let handles = spawn_watchable_listeners(catalog.clone(), &endpoint_catalog).unwrap();
    assert_eq!(handles.len(), 1);

    session.inject_notification(JsonRpcNotification {
        jsonrpc: JsonRpcVersion::V2_0,
        method: ACTRPC_METHOD_PROVIDER_CHANGED_METHOD.to_owned(),
        params: Some(JsonRpcParams::Array(vec![json!("not-an-object")])),
    });

    session.inject_notification(JsonRpcNotification {
        jsonrpc: JsonRpcVersion::V2_0,
        method: ACTRPC_METHOD_PROVIDER_CHANGED_METHOD.to_owned(),
        params: None,
    });

    session.inject_notification(changed_notification("agent_tools", None));

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    assert_eq!(
        catalog.provider(&provider).unwrap().snapshot().methods[0]
            .name
            .as_str(),
        "method_b"
    );

    for handle in handles {
        handle.abort();
    }
}
