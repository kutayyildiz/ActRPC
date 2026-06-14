use actrpc_core::{
    InterceptorInitialization,
    action::{ActionSpec, RequestedActionRecord},
    interception::{InterceptionRequest, InterceptionResponse, InterceptorContinuation},
    json_rpc::{
        JsonRpcId, JsonRpcMessage, JsonRpcParams, JsonRpcRequest, JsonRpcResponse,
        JsonRpcSingleMessage, JsonRpcSuccessResponse, JsonRpcVersion,
    },
};
use actrpc_orchestrator::{
    PROTOCOL_INTERCEPTOR_REQUEST, PROTOCOL_INTERCEPTOR_RESPONSE, PROTOCOL_METHOD_REQUEST,
    PROTOCOL_METHOD_RESPONSE,
    action::actions::{call_method::CallMethod, modify_params::ModifyParams},
    config::{OrchestratorConfig, RuntimeConfig},
    error::OrchestratorError,
    interceptor::{
        ImmutableInterceptorPipeline, Interceptor, InterceptorCatalog, InterceptorCatalogEntry,
        InterceptorFuture, InterceptorPolicy, InterceptorRuntimeLimitsOverride,
    },
    method::{MethodCatalog, MethodInfo, MethodName, MethodSourceConfig, ProviderName},
    review::UnavailableReviewProvider,
    runtime::{CallExecutionFactory, OrchestratorResources},
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
const NESTED_PROVIDER: &str = "nested";

#[tokio::test]
async fn root_call_records_interceptor_and_provider_exchanges() {
    let interceptor = Arc::new(QueuedInterceptor::new(vec![InterceptionResponse {
        continuation: InterceptorContinuation::Stop,
        actions: vec![],
    }]));

    let factory = test_factory(
        single_interceptor_catalog(
            "policy",
            interceptor,
            InterceptorPolicy {
                outbound: HashSet::new(),
                inbound: HashSet::new(),
            },
            vec!["policy"],
            vec![],
            None,
        ),
        RecordingClient::single(response_message(json!("ok"))),
        RuntimeConfig::default(),
    )
    .await;

    let execution = factory
        .create_root(
            ProviderName::from(TEST_PROVIDER),
            MethodName::from("sum"),
            None,
            "caller",
        )
        .unwrap();

    let transcript = execution.transcript();
    execution.run().await.unwrap();

    let entries = transcript.snapshot().unwrap();
    assert_eq!(entries.len(), 4);
    assert_eq!(entries[0].protocol, PROTOCOL_INTERCEPTOR_REQUEST);
    assert_eq!(entries[0].from, "orchestrator:main");
    assert_eq!(entries[0].to, "interceptor:policy");
    assert_eq!(entries[1].protocol, PROTOCOL_INTERCEPTOR_RESPONSE);
    assert_eq!(entries[2].protocol, PROTOCOL_METHOD_REQUEST);
    assert_eq!(entries[2].to, "method_provider:test");
    assert_eq!(entries[3].protocol, PROTOCOL_METHOD_RESPONSE);
    assert_eq!(entries[3].from, "method_provider:test");

    assert!(entries[0].seq < entries[1].seq);
    assert!(entries[1].seq < entries[2].seq);
    assert!(entries[1].ts_ms > 0);

    let request: InterceptionRequest = serde_json::from_value(entries[0].message.clone()).unwrap();
    assert!(request.resolved_action_history.is_empty());

    let _: InterceptionResponse = serde_json::from_value(entries[1].message.clone()).unwrap();
    let _: JsonRpcMessage = serde_json::from_value(entries[2].message.clone()).unwrap();
    let _: JsonRpcMessage = serde_json::from_value(entries[3].message.clone()).unwrap();
}

#[tokio::test]
async fn method_request_reflects_outbound_interceptor_mutation() {
    let interceptor = Arc::new(QueuedInterceptor::new(vec![InterceptionResponse {
        continuation: InterceptorContinuation::Stop,
        actions: vec![RequestedActionRecord {
            kind: ModifyParams::action_kind(),
            params: Some(json!({ "params": [10, 20] })),
        }],
    }]));

    let factory = test_factory(
        single_interceptor_catalog(
            "mutator",
            interceptor,
            InterceptorPolicy {
                outbound: HashSet::from([ModifyParams::action_kind()]),
                inbound: HashSet::new(),
            },
            vec!["mutator"],
            vec![],
            None,
        ),
        RecordingClient::single(response_message(json!("ok"))),
        RuntimeConfig::default(),
    )
    .await;

    let execution = factory
        .create_root(
            ProviderName::from(TEST_PROVIDER),
            MethodName::from("sum"),
            Some(JsonRpcParams::Array(vec![json!(1), json!(2)])),
            "caller",
        )
        .unwrap();

    let transcript = execution.transcript();
    execution.run().await.unwrap();

    let method_request = transcript
        .snapshot()
        .unwrap()
        .into_iter()
        .find(|entry| entry.protocol == PROTOCOL_METHOD_REQUEST)
        .unwrap();

    let message: JsonRpcMessage = serde_json::from_value(method_request.message).unwrap();
    assert_eq!(
        message,
        request_message(
            "sum",
            Some(JsonRpcParams::Array(vec![json!(10), json!(20)]))
        )
    );
}

#[tokio::test]
async fn nested_call_method_produces_linked_call_ids() {
    let interceptor = Arc::new(QueuedInterceptor::new(vec![
        InterceptionResponse {
            continuation: InterceptorContinuation::Stop,
            actions: vec![RequestedActionRecord {
                kind: CallMethod::action_kind(),
                params: Some(json!({
                    "provider": NESTED_PROVIDER,
                    "method": "nested_method"
                })),
            }],
        },
        InterceptionResponse {
            continuation: InterceptorContinuation::Stop,
            actions: vec![],
        },
    ]));

    let factory = test_factory_with_nested_provider(
        single_interceptor_catalog(
            "caller",
            interceptor,
            InterceptorPolicy {
                outbound: HashSet::from([CallMethod::action_kind()]),
                inbound: HashSet::new(),
            },
            vec!["caller"],
            vec![],
            None,
        ),
        response_message(json!("parent_ok")),
        response_message(json!("nested_ok")),
        RuntimeConfig::default(),
    )
    .await;

    let execution = factory
        .create_root(
            ProviderName::from(TEST_PROVIDER),
            MethodName::from("sum"),
            None,
            "caller",
        )
        .unwrap();

    let transcript = execution.transcript();
    execution.run().await.unwrap();

    let entries = transcript.snapshot().unwrap();
    let call_ids: HashSet<actrpc_core::CallId> =
        entries.iter().map(|entry| entry.call_id).collect();
    assert_eq!(call_ids.len(), 2);

    let root_entries: Vec<_> = entries.iter().filter(|entry| entry.depth == 0).collect();
    let nested_entries: Vec<_> = entries.iter().filter(|entry| entry.depth == 1).collect();

    assert!(!root_entries.is_empty());
    assert!(!nested_entries.is_empty());

    let root_id = root_entries[0].call_id;
    assert!(
        nested_entries
            .iter()
            .all(|entry| entry.parent_call_id == Some(root_id))
    );
}

#[tokio::test]
async fn max_call_depth_zero_rejects_nested_call_method() {
    let interceptor = Arc::new(QueuedInterceptor::new(vec![InterceptionResponse {
        continuation: InterceptorContinuation::Stop,
        actions: vec![RequestedActionRecord {
            kind: CallMethod::action_kind(),
            params: Some(json!({
                "provider": NESTED_PROVIDER,
                "method": "nested_method"
            })),
        }],
    }]));

    let factory = test_factory_with_nested_provider(
        single_interceptor_catalog(
            "caller",
            interceptor,
            InterceptorPolicy {
                outbound: HashSet::from([CallMethod::action_kind()]),
                inbound: HashSet::new(),
            },
            vec!["caller"],
            vec![],
            None,
        ),
        response_message(json!("parent_ok")),
        response_message(json!("nested_ok")),
        RuntimeConfig {
            max_call_depth: 0,
            ..RuntimeConfig::default()
        },
    )
    .await;

    let execution = factory
        .create_root(
            ProviderName::from(TEST_PROVIDER),
            MethodName::from("sum"),
            None,
            "caller",
        )
        .unwrap();

    let err = execution.run().await.unwrap_err();
    assert!(matches!(err, OrchestratorError::Action(_)));
}

#[tokio::test]
async fn reinvoke_limit_is_enforced() {
    let interceptor = Arc::new(QueuedInterceptor::new(vec![
        InterceptionResponse {
            continuation: InterceptorContinuation::Reinvoke,
            actions: vec![],
        },
        InterceptionResponse {
            continuation: InterceptorContinuation::Reinvoke,
            actions: vec![],
        },
    ]));

    let factory = test_factory(
        single_interceptor_catalog(
            "reinvoker",
            interceptor,
            empty_policy(),
            vec!["reinvoker"],
            vec![],
            None,
        ),
        RecordingClient::single(response_message(json!("ok"))),
        RuntimeConfig {
            max_interception_reinvokes: 0,
            ..RuntimeConfig::default()
        },
    )
    .await;

    let execution = factory
        .create_root(
            ProviderName::from(TEST_PROVIDER),
            MethodName::from("sum"),
            None,
            "caller",
        )
        .unwrap();

    let err = execution.run().await.unwrap_err();
    assert!(matches!(
        err,
        OrchestratorError::MaxInterceptionReinvokesExceeded { .. }
    ));
}

#[tokio::test]
async fn reinvoke_with_actions_still_trips_limit() {
    let interceptor = Arc::new(QueuedInterceptor::new(vec![
        InterceptionResponse {
            continuation: InterceptorContinuation::Reinvoke,
            actions: vec![RequestedActionRecord {
                kind: ModifyParams::action_kind(),
                params: Some(json!({ "params": [1] })),
            }],
        },
        InterceptionResponse {
            continuation: InterceptorContinuation::Reinvoke,
            actions: vec![],
        },
    ]));

    let factory = test_factory(
        single_interceptor_catalog(
            "reinvoker",
            interceptor,
            InterceptorPolicy {
                outbound: HashSet::from([ModifyParams::action_kind()]),
                inbound: HashSet::new(),
            },
            vec!["reinvoker"],
            vec![],
            None,
        ),
        RecordingClient::single(response_message(json!("ok"))),
        RuntimeConfig {
            max_interception_reinvokes: 0,
            ..RuntimeConfig::default()
        },
    )
    .await;

    let execution = factory
        .create_root(
            ProviderName::from(TEST_PROVIDER),
            MethodName::from("sum"),
            None,
            "caller",
        )
        .unwrap();

    let err = execution.run().await.unwrap_err();
    assert!(matches!(
        err,
        OrchestratorError::MaxInterceptionReinvokesExceeded { .. }
    ));
}

#[tokio::test]
async fn interception_timeout_is_enforced() {
    let interceptor = Arc::new(SleepingInterceptor);

    let factory = test_factory(
        single_interceptor_catalog(
            "slow",
            interceptor,
            empty_policy(),
            vec!["slow"],
            vec![],
            None,
        ),
        RecordingClient::single(response_message(json!("ok"))),
        RuntimeConfig {
            interception_request_timeout_ms: 50,
            ..RuntimeConfig::default()
        },
    )
    .await;

    let execution = factory
        .create_root(
            ProviderName::from(TEST_PROVIDER),
            MethodName::from("sum"),
            None,
            "caller",
        )
        .unwrap();

    let transcript = execution.transcript();
    let err = execution.run().await.unwrap_err();

    assert!(matches!(
        err,
        OrchestratorError::InterceptionRequestTimeout { .. }
    ));

    let entries = transcript.snapshot().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].protocol, PROTOCOL_INTERCEPTOR_REQUEST);
}

#[tokio::test]
async fn max_actions_per_interception_is_enforced_before_execution() {
    let interceptor = Arc::new(QueuedInterceptor::new(vec![InterceptionResponse {
        continuation: InterceptorContinuation::Stop,
        actions: vec![
            RequestedActionRecord {
                kind: ModifyParams::action_kind(),
                params: Some(json!({ "params": [1] })),
            },
            RequestedActionRecord {
                kind: ModifyParams::action_kind(),
                params: Some(json!({ "params": [2] })),
            },
        ],
    }]));

    let factory = test_factory(
        single_interceptor_catalog(
            "actions",
            interceptor,
            InterceptorPolicy {
                outbound: HashSet::from([ModifyParams::action_kind()]),
                inbound: HashSet::new(),
            },
            vec!["actions"],
            vec![],
            None,
        ),
        RecordingClient::single(response_message(json!("ok"))),
        RuntimeConfig {
            max_actions_per_interception: 1,
            ..RuntimeConfig::default()
        },
    )
    .await;

    let execution = factory
        .create_root(
            ProviderName::from(TEST_PROVIDER),
            MethodName::from("sum"),
            None,
            "caller",
        )
        .unwrap();

    let err = execution.run().await.unwrap_err();
    assert!(matches!(
        err,
        OrchestratorError::MaxActionsPerInterceptionExceeded { .. }
    ));
}

#[tokio::test]
async fn reinvoke_within_limit_succeeds() {
    let interceptor = Arc::new(QueuedInterceptor::new(vec![
        InterceptionResponse {
            continuation: InterceptorContinuation::Reinvoke,
            actions: vec![],
        },
        InterceptionResponse {
            continuation: InterceptorContinuation::Stop,
            actions: vec![],
        },
    ]));

    let factory = test_factory(
        single_interceptor_catalog(
            "reinvoker",
            interceptor,
            empty_policy(),
            vec!["reinvoker"],
            vec![],
            None,
        ),
        RecordingClient::single(response_message(json!("ok"))),
        RuntimeConfig {
            max_interception_reinvokes: 1,
            ..RuntimeConfig::default()
        },
    )
    .await;

    let execution = factory
        .create_root(
            ProviderName::from(TEST_PROVIDER),
            MethodName::from("sum"),
            None,
            "caller",
        )
        .unwrap();

    execution.run().await.unwrap();
}

#[tokio::test]
async fn per_interceptor_timeout_override_allows_slow_interceptor() {
    let interceptor = Arc::new(SleepingInterceptor);

    let factory = test_factory(
        single_interceptor_catalog(
            "slow",
            interceptor,
            empty_policy(),
            vec!["slow"],
            vec![],
            Some(InterceptorRuntimeLimitsOverride {
                interception_request_timeout_ms: Some(500),
                ..InterceptorRuntimeLimitsOverride::default()
            }),
        ),
        RecordingClient::single(response_message(json!("ok"))),
        RuntimeConfig {
            interception_request_timeout_ms: 50,
            ..RuntimeConfig::default()
        },
    )
    .await;

    let execution = factory
        .create_root(
            ProviderName::from(TEST_PROVIDER),
            MethodName::from("sum"),
            None,
            "caller",
        )
        .unwrap();

    execution.run().await.unwrap();
}

#[tokio::test]
async fn per_interceptor_reinvoke_override_relaxes_global_limit() {
    let interceptor = Arc::new(QueuedInterceptor::new(vec![
        InterceptionResponse {
            continuation: InterceptorContinuation::Reinvoke,
            actions: vec![],
        },
        InterceptionResponse {
            continuation: InterceptorContinuation::Stop,
            actions: vec![],
        },
    ]));

    let factory = test_factory(
        single_interceptor_catalog(
            "reinvoker",
            interceptor,
            empty_policy(),
            vec!["reinvoker"],
            vec![],
            Some(InterceptorRuntimeLimitsOverride {
                max_interception_reinvokes: Some(1),
                ..InterceptorRuntimeLimitsOverride::default()
            }),
        ),
        RecordingClient::single(response_message(json!("ok"))),
        RuntimeConfig {
            max_interception_reinvokes: 0,
            ..RuntimeConfig::default()
        },
    )
    .await;

    let execution = factory
        .create_root(
            ProviderName::from(TEST_PROVIDER),
            MethodName::from("sum"),
            None,
            "caller",
        )
        .unwrap();

    execution.run().await.unwrap();
}

#[tokio::test]
async fn guard_errors_include_global_config_hint() {
    let interceptor = Arc::new(QueuedInterceptor::new(vec![
        InterceptionResponse {
            continuation: InterceptorContinuation::Reinvoke,
            actions: vec![],
        },
        InterceptionResponse {
            continuation: InterceptorContinuation::Reinvoke,
            actions: vec![],
        },
    ]));

    let factory = test_factory(
        single_interceptor_catalog(
            "reinvoker",
            interceptor,
            empty_policy(),
            vec!["reinvoker"],
            vec![],
            None,
        ),
        RecordingClient::single(response_message(json!("ok"))),
        RuntimeConfig {
            max_interception_reinvokes: 0,
            ..RuntimeConfig::default()
        },
    )
    .await;

    let execution = factory
        .create_root(
            ProviderName::from(TEST_PROVIDER),
            MethodName::from("sum"),
            None,
            "caller",
        )
        .unwrap();

    let err = execution.run().await.unwrap_err();
    let message = err.to_string();
    assert!(message.contains("runtime.max_interception_reinvokes"));
}

#[tokio::test]
async fn guard_errors_include_interceptor_config_hint() {
    let interceptor = Arc::new(QueuedInterceptor::new(vec![
        InterceptionResponse {
            continuation: InterceptorContinuation::Reinvoke,
            actions: vec![],
        },
        InterceptionResponse {
            continuation: InterceptorContinuation::Reinvoke,
            actions: vec![],
        },
    ]));

    let factory = test_factory(
        single_interceptor_catalog(
            "reinvoker",
            interceptor,
            empty_policy(),
            vec!["reinvoker"],
            vec![],
            Some(InterceptorRuntimeLimitsOverride {
                max_interception_reinvokes: Some(0),
                ..InterceptorRuntimeLimitsOverride::default()
            }),
        ),
        RecordingClient::single(response_message(json!("ok"))),
        RuntimeConfig::default(),
    )
    .await;

    let execution = factory
        .create_root(
            ProviderName::from(TEST_PROVIDER),
            MethodName::from("sum"),
            None,
            "caller",
        )
        .unwrap();

    let err = execution.run().await.unwrap_err();
    let message = err.to_string();
    assert!(
        message.contains("interceptors[name=\"reinvoker\"].runtime.max_interception_reinvokes")
    );
}

#[test]
fn runtime_config_defaults_when_section_omitted() {
    let config: OrchestratorConfig = serde_yaml::from_str("endpoints: []").unwrap();
    let runtime = config.runtime.clone().unwrap_or_default();

    assert_eq!(runtime.max_call_depth, 8);
    assert_eq!(runtime.max_interception_reinvokes, 8);
    assert_eq!(runtime.interception_request_timeout_ms, 30_000);
    assert_eq!(runtime.max_actions_per_interception, 64);
    runtime.validate().unwrap();
}

#[test]
fn runtime_config_rejects_zero_interception_request_timeout() {
    let runtime = RuntimeConfig {
        interception_request_timeout_ms: 0,
        ..RuntimeConfig::default()
    };

    let err = runtime.validate().unwrap_err();
    let message = err.to_string();
    assert!(message.contains("runtime.interception_request_timeout_ms"));
    assert!(message.contains("greater than 0"));
}

#[test]
fn interceptor_runtime_override_rejects_zero_interception_request_timeout() {
    let overrides = InterceptorRuntimeLimitsOverride {
        interception_request_timeout_ms: Some(0),
        ..InterceptorRuntimeLimitsOverride::default()
    };

    let err = overrides.validate("slow").unwrap_err();
    let message = err.to_string();
    assert!(
        message.contains("interceptors[name=\"slow\"].runtime.interception_request_timeout_ms")
    );
    assert!(message.contains("greater than 0"));
}

#[test]
fn runtime_config_merge_preserves_first_file_when_second_omits_runtime() {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = PathBuf::from(format!("/tmp/actrpc_runtime_merge_{stamp}"));
    fs::create_dir_all(&dir).unwrap();

    let first = dir.join("first.yaml");
    let second = dir.join("second.yaml");

    fs::write(
        &first,
        r#"
runtime:
  max_call_depth: 4
endpoints: []
"#,
    )
    .unwrap();
    fs::write(&second, "endpoints: []\n").unwrap();

    let merged = OrchestratorConfig::from_paths([&first, &second]).unwrap();
    let runtime = merged.runtime.unwrap();
    assert_eq!(runtime.max_call_depth, 4);

    let _ = fs::remove_dir_all(&dir);
}

// --- test helpers (mirrors default_orchestrator patterns) ---

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
    ) -> InterceptorFuture<
        'a,
        Result<InterceptorInitialization, actrpc_orchestrator::error::InterceptorRuntimeError>,
    >
    where
        Self: 'a,
    {
        Box::pin(async move { Ok(InterceptorInitialization::default()) })
    }

    fn intercept<'a>(
        &'a self,
        _request: &'a InterceptionRequest,
    ) -> InterceptorFuture<
        'a,
        Result<InterceptionResponse, actrpc_orchestrator::error::InterceptorRuntimeError>,
    >
    where
        Self: 'a,
    {
        Box::pin(async move {
            self.responses.lock().unwrap().pop_front().ok_or_else(|| {
                actrpc_orchestrator::error::InterceptorRuntimeError::Internal {
                    message: "no queued response".to_owned(),
                }
            })
        })
    }
}

struct SleepingInterceptor;

impl Interceptor for SleepingInterceptor {
    fn initialize<'a>(
        &'a self,
    ) -> InterceptorFuture<
        'a,
        Result<InterceptorInitialization, actrpc_orchestrator::error::InterceptorRuntimeError>,
    >
    where
        Self: 'a,
    {
        Box::pin(async move { Ok(InterceptorInitialization::default()) })
    }

    fn intercept<'a>(
        &'a self,
        _request: &'a InterceptionRequest,
    ) -> InterceptorFuture<
        'a,
        Result<InterceptionResponse, actrpc_orchestrator::error::InterceptorRuntimeError>,
    >
    where
        Self: 'a,
    {
        Box::pin(async move {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            Ok(InterceptionResponse {
                continuation: InterceptorContinuation::Stop,
                actions: vec![],
            })
        })
    }
}

struct RecordingClient {
    responses: HashMap<String, JsonRpcMessage>,
    default: JsonRpcMessage,
}

impl RecordingClient {
    fn new(responses: HashMap<String, JsonRpcMessage>, default: JsonRpcMessage) -> Self {
        Self { responses, default }
    }

    fn single(response: JsonRpcMessage) -> Self {
        Self {
            responses: HashMap::new(),
            default: response,
        }
    }
}

impl JsonRpcClient for RecordingClient {
    type Error = TransportError;

    fn send<'a>(
        &'a self,
        message: JsonRpcMessage,
    ) -> JsonRpcClientFuture<'a, Result<JsonRpcMessage, Self::Error>> {
        let method = match &message {
            JsonRpcMessage::Single(JsonRpcSingleMessage::Request(request)) => {
                request.method.clone()
            }
            _ => String::new(),
        };
        let response = self
            .responses
            .get(&method)
            .cloned()
            .unwrap_or_else(|| self.default.clone());
        Box::pin(async move { Ok(response) })
    }
}

struct StaticProvider {
    client: Arc<RecordingClient>,
}

impl JsonRpcClientProvider for StaticProvider {
    type Error = TransportError;
    type Client = Arc<dyn JsonRpcClient<Error = TransportError>>;

    fn get_client<'a>(
        &'a self,
        _target: &'a TransportTarget,
    ) -> JsonRpcClientProviderFuture<'a, Result<Self::Client, Self::Error>> {
        let client = self.client.clone() as Arc<dyn JsonRpcClient<Error = TransportError>>;
        Box::pin(async move { Ok(client) })
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

async fn test_factory(
    catalog: InterceptorCatalog,
    client: RecordingClient,
    runtime: RuntimeConfig,
) -> Arc<CallExecutionFactory> {
    build_factory(catalog, client, runtime).await
}

async fn test_factory_with_nested_provider(
    catalog: InterceptorCatalog,
    parent_response: JsonRpcMessage,
    nested_response: JsonRpcMessage,
    runtime: RuntimeConfig,
) -> Arc<CallExecutionFactory> {
    let client = RecordingClient::new(
        HashMap::from([
            ("sum".to_owned(), parent_response.clone()),
            ("nested_method".to_owned(), nested_response),
        ]),
        parent_response,
    );

    build_factory(catalog, client, runtime).await
}

async fn build_factory(
    catalog: InterceptorCatalog,
    client: RecordingClient,
    runtime: RuntimeConfig,
) -> Arc<CallExecutionFactory> {
    let client = Arc::new(client);

    let client_provider = StaticProvider { client };

    let endpoint_name = actrpc_orchestrator::EndpointName::new("test_ep");
    let nested_endpoint = actrpc_orchestrator::EndpointName::new("nested_ep");

    let endpoint_catalog = actrpc_orchestrator::EndpointCatalog::from_configs(
        vec![
            actrpc_orchestrator::EndpointConfig {
                name: endpoint_name.clone(),
                target: dummy_target(),
            },
            actrpc_orchestrator::EndpointConfig {
                name: nested_endpoint.clone(),
                target: dummy_target(),
            },
        ],
        &[],
        &[],
        &client_provider,
        &NoopSessionProvider,
    )
    .await
    .unwrap();

    let method_sources = vec![
        MethodSourceConfig::JsonRpc(actrpc_orchestrator::method::JsonRpcMethodSourceConfig {
            provider: ProviderName::from(TEST_PROVIDER),
            endpoint: endpoint_name,
            discovery: actrpc_orchestrator::method::JsonRpcMethodDiscoveryConfig::Static {
                methods: vec![MethodInfo {
                    name: MethodName::from("sum"),
                    description: None,
                    info: json!({}),
                }],
            },
        }),
        MethodSourceConfig::JsonRpc(actrpc_orchestrator::method::JsonRpcMethodSourceConfig {
            provider: ProviderName::from(NESTED_PROVIDER),
            endpoint: nested_endpoint,
            discovery: actrpc_orchestrator::method::JsonRpcMethodDiscoveryConfig::Static {
                methods: vec![MethodInfo {
                    name: MethodName::from("nested_method"),
                    description: None,
                    info: json!({}),
                }],
            },
        }),
    ];

    let method_catalog = MethodCatalog::from_configs(method_sources, &endpoint_catalog)
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

fn empty_policy() -> InterceptorPolicy {
    InterceptorPolicy {
        outbound: HashSet::new(),
        inbound: HashSet::new(),
    }
}

fn single_interceptor_catalog(
    name: &str,
    interceptor: Arc<dyn Interceptor>,
    policy: InterceptorPolicy,
    outbound: Vec<&str>,
    inbound: Vec<&str>,
    runtime_limits: Option<InterceptorRuntimeLimitsOverride>,
) -> InterceptorCatalog {
    let mut entries = HashMap::new();
    entries.insert(
        name.to_owned(),
        InterceptorCatalogEntry {
            name: name.to_owned(),
            policy,
            interceptor,
            runtime_limits,
        },
    );

    InterceptorCatalog::new(
        entries,
        ImmutableInterceptorPipeline::new(outbound.into_iter().map(str::to_owned).collect()),
        ImmutableInterceptorPipeline::new(inbound.into_iter().map(str::to_owned).collect()),
    )
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
