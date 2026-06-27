mod common;

use actrpc_core::{
    action::ActionSpec,
    json_rpc::{
        JsonRpcId, JsonRpcMessage, JsonRpcParams, JsonRpcRequest, JsonRpcResponse,
        JsonRpcSingleMessage, JsonRpcSuccessResponse, JsonRpcVersion,
    },
};
use actrpc_interceptor::interceptors::{
    call_request::{CallRequestExecutor, ExecutorConfig},
    dynamic_policy::{
        DynamicPolicyConfig, DynamicPolicyInterceptor, DynamicPolicyStore, UnscopedBehavior,
        UnscopedPolicy,
    },
};
use actrpc_orchestrator::{
    action::actions::{
        call_method::CallMethod, modify_result::ModifyResult,
        query_execution_context::QueryExecutionContext, reject_call::RejectCall,
        request_review::RequestReview,
    },
    interceptor::{
        ImmutableInterceptorPipeline, Interceptor, InterceptorCatalog, InterceptorCatalogEntry,
        InterceptorPolicy,
    },
    method::{
        MethodCatalog, MethodInfo, MethodName, MethodProvider, MethodProviderFuture,
        MethodProviderSnapshot, ProviderName,
    },
    review::UnavailableReviewProvider,
    runtime::{CallExecutionFactory, OrchestratorResources},
};
use common::support::method_target;
use serde_json::json;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

const AGENTS_PROVIDER: &str = "agents";
const TOOLS_PROVIDER: &str = "tools";

struct StaticMethodProvider {
    name: ProviderName,
    methods: Vec<MethodInfo>,
    response: JsonRpcMessage,
}

impl StaticMethodProvider {
    fn new(name: &str, method: &str, response: JsonRpcMessage) -> Self {
        Self {
            name: ProviderName::from(name),
            methods: vec![MethodInfo {
                name: MethodName::from(method),
                description: None,
                params_schema: None,
                result_schema: None,
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

    fn snapshot(&self) -> MethodProviderSnapshot {
        MethodProviderSnapshot {
            provider: self.name.clone(),
            version: None,
            description: None,
            methods: self.methods.clone(),
            info: json!({}),
        }
    }

    fn refresh<'a>(
        &'a self,
    ) -> MethodProviderFuture<
        'a,
        Result<MethodProviderSnapshot, actrpc_orchestrator::error::MethodProviderRefreshError>,
    > {
        let provider = self.name.clone();
        Box::pin(async move {
            Err(actrpc_orchestrator::error::MethodProviderRefreshError::Unsupported { provider })
        })
    }

    fn request_message(
        &self,
        method: &MethodName,
        params: Option<JsonRpcParams>,
    ) -> Result<JsonRpcMessage, actrpc_orchestrator::error::MethodCallError> {
        if self.method(method).is_none() {
            return Err(
                actrpc_orchestrator::error::MethodCallError::MethodNotFound {
                    provider: self.name.clone(),
                    method: method.clone(),
                },
            );
        }

        Ok(JsonRpcMessage::Single(JsonRpcSingleMessage::Request(
            JsonRpcRequest {
                jsonrpc: JsonRpcVersion::V2_0,
                id: JsonRpcId::Number(1.into()),
                method: method.as_str().to_owned(),
                params,
            },
        )))
    }

    fn send_message<'a>(
        &'a self,
        _method: &'a MethodName,
        _message: JsonRpcMessage,
    ) -> MethodProviderFuture<'a, Result<JsonRpcMessage, actrpc_orchestrator::error::MethodCallError>>
    {
        Box::pin(async move { Ok(self.response.clone()) })
    }
}

#[tokio::test]
async fn executor_nested_call_rejected_by_dynamic_policy_surfaces_error_in_parent_result() {
    let dynamic_store = DynamicPolicyStore::shared();
    let dynamic_policy = Arc::new(DynamicPolicyInterceptor::new(
        dynamic_store,
        DynamicPolicyConfig {
            unscoped_policy: UnscopedPolicy {
                on_unscoped: UnscopedBehavior::ScopeRoot,
                allowed_method_targets: vec![method_target("tools", "read")],
            },
        },
    ));
    let call_request_executor = Arc::new(CallRequestExecutor::new(ExecutorConfig {
        version: 1,
        call_requests_field: "_actrpc_call_requests".to_owned(),
        results_field: "_actrpc_call_results".to_owned(),
    }));

    let catalog = dual_interceptor_catalog(dynamic_policy, call_request_executor);
    let factory = test_factory(catalog).await;

    let execution = factory
        .create_root(
            ProviderName::from(AGENTS_PROVIDER),
            MethodName::from("invoke"),
            None,
            "caller",
        )
        .unwrap();

    let response = execution.run().await.expect("root call should succeed");

    let JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Success(success))) =
        response
    else {
        panic!("expected success response");
    };

    let results = &success.result["_actrpc_call_results"];
    assert_eq!(results[0]["request"]["target"], "tools::write");
    assert_eq!(results[0]["response"]["jsonrpc"], "2.0");
    assert_eq!(results[0]["response"]["error"]["code"], -32011);
    assert_eq!(
        results[0]["response"]["error"]["message"].as_str().unwrap(),
        "method not allowed by dynamic scope"
    );
    assert!(results[0].get("error").is_none());
}

fn dual_interceptor_catalog(
    dynamic_policy: Arc<DynamicPolicyInterceptor>,
    call_request_executor: Arc<CallRequestExecutor>,
) -> InterceptorCatalog {
    let dynamic_policy_policy = InterceptorPolicy {
        outbound: HashSet::from([
            QueryExecutionContext::action_kind(),
            RejectCall::action_kind(),
            RequestReview::action_kind(),
        ]),
        inbound: HashSet::from([QueryExecutionContext::action_kind()]),
    };
    let call_request_policy = InterceptorPolicy {
        outbound: HashSet::new(),
        inbound: HashSet::from([CallMethod::action_kind(), ModifyResult::action_kind()]),
    };

    let mut entries = HashMap::new();
    entries.insert(
        "dynamic_policy".to_owned(),
        InterceptorCatalogEntry {
            name: "dynamic_policy".to_owned(),
            policy: dynamic_policy_policy,
            interceptor: dynamic_policy as Arc<dyn Interceptor>,
            runtime_limits: None,
        },
    );
    entries.insert(
        "call_request_executor".to_owned(),
        InterceptorCatalogEntry {
            name: "call_request_executor".to_owned(),
            policy: call_request_policy,
            interceptor: call_request_executor as Arc<dyn Interceptor>,
            runtime_limits: None,
        },
    );

    InterceptorCatalog::new(
        entries,
        ImmutableInterceptorPipeline::new(vec!["dynamic_policy".to_owned()]),
        ImmutableInterceptorPipeline::new(vec![
            "call_request_executor".to_owned(),
            "dynamic_policy".to_owned(),
        ]),
    )
}

async fn test_factory(catalog: InterceptorCatalog) -> Arc<CallExecutionFactory> {
    let mut providers = HashMap::new();
    providers.insert(
        ProviderName::from(AGENTS_PROVIDER),
        Arc::new(StaticMethodProvider::new(
            AGENTS_PROVIDER,
            "invoke",
            success_message(json!({
                "answer": "planned",
                "_actrpc_call_requests": [{
                    "target": "tools::write"
                }]
            })),
        )) as Arc<dyn MethodProvider>,
    );
    providers.insert(
        ProviderName::from(TOOLS_PROVIDER),
        Arc::new(StaticMethodProvider::new(
            TOOLS_PROVIDER,
            "write",
            success_message(json!({ "ok": true })),
        )) as Arc<dyn MethodProvider>,
    );

    let resources = Arc::new(OrchestratorResources::with_review_provider(
        Arc::new(catalog),
        Arc::new(MethodCatalog::new(providers)),
        Arc::new(UnavailableReviewProvider),
        vec![],
    ));

    Arc::new(CallExecutionFactory::new(resources))
}

fn success_message(result: serde_json::Value) -> JsonRpcMessage {
    JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Success(
        JsonRpcSuccessResponse {
            jsonrpc: JsonRpcVersion::V2_0,
            id: JsonRpcId::Number(1.into()),
            result,
        },
    )))
}
