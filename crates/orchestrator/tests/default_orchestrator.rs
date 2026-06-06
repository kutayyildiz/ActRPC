use actrpc_core::{
    InterceptorInitialization,
    action::{ActionKind, ActionSpec, RequestedActionRecord},
    interception::{InterceptionRequest, InterceptionResponse, InterceptorContinuation},
    json_rpc::{
        JsonRpcId, JsonRpcMessage, JsonRpcParams, JsonRpcRequest, JsonRpcResponse,
        JsonRpcSingleMessage, JsonRpcSuccessResponse, JsonRpcVersion,
    },
};
use actrpc_orchestrator::{
    action::actions::{modify_params::ModifyParams, modify_result::ModifyResult},
    error::{ActionError, InterceptorRuntimeError, OrchestratorError},
    interceptor::{
        ImmutableInterceptorPipeline, Interceptor, InterceptorCatalog, InterceptorCatalogEntry,
        InterceptorFuture, InterceptorPolicy,
    },
    method::{MethodCatalog, MethodInfo, MethodName, MethodSourceConfig, ProviderName},
    runtime::{CallExecutionFactory, DefaultOrchestrator, OrchestratorResources},
};
use actrpc_transport::{
    JsonRpcClient, JsonRpcClientFuture, JsonRpcClientProvider, JsonRpcClientProviderFuture,
    JsonRpcSession, JsonRpcSessionProvider, JsonRpcSessionProviderFuture, TransportError,
    TransportTarget, target::HttpTarget,
};
use serde_json::json;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, Mutex},
};

const TEST_PROVIDER: &str = "test";

#[tokio::test]
async fn default_orchestrator_forwards_without_interceptors() {
    let client = Arc::new(RecordingClient::new(response_message(json!("downstream"))));

    let orchestrator = test_orchestrator(
        InterceptorCatalog::new(
            HashMap::new(),
            ImmutableInterceptorPipeline::new(vec![]),
            ImmutableInterceptorPipeline::new(vec![]),
        ),
        client.clone(),
    )
    .await;

    let result = orchestrator
        .call(
            ProviderName::from(TEST_PROVIDER),
            MethodName::from("sum"),
            Some(JsonRpcParams::Array(vec![json!(1), json!(2)])),
        )
        .await
        .unwrap();

    assert_eq!(result, response_message(json!("downstream")));

    let sent = client.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(
        sent[0],
        request_message("sum", Some(JsonRpcParams::Array(vec![json!(1), json!(2)])))
    );
}

#[tokio::test]
async fn outbound_action_mutates_message_before_downstream_send() {
    let interceptor = Arc::new(QueuedInterceptor::new(vec![InterceptionResponse {
        continuation: InterceptorContinuation::Stop,
        actions: vec![RequestedActionRecord {
            kind: ModifyParams::action_kind(),
            params: Some(json!({
                "params": [10, 20]
            })),
        }],
    }]));

    let catalog = single_interceptor_catalog(
        "outbound_mutator",
        interceptor.clone(),
        InterceptorPolicy {
            outbound: HashSet::from([ModifyParams::action_kind()]),
            inbound: HashSet::new(),
        },
        vec!["outbound_mutator"],
        vec![],
    );

    let client = Arc::new(RecordingClient::new(response_message(json!("ok"))));

    let orchestrator = test_orchestrator(catalog, client.clone()).await;

    let result = orchestrator
        .call(
            ProviderName::from(TEST_PROVIDER),
            MethodName::from("sum"),
            Some(JsonRpcParams::Array(vec![json!(1), json!(2)])),
        )
        .await
        .unwrap();

    assert_eq!(result, response_message(json!("ok")));

    let sent = client.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(
        sent[0],
        request_message(
            "sum",
            Some(JsonRpcParams::Array(vec![json!(10), json!(20)]))
        )
    );

    let seen = interceptor.seen();
    assert_eq!(seen.len(), 1);
    assert!(seen[0].resolved_action_history.is_empty());
}

#[tokio::test]
async fn inbound_action_mutates_final_response() {
    let interceptor = Arc::new(QueuedInterceptor::new(vec![InterceptionResponse {
        continuation: InterceptorContinuation::Stop,
        actions: vec![RequestedActionRecord {
            kind: ModifyResult::action_kind(),
            params: Some(json!({
                "result": "rewritten"
            })),
        }],
    }]));

    let catalog = single_interceptor_catalog(
        "inbound_mutator",
        interceptor.clone(),
        InterceptorPolicy {
            outbound: HashSet::new(),
            inbound: HashSet::from([ModifyResult::action_kind()]),
        },
        vec![],
        vec!["inbound_mutator"],
    );

    let client = Arc::new(RecordingClient::new(response_message(json!("original"))));

    let orchestrator = test_orchestrator(catalog, client).await;

    let result = orchestrator
        .call(
            ProviderName::from(TEST_PROVIDER),
            MethodName::from("get"),
            None,
        )
        .await
        .unwrap();

    assert_eq!(result, response_message(json!("rewritten")));

    let seen = interceptor.seen();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].message, response_message(json!("original")));
    assert!(seen[0].resolved_action_history.is_empty());
}

#[tokio::test]
async fn default_orchestrator_rejects_action_forbidden_by_policy() {
    let interceptor = Arc::new(QueuedInterceptor::new(vec![InterceptionResponse {
        continuation: InterceptorContinuation::Stop,
        actions: vec![RequestedActionRecord {
            kind: ModifyParams::action_kind(),
            params: Some(json!({
                "params": [99]
            })),
        }],
    }]));

    let catalog = single_interceptor_catalog(
        "policy_test",
        interceptor,
        InterceptorPolicy {
            outbound: HashSet::new(),
            inbound: HashSet::new(),
        },
        vec!["policy_test"],
        vec![],
    );

    let client = Arc::new(RecordingClient::new(response_message(json!(
        "should_not_send"
    ))));

    let orchestrator = test_orchestrator(catalog, client.clone()).await;

    let err = orchestrator
        .call(
            ProviderName::from(TEST_PROVIDER),
            MethodName::from("sum"),
            None,
        )
        .await
        .unwrap_err();

    match err {
        OrchestratorError::Action(ActionError::ForbiddenByPolicy {
            interceptor,
            action,
            phase,
        }) => {
            assert_eq!(interceptor, "policy_test");
            assert_eq!(action, ModifyParams::action_kind());
            assert_eq!(
                phase,
                actrpc_core::interception::InterceptionPhase::Outbound
            );
        }
        other => panic!("unexpected error: {other:?}"),
    }

    assert!(client.sent().is_empty());
}

#[tokio::test]
async fn default_orchestrator_errors_when_handler_is_missing() {
    let interceptor = Arc::new(QueuedInterceptor::new(vec![InterceptionResponse {
        continuation: InterceptorContinuation::Stop,
        actions: vec![RequestedActionRecord {
            kind: ActionKind::from("missing_action"),
            params: Some(json!({})),
        }],
    }]));

    let catalog = single_interceptor_catalog(
        "missing_handler",
        interceptor,
        InterceptorPolicy {
            outbound: HashSet::from([ActionKind::from("missing_action")]),
            inbound: HashSet::new(),
        },
        vec!["missing_handler"],
        vec![],
    );

    let client = Arc::new(RecordingClient::new(response_message(json!(
        "should_not_send"
    ))));

    let orchestrator = test_orchestrator(catalog, client.clone()).await;

    let err = orchestrator
        .call(
            ProviderName::from(TEST_PROVIDER),
            MethodName::from("sum"),
            None,
        )
        .await
        .unwrap_err();

    match err {
        OrchestratorError::Action(ActionError::HandlerNotFound { action }) => {
            assert_eq!(action, ActionKind::from("missing_action"));
        }
        other => panic!("unexpected error: {other:?}"),
    }

    assert!(client.sent().is_empty());
}

#[tokio::test]
async fn reinvoke_reuses_prior_actions_only_for_same_interceptor() {
    let first = Arc::new(QueuedInterceptor::new(vec![
        InterceptionResponse {
            continuation: InterceptorContinuation::Reinvoke,
            actions: vec![RequestedActionRecord {
                kind: ModifyParams::action_kind(),
                params: Some(json!({
                    "params": [10, 20]
                })),
            }],
        },
        InterceptionResponse {
            continuation: InterceptorContinuation::Stop,
            actions: vec![],
        },
    ]));

    let second = Arc::new(QueuedInterceptor::new(vec![InterceptionResponse {
        continuation: InterceptorContinuation::Stop,
        actions: vec![],
    }]));

    let catalog = two_interceptor_catalog(
        ("first", first.clone()),
        ("second", second.clone()),
        InterceptorPolicy {
            outbound: HashSet::from([ModifyParams::action_kind()]),
            inbound: HashSet::new(),
        },
        InterceptorPolicy {
            outbound: HashSet::new(),
            inbound: HashSet::new(),
        },
        vec!["first", "second"],
        vec![],
    );

    let client = Arc::new(RecordingClient::new(response_message(json!("ok"))));

    let orchestrator = test_orchestrator(catalog, client.clone()).await;

    let result = orchestrator
        .call(
            ProviderName::from(TEST_PROVIDER),
            MethodName::from("sum"),
            Some(JsonRpcParams::Array(vec![json!(1), json!(2)])),
        )
        .await
        .unwrap();

    assert_eq!(result, response_message(json!("ok")));

    let first_seen = first.seen();
    assert_eq!(first_seen.len(), 2);
    assert!(first_seen[0].resolved_action_history.is_empty());
    assert_eq!(first_seen[1].resolved_action_history.len(), 1);
    assert_eq!(first_seen[1].resolved_action_history[0].len(), 1);
    assert_eq!(
        first_seen[1].resolved_action_history[0][0].kind,
        ModifyParams::action_kind()
    );

    let second_seen = second.seen();
    assert_eq!(second_seen.len(), 1);
    assert!(
        second_seen[0].resolved_action_history.is_empty(),
        "resolved_action_history must not leak from one interceptor to the next"
    );

    let sent = client.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(
        sent[0],
        request_message(
            "sum",
            Some(JsonRpcParams::Array(vec![json!(10), json!(20)]))
        )
    );
}

async fn test_orchestrator(
    catalog: InterceptorCatalog,
    client: Arc<RecordingClient>,
) -> DefaultOrchestrator {
    let client_provider = StaticProvider {
        client: client as Arc<dyn JsonRpcClient<Error = TransportError>>,
    };
    let endpoint_name = actrpc_orchestrator::EndpointName::new("test_ep");
    let ep_config = actrpc_orchestrator::EndpointConfig {
        name: endpoint_name.clone(),
        target: dummy_target(),
    };
    let endpoint_catalog = actrpc_orchestrator::EndpointCatalog::from_configs(
        vec![ep_config],
        &[],
        &[],
        &client_provider,
        &NoopSessionProvider,
    )
    .await
    .unwrap();

    let method_source =
        MethodSourceConfig::JsonRpc(actrpc_orchestrator::method::JsonRpcMethodSourceConfig {
            provider: ProviderName::from(TEST_PROVIDER),
            endpoint: endpoint_name,
            discovery: actrpc_orchestrator::method::JsonRpcMethodDiscoveryConfig::Static {
                methods: vec![
                    MethodInfo {
                        name: MethodName::from("sum"),
                        description: None,
                        info: json!({}),
                    },
                    MethodInfo {
                        name: MethodName::from("get"),
                        description: None,
                        info: json!({}),
                    },
                ],
            },
        });

    let method_catalog = MethodCatalog::from_configs(vec![method_source], &endpoint_catalog)
        .await
        .unwrap();

    let resources = Arc::new(OrchestratorResources::new(
        Arc::new(catalog),
        Arc::new(method_catalog),
    ));

    let factory = Arc::new(CallExecutionFactory::new(resources));

    DefaultOrchestrator::new(factory)
}

fn single_interceptor_catalog(
    name: &str,
    interceptor: Arc<dyn Interceptor>,
    policy: InterceptorPolicy,
    outbound: Vec<&str>,
    inbound: Vec<&str>,
) -> InterceptorCatalog {
    let mut entries = HashMap::new();

    entries.insert(
        name.to_owned(),
        InterceptorCatalogEntry {
            name: name.to_owned(),
            policy,
            interceptor,
            runtime_limits: None,
        },
    );

    InterceptorCatalog::new(
        entries,
        ImmutableInterceptorPipeline::new(outbound.into_iter().map(str::to_owned).collect()),
        ImmutableInterceptorPipeline::new(inbound.into_iter().map(str::to_owned).collect()),
    )
}

fn two_interceptor_catalog(
    first: (&str, Arc<dyn Interceptor>),
    second: (&str, Arc<dyn Interceptor>),
    first_policy: InterceptorPolicy,
    second_policy: InterceptorPolicy,
    outbound: Vec<&str>,
    inbound: Vec<&str>,
) -> InterceptorCatalog {
    let mut entries = HashMap::new();

    entries.insert(
        first.0.to_owned(),
        InterceptorCatalogEntry {
            name: first.0.to_owned(),
            policy: first_policy,
            interceptor: first.1,
            runtime_limits: None,
        },
    );

    entries.insert(
        second.0.to_owned(),
        InterceptorCatalogEntry {
            name: second.0.to_owned(),
            policy: second_policy,
            interceptor: second.1,
            runtime_limits: None,
        },
    );

    InterceptorCatalog::new(
        entries,
        ImmutableInterceptorPipeline::new(outbound.into_iter().map(str::to_owned).collect()),
        ImmutableInterceptorPipeline::new(inbound.into_iter().map(str::to_owned).collect()),
    )
}

struct QueuedInterceptor {
    responses: Mutex<VecDeque<InterceptionResponse>>,
    seen: Mutex<Vec<InterceptionRequest>>,
}

impl QueuedInterceptor {
    fn new(responses: Vec<InterceptionResponse>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            seen: Mutex::new(Vec::new()),
        }
    }

    fn seen(&self) -> Vec<InterceptionRequest> {
        self.seen.lock().unwrap().clone()
    }
}

impl Interceptor for QueuedInterceptor {
    fn initialize<'a>(
        &'a self,
    ) -> InterceptorFuture<'a, Result<InterceptorInitialization, InterceptorRuntimeError>>
    where
        Self: 'a,
    {
        Box::pin(async move { Ok(InterceptorInitialization::default()) })
    }

    fn intercept<'a>(
        &'a self,
        request: &'a InterceptionRequest,
    ) -> InterceptorFuture<'a, Result<InterceptionResponse, InterceptorRuntimeError>>
    where
        Self: 'a,
    {
        Box::pin(async move {
            self.seen.lock().unwrap().push(request.clone());

            self.responses.lock().unwrap().pop_front().ok_or_else(|| {
                InterceptorRuntimeError::Internal {
                    message: "no queued response".to_owned(),
                }
            })
        })
    }
}

struct RecordingClient {
    response: JsonRpcMessage,
    sent: Mutex<Vec<JsonRpcMessage>>,
}

impl RecordingClient {
    fn new(response: JsonRpcMessage) -> Self {
        Self {
            response,
            sent: Mutex::new(Vec::new()),
        }
    }

    fn sent(&self) -> Vec<JsonRpcMessage> {
        self.sent.lock().unwrap().clone()
    }
}

impl JsonRpcClient for RecordingClient {
    type Error = TransportError;

    fn send<'a>(
        &'a self,
        message: JsonRpcMessage,
    ) -> JsonRpcClientFuture<'a, Result<JsonRpcMessage, Self::Error>> {
        Box::pin(async move {
            self.sent.lock().unwrap().push(message);
            Ok(self.response.clone())
        })
    }
}

struct StaticProvider {
    client: Arc<dyn JsonRpcClient<Error = TransportError>>,
}

impl JsonRpcClientProvider for StaticProvider {
    type Error = TransportError;
    type Client = Arc<dyn JsonRpcClient<Error = TransportError>>;

    fn get_client<'a>(
        &'a self,
        _target: &'a TransportTarget,
    ) -> JsonRpcClientProviderFuture<'a, Result<Self::Client, Self::Error>> {
        Box::pin(async move { Ok(self.client.clone()) })
    }
}

fn request_message(method: &str, params: Option<JsonRpcParams>) -> JsonRpcMessage {
    JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
        jsonrpc: JsonRpcVersion::V2_0,
        id: JsonRpcId::Number(1.into()),
        method: method.to_owned(),
        params,
    }))
}

fn response_message(result: serde_json::Value) -> JsonRpcMessage {
    JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Success(
        JsonRpcSuccessResponse {
            jsonrpc: JsonRpcVersion::V2_0,
            id: JsonRpcId::Number(1.into()),
            result,
        },
    )))
}

fn dummy_target() -> TransportTarget {
    TransportTarget::Http(HttpTarget {
        url: "http://example.invalid/rpc".to_owned(),
        headers: vec![],
        timeout_ms: 1_000,
    })
}

struct NoopSessionProvider;

impl JsonRpcSessionProvider for NoopSessionProvider {
    type Error = TransportError;
    type Session = Arc<dyn JsonRpcSession<Error = TransportError>>;

    fn get_session<'a>(
        &'a self,
        _target: &'a TransportTarget,
    ) -> JsonRpcSessionProviderFuture<'a, Result<Self::Session, Self::Error>> {
        Box::pin(async move {
            Err(TransportError::Internal {
                message: "session not used in this test".to_owned(),
            })
        })
    }
}
