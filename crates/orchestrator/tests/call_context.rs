use actrpc_core::{
    CallContext, InterceptionContext, InterceptorInitialization, MAX_CALL_CONTEXT_BYTES,
    MAX_INTERCEPTOR_CTX_ENTRIES, MethodTarget,
    action::{ActionSpec, RequestedActionRecord},
    interception::{InterceptionRequest, InterceptionResponse, InterceptorContinuation},
    json_rpc::{
        JsonRpcId, JsonRpcMessage, JsonRpcParams, JsonRpcRequest, JsonRpcResponse,
        JsonRpcSingleMessage, JsonRpcSuccessResponse, JsonRpcVersion,
    },
    participant::{Participant, ParticipantType},
};
use actrpc_orchestrator::{
    action::actions::{
        call_method::{CallMethod, CallMethodParams},
        modify_params::ModifyParams,
    },
    error::{ActionError, OrchestratorError},
    interceptor::{
        ImmutableInterceptorPipeline, Interceptor, InterceptorCatalog, InterceptorCatalogEntry,
        InterceptorFuture, InterceptorPolicy,
    },
    method::{MethodCatalog, MethodInfo, MethodName, MethodSourceConfig, ProviderName},
    review::UnavailableReviewProvider,
    runtime::{CallExecutionFactory, CallRuntime, OrchestratorResources},
};
use actrpc_transport::{
    JsonRpcClient, JsonRpcClientFuture, JsonRpcClientProvider, JsonRpcClientProviderFuture,
    JsonRpcSession, JsonRpcSessionProvider, JsonRpcSessionProviderFuture, TransportError,
    TransportTarget, target::HttpTarget,
};
use serde_json::json;
use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    sync::{Arc, Mutex},
};

const TEST_PROVIDER: &str = "test";

#[test]
fn call_method_params_rejects_unknown_target_field() {
    let err = serde_json::from_str::<CallMethodParams>(
        r#"{"provider":"p","method":"m","target":"p::m"}"#,
    )
    .unwrap_err();
    assert!(err.to_string().contains("unknown field"));
}

#[test]
fn call_method_params_accepts_optional_ctx() {
    let params: CallMethodParams = serde_json::from_value(json!({
        "provider": "agents",
        "method": "invoke",
        "ctx": {
            "shared": { "trace": "1" },
            "interceptors": {
                "dynamic_policy": { "mode": "detached" }
            }
        }
    }))
    .unwrap();

    assert!(params.ctx.is_some());
}

#[test]
fn interception_request_skips_empty_ctx() {
    let request = InterceptionRequest {
        origin: Participant {
            kind: ParticipantType::External,
            id: "caller".to_owned(),
        },
        target: MethodTarget {
            provider: "test".to_owned(),
            method: "m".to_owned(),
        },
        message: JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
            jsonrpc: JsonRpcVersion::V2_0,
            id: actrpc_core::json_rpc::JsonRpcId::Number(1.into()),
            method: "m".to_owned(),
            params: None,
        })),
        call_id: actrpc_core::CallId::new(),
        interception_id: actrpc_core::InterceptionId::new(),
        resolved_action_history: vec![],
        ctx: InterceptionContext::default(),
    };

    let encoded = serde_json::to_string(&request).unwrap();
    assert!(!encoded.contains("ctx"));
}

#[test]
fn interception_request_defaults_missing_ctx_on_deserialize() {
    let json = r#"{
        "origin": { "kind": "external", "id": "caller" },
        "target": { "provider": "p", "method": "m" },
        "message": {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "m"
        },
        "call_id": "550e8400-e29b-41d4-a716-446655440000",
        "interception_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
    }"#;

    let request: InterceptionRequest = serde_json::from_str(json).unwrap();
    assert_eq!(request.ctx, InterceptionContext::default());
}

#[test]
fn child_runtime_stores_ctx() {
    let transcript = Arc::new(actrpc_orchestrator::runtime::TranscriptState::new());
    let root_id = transcript.allocate_call_id();
    let child_id = transcript.allocate_call_id();

    let mut interceptors = BTreeMap::new();
    interceptors.insert("dynamic_policy".to_owned(), json!({ "mode": "detached" }));
    let ctx = CallContext {
        shared: Some(json!({ "trace": "1" })),
        interceptors,
    };

    let child_call = CallRuntime::nested(
        request_message("sum", None),
        transcript,
        child_id,
        root_id,
        root_id,
        1,
        Participant {
            kind: ParticipantType::Interceptor,
            id: "test".to_owned(),
        },
        MethodTarget {
            provider: TEST_PROVIDER.to_owned(),
            method: "sum".to_owned(),
        },
        Some(ctx.clone()),
    );

    assert_eq!(child_call.call_ctx(), Some(&ctx));
}

#[tokio::test]
async fn interceptor_receives_filtered_ctx_through_pipeline() {
    let ctx_probe = Arc::new(CtxProbeInterceptor::new());
    let catalog = single_interceptor_catalog("ctx_probe", ctx_probe.clone());

    let factory = test_factory(
        catalog,
        response_message(json!("ok")),
        actrpc_orchestrator::config::RuntimeConfig::default(),
    )
    .await;

    let transcript = Arc::new(actrpc_orchestrator::runtime::TranscriptState::new());
    let root_id = transcript.allocate_call_id();
    transcript
        .execution_tree()
        .register_root(root_id)
        .expect("register root");
    let parent_call = Arc::new(CallRuntime::root(
        request_message("sum", None),
        transcript,
        root_id,
        Participant {
            kind: ParticipantType::External,
            id: "caller".to_owned(),
        },
        MethodTarget {
            provider: TEST_PROVIDER.to_owned(),
            method: "sum".to_owned(),
        },
    ));

    let mut interceptors = BTreeMap::new();
    interceptors.insert("ctx_probe".to_owned(), json!({ "private": true }));
    interceptors.insert("other".to_owned(), json!({ "secret": true }));
    let ctx = CallContext {
        shared: Some(json!({ "trace": "abc" })),
        interceptors,
    };

    let child = factory
        .create_piped(
            ProviderName::from(TEST_PROVIDER),
            MethodName::from("sum"),
            None,
            Some(ctx),
            parent_call.as_ref(),
            Participant {
                kind: ParticipantType::Interceptor,
                id: "parent".to_owned(),
            },
        )
        .unwrap();

    child.run().await.unwrap();

    let seen = ctx_probe.seen();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].ctx.shared, Some(json!({ "trace": "abc" })));
    assert_eq!(seen[0].ctx.private, Some(json!({ "private": true })));
    assert!(seen[0].ctx.private.as_ref().unwrap().get("secret").is_none());
}

#[tokio::test]
async fn method_provider_params_do_not_contain_ctx() {
    let client = Arc::new(RecordingClient::new(response_message(json!("ok"))));
    let factory = test_factory_with_client(
        empty_catalog(),
        client.clone(),
        actrpc_orchestrator::config::RuntimeConfig::default(),
    )
    .await;

    let transcript = Arc::new(actrpc_orchestrator::runtime::TranscriptState::new());
    let root_id = transcript.allocate_call_id();
    transcript
        .execution_tree()
        .register_root(root_id)
        .expect("register root");
    let parent_call = Arc::new(CallRuntime::root(
        request_message("sum", Some(JsonRpcParams::Array(vec![json!(1), json!(2)]))),
        transcript,
        root_id,
        Participant {
            kind: ParticipantType::External,
            id: "caller".to_owned(),
        },
        MethodTarget {
            provider: TEST_PROVIDER.to_owned(),
            method: "sum".to_owned(),
        },
    ));

    let mut interceptors = BTreeMap::new();
    interceptors.insert("dynamic_policy".to_owned(), json!({ "mode": "detached" }));
    let ctx = CallContext {
        shared: Some(json!({ "trace": "1" })),
        interceptors,
    };

    let child = factory
        .create_piped(
            ProviderName::from(TEST_PROVIDER),
            MethodName::from("sum"),
            Some(JsonRpcParams::Array(vec![json!(1), json!(2)])),
            Some(ctx),
            parent_call.as_ref(),
            Participant {
                kind: ParticipantType::Interceptor,
                id: "test".to_owned(),
            },
        )
        .unwrap();

    child.run().await.unwrap();

    let sent = client.sent();
    assert_eq!(sent.len(), 1);
    let encoded = serde_json::to_string(&sent[0]).unwrap();
    assert!(!encoded.contains("ctx"));
    assert!(!encoded.contains("interceptors"));
}

#[tokio::test]
async fn oversized_ctx_fails_call_method_deterministically() {
    let mut interceptors = BTreeMap::new();
    for index in 0..=MAX_INTERCEPTOR_CTX_ENTRIES {
        interceptors.insert(format!("i{index}"), json!({ "v": index }));
    }

    let interceptor = QueuedInterceptor::new(vec![InterceptionResponse {
        continuation: InterceptorContinuation::Stop,
        actions: vec![RequestedActionRecord {
            kind: CallMethod::action_kind(),
            params: Some(json!({
                "provider": TEST_PROVIDER,
                "method": "sum",
                "ctx": {
                    "interceptors": interceptors
                }
            })),
        }],
    }]);

    let execution = test_factory(
        single_interceptor_catalog("ctx_fail", Arc::new(interceptor)),
        response_message(json!("ok")),
        actrpc_orchestrator::config::RuntimeConfig::default(),
    )
    .await
    .create_root(ProviderName::from(TEST_PROVIDER), MethodName::from("sum"), None, "caller")
    .unwrap();

    let err = execution.run().await.unwrap_err();
    let message = err.to_string();
    assert!(message.contains("interceptor ctx entry count"));
    assert!(message.contains("exceeds max"));
}

#[tokio::test]
async fn oversized_serialized_ctx_fails_call_method_deterministically() {
    let large = "x".repeat(MAX_CALL_CONTEXT_BYTES + 1);
    let interceptor = QueuedInterceptor::new(vec![InterceptionResponse {
        continuation: InterceptorContinuation::Stop,
        actions: vec![RequestedActionRecord {
            kind: CallMethod::action_kind(),
            params: Some(json!({
                "provider": TEST_PROVIDER,
                "method": "sum",
                "ctx": {
                    "shared": large
                }
            })),
        }],
    }]);

    let execution = test_factory(
        single_interceptor_catalog("ctx_fail", Arc::new(interceptor)),
        response_message(json!("ok")),
        actrpc_orchestrator::config::RuntimeConfig::default(),
    )
    .await
    .create_root(ProviderName::from(TEST_PROVIDER), MethodName::from("sum"), None, "caller")
    .unwrap();

    let err = execution.run().await.unwrap_err();
    let message = err.to_string();
    assert!(message.contains("serialized ctx size"));
    assert!(message.contains("exceeds max"));
}

#[tokio::test]
async fn fail_fast_stops_after_first_failed_action() {
    let interceptor = QueuedInterceptor::new(vec![InterceptionResponse {
        continuation: InterceptorContinuation::Stop,
        actions: vec![
            RequestedActionRecord {
                kind: CallMethod::action_kind(),
                params: Some(json!({
                    "provider": TEST_PROVIDER,
                    "method": "sum",
                    "ctx": { "interceptors": { "x": 1, "y": 2, "z": 3, "a": 4, "b": 5, "c": 6, "d": 7, "e": 8, "f": 9, "g": 10, "h": 11, "i": 12, "j": 13, "k": 14, "l": 15, "m": 16, "n": 17 } }
                })),
            },
            RequestedActionRecord {
                kind: CallMethod::action_kind(),
                params: Some(json!({
                    "provider": TEST_PROVIDER,
                    "method": "sum",
                    "params": [99]
                })),
            },
        ],
    }]);

    let client = Arc::new(RecordingClient::new(response_message(json!("ok"))));
    let factory = test_factory_with_client(
        single_interceptor_catalog("fail_fast", Arc::new(interceptor)),
        client.clone(),
        actrpc_orchestrator::config::RuntimeConfig::default(),
    )
    .await;

    let execution = factory
        .create_root(ProviderName::from(TEST_PROVIDER), MethodName::from("sum"), Some(JsonRpcParams::Array(vec![json!(1)])), "caller")
        .unwrap();

    let transcript = execution.transcript();
    let err = execution.run().await.unwrap_err();
    assert!(matches!(err, OrchestratorError::Action(ActionError::HandlerFailed { .. })));

    // Outbound action failure stops the round before downstream send or later actions.
    assert!(client.sent().is_empty());

    // The second CallMethod in the same round did not run: no nested call was created.
    let entries = transcript.snapshot().unwrap();
    assert!(
        entries.iter().all(|entry| entry.depth == 0),
        "expected no nested calls when fail-fast stops the action round"
    );
}

struct QueuedInterceptor {
    responses: Mutex<VecDeque<InterceptionResponse>>,
}

impl QueuedInterceptor {
    fn new(responses: Vec<InterceptionResponse>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
        }
    }
}

impl Interceptor for QueuedInterceptor {
    fn initialize<'a>(
        &'a self,
    ) -> InterceptorFuture<'a, Result<InterceptorInitialization, actrpc_orchestrator::error::InterceptorRuntimeError>>
    where
        Self: 'a,
    {
        Box::pin(async move { Ok(InterceptorInitialization::default()) })
    }

    fn intercept<'a>(
        &'a self,
        request: &'a InterceptionRequest,
    ) -> InterceptorFuture<'a, Result<InterceptionResponse, actrpc_orchestrator::error::InterceptorRuntimeError>>
    where
        Self: 'a,
    {
        let _ = request;
        Box::pin(async move {
            self.responses.lock().unwrap().pop_front().ok_or_else(|| {
                actrpc_orchestrator::error::InterceptorRuntimeError::Internal {
                    message: "no queued response".to_owned(),
                }
            })
        })
    }
}

struct CtxProbeInterceptor {
    seen: Mutex<Vec<InterceptionRequest>>,
}

impl CtxProbeInterceptor {
    fn new() -> Self {
        Self {
            seen: Mutex::new(Vec::new()),
        }
    }

    fn seen(&self) -> Vec<InterceptionRequest> {
        self.seen.lock().unwrap().clone()
    }
}

impl Interceptor for CtxProbeInterceptor {
    fn initialize<'a>(
        &'a self,
    ) -> InterceptorFuture<'a, Result<InterceptorInitialization, actrpc_orchestrator::error::InterceptorRuntimeError>>
    where
        Self: 'a,
    {
        Box::pin(async move { Ok(InterceptorInitialization::default()) })
    }

    fn intercept<'a>(
        &'a self,
        request: &'a InterceptionRequest,
    ) -> InterceptorFuture<'a, Result<InterceptionResponse, actrpc_orchestrator::error::InterceptorRuntimeError>>
    where
        Self: 'a,
    {
        Box::pin(async move {
            self.seen.lock().unwrap().push(request.clone());
            Ok(InterceptionResponse {
                actions: vec![],
                continuation: InterceptorContinuation::Stop,
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

fn empty_catalog() -> InterceptorCatalog {
    InterceptorCatalog::new(
        HashMap::new(),
        ImmutableInterceptorPipeline::new(vec![]),
        ImmutableInterceptorPipeline::new(vec![]),
    )
}

fn single_interceptor_catalog(name: &str, interceptor: Arc<dyn Interceptor>) -> InterceptorCatalog {
    let mut entries = HashMap::new();
    entries.insert(
        name.to_owned(),
        InterceptorCatalogEntry {
            name: name.to_owned(),
            policy: InterceptorPolicy {
                outbound: HashSet::from([CallMethod::action_kind(), ModifyParams::action_kind()]),
                inbound: HashSet::new(),
            },
            interceptor,
            runtime_limits: None,
        },
    );

    InterceptorCatalog::new(
        entries,
        ImmutableInterceptorPipeline::new(vec![name.to_owned()]),
        ImmutableInterceptorPipeline::new(vec![]),
    )
}

async fn test_factory(
    catalog: InterceptorCatalog,
    response: JsonRpcMessage,
    runtime: actrpc_orchestrator::config::RuntimeConfig,
) -> Arc<CallExecutionFactory> {
    test_factory_with_client(
        catalog,
        Arc::new(RecordingClient::new(response)),
        runtime,
    )
    .await
}

async fn test_factory_with_client(
    catalog: InterceptorCatalog,
    client: Arc<RecordingClient>,
    runtime: actrpc_orchestrator::config::RuntimeConfig,
) -> Arc<CallExecutionFactory> {
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

    let method_source = MethodSourceConfig::JsonRpc(actrpc_orchestrator::method::JsonRpcMethodSourceConfig {
        provider: ProviderName::from(TEST_PROVIDER),
        endpoint: endpoint_name,
        discovery: actrpc_orchestrator::method::JsonRpcMethodDiscoveryConfig::Static {
            methods: vec![MethodInfo {
                name: MethodName::from("sum"),
                description: None,
                info: json!({}),
            }],
        },
    });

    let method_catalog = MethodCatalog::from_configs(vec![method_source], &endpoint_catalog)
        .await
        .unwrap();

    let resources = Arc::new(OrchestratorResources::with_review_provider_and_runtime(
        Arc::new(catalog),
        Arc::new(method_catalog),
        Arc::new(UnavailableReviewProvider),
        vec![],
        runtime,
    ));

    Arc::new(CallExecutionFactory::new(resources))
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

fn request_message(method: &str, params: Option<JsonRpcParams>) -> JsonRpcMessage {
    JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
        jsonrpc: JsonRpcVersion::V2_0,
        id: JsonRpcId::Number(1.into()),
        method: method.to_owned(),
        params,
    }))
}