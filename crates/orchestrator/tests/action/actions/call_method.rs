use actrpc_core::{
    MethodTarget,
    action::ActionSpec,
    json_rpc::{
        JsonRpcId, JsonRpcMessage, JsonRpcParams, JsonRpcRequest, JsonRpcResponse,
        JsonRpcSingleMessage, JsonRpcSuccessResponse, JsonRpcVersion,
    },
    participant::{Participant, ParticipantType},
};
use actrpc_orchestrator::{
    action::{
        ActionRegistry,
        actions::call_method::{CallMethod, CallMethodHandler},
    },
    error::{ActionExecutionError, ActionHandlerError, MethodCallError},
    interceptor::{ImmutableInterceptorPipeline, InterceptorCatalog},
    method::{
        MethodCatalog, MethodInfo, MethodName, MethodProvider, MethodProviderFuture, ProviderName,
    },
    review::UnavailableReviewProvider,
    runtime::{CallExecutionFactory, CallRuntime, OrchestratorResources},
};
use serde_json::json;
use std::{collections::HashMap, sync::Arc};

use super::super::helpers::{
    action_record, dummy_request, error_message, invocation_context, json_error, request_message,
    success_message,
};

fn test_parent_call(
    transcript: actrpc_orchestrator::runtime::TranscriptState,
    call_id: actrpc_core::CallId,
) -> Arc<CallRuntime> {
    let transcript = Arc::new(transcript);
    transcript.execution_tree().register_root(call_id).unwrap();
    Arc::new(CallRuntime::root(
        request_message("parent", None),
        transcript,
        call_id,
        Participant {
            kind: ParticipantType::External,
            id: "caller".to_owned(),
        },
        MethodTarget {
            provider: "test_provider".to_owned(),
            method: "parent".to_owned(),
        },
    ))
}

struct StaticMethodProvider {
    name: ProviderName,
    methods: Vec<MethodInfo>,
    response: JsonRpcMessage,
}

impl StaticMethodProvider {
    fn new(response: JsonRpcMessage) -> Self {
        Self {
            name: ProviderName::from("test_provider"),
            methods: vec![MethodInfo {
                name: MethodName::from("test_method"),
                description: None,
                info: json!({}),
            }],
            response,
        }
    }
}

impl MethodProvider for StaticMethodProvider {
    fn name(&self) -> &ProviderName {
        &self.name
    }

    fn snapshot(&self) -> actrpc_orchestrator::method::MethodProviderSnapshot {
        actrpc_orchestrator::method::MethodProviderSnapshot {
            provider: self.name.clone(),
            version: None,
            description: None,
            methods: self.methods.clone(),
            info: json!({}),
        }
    }

    fn refresh<'a>(
        &'a self,
    ) -> actrpc_orchestrator::method::MethodProviderFuture<
        'a,
        Result<
            actrpc_orchestrator::method::MethodProviderSnapshot,
            actrpc_orchestrator::error::MethodProviderRefreshError,
        >,
    > {
        let p = self.name.clone();
        Box::pin(async move {
            Err(actrpc_orchestrator::error::MethodProviderRefreshError::Unsupported { provider: p })
        })
    }

    fn request_message(
        &self,
        method: &MethodName,
        params: Option<JsonRpcParams>,
    ) -> Result<JsonRpcMessage, MethodCallError> {
        if self.method(method).is_none() {
            return Err(MethodCallError::MethodNotFound {
                provider: self.name.clone(),
                method: method.clone(),
            });
        }

        Ok(JsonRpcMessage::Single(JsonRpcSingleMessage::Request(
            JsonRpcRequest {
                jsonrpc: JsonRpcVersion::V2_0,
                id: actrpc_core::json_rpc::JsonRpcId::Number(1_u64.into()),
                method: method.as_str().to_owned(),
                params,
            },
        )))
    }

    fn send_message<'a>(
        &'a self,
        _method: &'a MethodName,
        _message: JsonRpcMessage,
    ) -> MethodProviderFuture<'a, Result<JsonRpcMessage, MethodCallError>> {
        Box::pin(async move { Ok(self.response.clone()) })
    }
}

#[tokio::test]
async fn call_method_returns_success_result() {
    let factory = test_factory(success_message(json!({
        "ok": true,
        "value": 42
    })));

    let transcript = actrpc_orchestrator::runtime::TranscriptState::new();
    let call_id = transcript.allocate_call_id();
    let parent_call = test_parent_call(transcript, call_id);

    let mut registry = ActionRegistry::new();
    registry
        .register::<CallMethod, _>(CallMethodHandler::new(factory, parent_call))
        .unwrap();

    let action = action_record::<CallMethod>(json!({
        "provider": "test_provider",
        "method": "test_method",
        "params": {
            "input": 123
        }
    }));

    let resolved = registry
        .get(&CallMethod::action_kind())
        .unwrap()
        .handle(&dummy_request(), action, &invocation_context("test"))
        .await
        .unwrap();

    let response: JsonRpcResponse = serde_json::from_value(
        resolved.result.unwrap().unwrap(),
    )
    .unwrap();
    assert_eq!(
        response,
        JsonRpcResponse::Success(JsonRpcSuccessResponse {
            jsonrpc: JsonRpcVersion::V2_0,
            id: JsonRpcId::Number(1.into()),
            result: json!({
                "ok": true,
                "value": 42
            }),
        })
    );
}

#[tokio::test]
async fn call_method_maps_json_rpc_error_response_to_resolved_json_rpc_response() {
    use actrpc_core::json_rpc::JsonRpcErrorResponse;

    let factory = test_factory(error_message(json_error(-32000, "downstream failed")));

    let transcript = actrpc_orchestrator::runtime::TranscriptState::new();
    let call_id = transcript.allocate_call_id();
    let parent_call = test_parent_call(transcript, call_id);

    let mut registry = ActionRegistry::new();
    registry
        .register::<CallMethod, _>(CallMethodHandler::new(factory, parent_call))
        .unwrap();

    let action = action_record::<CallMethod>(json!({
        "provider": "test_provider",
        "method": "test_method"
    }));

    let resolved = registry
        .get(&CallMethod::action_kind())
        .unwrap()
        .handle(&dummy_request(), action, &invocation_context("test"))
        .await
        .unwrap();

    let response: JsonRpcResponse = serde_json::from_value(
        resolved.result.unwrap().unwrap(),
    )
    .unwrap();
    assert_eq!(
        response,
        JsonRpcResponse::Error(JsonRpcErrorResponse {
            jsonrpc: JsonRpcVersion::V2_0,
            id: JsonRpcId::Number(1.into()),
            error: json_error(-32000, "downstream failed"),
        })
    );
}

#[tokio::test]
async fn call_method_maps_missing_provider_to_call_execution_dependency_failed() {
    let resources = OrchestratorResources::with_review_provider(
        Arc::new(empty_interceptor_catalog()),
        Arc::new(MethodCatalog::new(HashMap::new())),
        Arc::new(UnavailableReviewProvider),
        vec![],
    );

    let factory = Arc::new(CallExecutionFactory::new(Arc::new(resources)));
    let transcript = actrpc_orchestrator::runtime::TranscriptState::new();
    let call_id = transcript.allocate_call_id();
    let parent_call = test_parent_call(transcript, call_id);

    let mut registry = ActionRegistry::new();
    registry
        .register::<CallMethod, _>(CallMethodHandler::new(factory, parent_call))
        .unwrap();

    let action = action_record::<CallMethod>(json!({
        "provider": "missing_provider",
        "method": "test_method"
    }));

    let err = registry
        .get(&CallMethod::action_kind())
        .unwrap()
        .handle(&dummy_request(), action, &invocation_context("test"))
        .await
        .unwrap_err();

    match err {
        ActionHandlerError::Execution(ActionExecutionError::DependencyFailed {
            dependency,
            message,
        }) => {
            assert_eq!(dependency, "call_execution");
            assert!(message.contains("method provider not found"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

fn test_factory(response: JsonRpcMessage) -> Arc<CallExecutionFactory> {
    let provider = Arc::new(StaticMethodProvider::new(response)) as Arc<dyn MethodProvider>;

    let mut providers = HashMap::new();
    providers.insert(ProviderName::from("test_provider"), provider);

    let resources = OrchestratorResources::with_review_provider(
        Arc::new(empty_interceptor_catalog()),
        Arc::new(MethodCatalog::new(providers)),
        Arc::new(UnavailableReviewProvider),
        vec![],
    );

    Arc::new(CallExecutionFactory::new(Arc::new(resources)))
}

fn empty_interceptor_catalog() -> InterceptorCatalog {
    InterceptorCatalog::new(
        HashMap::new(),
        ImmutableInterceptorPipeline::new(vec![]),
        ImmutableInterceptorPipeline::new(vec![]),
    )
}
