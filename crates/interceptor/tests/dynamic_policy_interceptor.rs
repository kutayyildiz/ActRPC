mod common;

use actrpc_core::{
    CallId, CurrentExecutionContext, InterceptionId, MethodTarget,
    action::{ActionSpec, ResolvedActionRecord},
    interception::{InterceptionRequest, InterceptorContinuation},
    json_rpc::{
        JsonRpcMessage, JsonRpcParams, JsonRpcRequest, JsonRpcSingleMessage, JsonRpcVersion,
    },
};
use actrpc_interceptor::interceptors::dynamic_policy::{
    DynamicPolicyConfig, DynamicPolicyStore, UnscopedBehavior, UnscopedPolicy,
    new_component_with_config,
};
use actrpc_orchestrator::{
    action::actions::{reject_call::RejectCall, request_review::RequestReview},
    interceptor::Interceptor,
};
use common::support::{
    current_context, default_target, external_origin, method_target, resolved_query_current,
};
use serde_json::json;

fn outbound_request(
    current: CurrentExecutionContext,
    history: Vec<Vec<actrpc_core::action::ResolvedActionRecord>>,
    ctx: actrpc_core::InterceptionContext,
) -> InterceptionRequest {
    InterceptionRequest {
        origin: external_origin("caller"),
        target: current.target.clone(),
        message: JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
            jsonrpc: JsonRpcVersion::V2_0,
            id: actrpc_core::json_rpc::JsonRpcId::Number(1.into()),
            method: current.target.method.clone(),
            params: Some(JsonRpcParams::Object(serde_json::Map::new())),
        })),
        call_id: current.call_id,
        interception_id: InterceptionId::new(),
        resolved_action_history: history,
        ctx,
    }
}

fn detached_ctx(allowed: Vec<MethodTarget>) -> actrpc_core::InterceptionContext {
    actrpc_core::InterceptionContext {
        private: Some(json!({
            "mode": "detached",
            "allowed_method_targets": allowed,
        })),
        ..Default::default()
    }
}

fn seed_parent_scope(
    store: &DynamicPolicyStore,
    parent_call_id: CallId,
    root_call_id: CallId,
    allowed: Vec<MethodTarget>,
) -> actrpc_interceptor::interceptors::dynamic_policy::ScopeId {
    let scope_id = store
        .create_scope_for_call(parent_call_id, root_call_id, allowed)
        .unwrap();
    store.bind_call(parent_call_id, scope_id);
    scope_id
}

fn review_key(call_id: CallId) -> String {
    format!("dynamic_policy:detached:{call_id}")
}

fn resolved_request_review(rule_name: &str, decision: &str) -> ResolvedActionRecord {
    ResolvedActionRecord {
        kind: RequestReview::action_kind(),
        params: Some(json!({
            "rule_name": rule_name,
            "title": "Dynamic policy detached scope expansion",
            "reason": "review",
            "severity": "medium"
        })),
        result: Ok(Some(json!({ "decision": decision }))),
    }
}

#[tokio::test]
async fn no_ctx_no_parent_allow_unscoped() {
    let component = new_component_with_config(DynamicPolicyConfig::default());
    let current = current_context(CallId::new(), CallId::new(), None, default_target("run"));
    let response = component
        .interceptor
        .intercept(&outbound_request(
            current.clone(),
            vec![vec![resolved_query_current(current.clone())]],
            Default::default(),
        ))
        .await
        .unwrap();

    assert!(response.actions.is_empty());
    assert_eq!(response.continuation, InterceptorContinuation::Stop);
    assert!(component.store.scope_for_call(current.call_id).is_none());
}

#[tokio::test]
async fn no_ctx_no_parent_reject_unscoped() {
    let config = DynamicPolicyConfig {
        unscoped_policy: UnscopedPolicy {
            on_unscoped: UnscopedBehavior::Reject,
            allowed_method_targets: vec![],
        },
    };
    let component = new_component_with_config(config);
    let current = current_context(CallId::new(), CallId::new(), None, default_target("run"));
    let response = component
        .interceptor
        .intercept(&outbound_request(
            current.clone(),
            vec![vec![resolved_query_current(current.clone())]],
            Default::default(),
        ))
        .await
        .unwrap();

    assert_eq!(response.actions[0].kind, RejectCall::action_kind());
}

#[tokio::test]
async fn no_ctx_no_parent_scope_root_creates_configured_scope() {
    let config = DynamicPolicyConfig {
        unscoped_policy: UnscopedPolicy {
            on_unscoped: UnscopedBehavior::ScopeRoot,
            allowed_method_targets: vec![method_target("tools", "read")],
        },
    };
    let component = new_component_with_config(config);
    let call_id = CallId::new();
    let root = call_id;
    let current = current_context(call_id, root, None, method_target("tools", "read"));
    let response = component
        .interceptor
        .intercept(&outbound_request(
            current.clone(),
            vec![vec![resolved_query_current(current.clone())]],
            Default::default(),
        ))
        .await
        .unwrap();

    assert!(response.actions.is_empty());
    assert!(component.store.scope_for_call(call_id).is_some());
}

#[tokio::test]
async fn scope_root_child_allowed_inherits_and_allows() {
    let config = DynamicPolicyConfig {
        unscoped_policy: UnscopedPolicy {
            on_unscoped: UnscopedBehavior::ScopeRoot,
            allowed_method_targets: vec![method_target("tools", "read")],
        },
    };
    let component = new_component_with_config(config);
    let root = CallId::new();
    let child = CallId::new();

    let root_current = current_context(root, root, None, method_target("tools", "read"));
    component
        .interceptor
        .intercept(&outbound_request(
            root_current.clone(),
            vec![vec![resolved_query_current(root_current.clone())]],
            Default::default(),
        ))
        .await
        .unwrap();

    let child_current = current_context(child, root, Some(root), method_target("tools", "read"));
    let response = component
        .interceptor
        .intercept(&outbound_request(
            child_current.clone(),
            vec![vec![resolved_query_current(child_current.clone())]],
            Default::default(),
        ))
        .await
        .unwrap();

    assert!(response.actions.is_empty());
    assert_eq!(
        component.store.scope_for_call(child),
        component.store.scope_for_call(root)
    );
}

#[tokio::test]
async fn scope_root_child_denied_rejects() {
    let config = DynamicPolicyConfig {
        unscoped_policy: UnscopedPolicy {
            on_unscoped: UnscopedBehavior::ScopeRoot,
            allowed_method_targets: vec![method_target("tools", "read")],
        },
    };
    let component = new_component_with_config(config);
    let root = CallId::new();
    let child = CallId::new();

    let root_current = current_context(root, root, None, method_target("tools", "read"));
    component
        .interceptor
        .intercept(&outbound_request(
            root_current.clone(),
            vec![vec![resolved_query_current(root_current.clone())]],
            Default::default(),
        ))
        .await
        .unwrap();

    let child_current = current_context(child, root, Some(root), method_target("tools", "write"));
    let response = component
        .interceptor
        .intercept(&outbound_request(
            child_current.clone(),
            vec![vec![resolved_query_current(child_current.clone())]],
            Default::default(),
        ))
        .await
        .unwrap();

    assert_eq!(response.actions[0].kind, RejectCall::action_kind());
    assert!(component.store.scope_for_call(child).is_none());
}

#[tokio::test]
async fn no_ctx_parent_scope_allowed_binds() {
    let component = new_component_with_config(DynamicPolicyConfig::default());
    let root = CallId::new();
    let parent = CallId::new();
    let child = CallId::new();
    seed_parent_scope(
        &component.store,
        parent,
        root,
        vec![method_target("test-provider", "child_method")],
    );

    let current = current_context(
        child,
        root,
        Some(parent),
        method_target("test-provider", "child_method"),
    );
    let response = component
        .interceptor
        .intercept(&outbound_request(
            current.clone(),
            vec![vec![resolved_query_current(current.clone())]],
            Default::default(),
        ))
        .await
        .unwrap();

    assert!(response.actions.is_empty());
    assert_eq!(
        component.store.scope_for_call(child),
        component.store.scope_for_call(parent)
    );
}

#[tokio::test]
async fn no_ctx_parent_scope_denied_rejects_without_bind() {
    let component = new_component_with_config(DynamicPolicyConfig::default());
    let root = CallId::new();
    let parent = CallId::new();
    let child = CallId::new();
    seed_parent_scope(
        &component.store,
        parent,
        root,
        vec![method_target("test-provider", "allowed")],
    );

    let current = current_context(
        child,
        root,
        Some(parent),
        method_target("test-provider", "denied"),
    );
    let response = component
        .interceptor
        .intercept(&outbound_request(
            current.clone(),
            vec![vec![resolved_query_current(current.clone())]],
            Default::default(),
        ))
        .await
        .unwrap();

    assert_eq!(response.actions[0].kind, RejectCall::action_kind());
    assert!(component.store.scope_for_call(child).is_none());
}

#[tokio::test]
async fn valid_detached_no_parent_creates_scope() {
    let component = new_component_with_config(DynamicPolicyConfig::default());
    let call_id = CallId::new();
    let root = CallId::new();
    let current = current_context(call_id, root, None, default_target("run"));
    let response = component
        .interceptor
        .intercept(&outbound_request(
            current.clone(),
            vec![vec![resolved_query_current(current.clone())]],
            detached_ctx(vec![method_target("filesystem", "read_file")]),
        ))
        .await
        .unwrap();

    assert!(response.actions.is_empty());
    assert!(component.store.scope_for_call(call_id).is_some());
}

#[tokio::test]
async fn valid_detached_parent_subset_creates_new_scope() {
    let component = new_component_with_config(DynamicPolicyConfig::default());
    let root = CallId::new();
    let parent = CallId::new();
    let child = CallId::new();
    seed_parent_scope(
        &component.store,
        parent,
        root,
        vec![
            method_target("filesystem", "read_file"),
            method_target("filesystem", "write_file"),
        ],
    );

    let current = current_context(
        child,
        root,
        Some(parent),
        method_target("filesystem", "read_file"),
    );
    let response = component
        .interceptor
        .intercept(&outbound_request(
            current.clone(),
            vec![vec![resolved_query_current(current.clone())]],
            detached_ctx(vec![method_target("filesystem", "read_file")]),
        ))
        .await
        .unwrap();

    assert!(response.actions.is_empty());
    let child_scope = component.store.scope_for_call(child).unwrap();
    let parent_scope = component.store.scope_for_call(parent).unwrap();
    assert_ne!(child_scope, parent_scope);
}

#[tokio::test]
async fn valid_detached_parent_target_denied_rejects_without_review() {
    let component = new_component_with_config(DynamicPolicyConfig::default());
    let root = CallId::new();
    let parent = CallId::new();
    let child = CallId::new();
    seed_parent_scope(
        &component.store,
        parent,
        root,
        vec![method_target("filesystem", "read_file")],
    );

    let current = current_context(
        child,
        root,
        Some(parent),
        method_target("filesystem", "write_file"),
    );
    let response = component
        .interceptor
        .intercept(&outbound_request(
            current.clone(),
            vec![vec![resolved_query_current(current.clone())]],
            detached_ctx(vec![method_target("filesystem", "read_file")]),
        ))
        .await
        .unwrap();

    assert_eq!(response.actions[0].kind, RejectCall::action_kind());
    assert_ne!(response.actions[0].kind, RequestReview::action_kind());
    assert!(component.store.scope_for_call(child).is_none());
    assert!(component.store.scope_created_by_call(child).is_none());
}

#[tokio::test]
async fn valid_detached_parent_superset_requests_review() {
    let component = new_component_with_config(DynamicPolicyConfig::default());
    let root = CallId::new();
    let parent = CallId::new();
    let child = CallId::new();
    seed_parent_scope(
        &component.store,
        parent,
        root,
        vec![method_target("filesystem", "read_file")],
    );

    let current = current_context(
        child,
        root,
        Some(parent),
        method_target("filesystem", "read_file"),
    );
    let response = component
        .interceptor
        .intercept(&outbound_request(
            current.clone(),
            vec![vec![resolved_query_current(current.clone())]],
            detached_ctx(vec![
                method_target("filesystem", "read_file"),
                method_target("filesystem", "write_file"),
            ]),
        ))
        .await
        .unwrap();

    assert_eq!(response.actions[0].kind, RequestReview::action_kind());
}

#[tokio::test]
async fn approved_review_creates_and_binds_detached_scope() {
    let component = new_component_with_config(DynamicPolicyConfig::default());
    let root = CallId::new();
    let parent = CallId::new();
    let child = CallId::new();
    seed_parent_scope(
        &component.store,
        parent,
        root,
        vec![method_target("filesystem", "read_file")],
    );

    let current = current_context(
        child,
        root,
        Some(parent),
        method_target("filesystem", "read_file"),
    );
    let history = vec![vec![
        resolved_query_current(current.clone()),
        resolved_request_review(&review_key(child), "approved"),
    ]];

    let response = component
        .interceptor
        .intercept(&outbound_request(
            current.clone(),
            history,
            detached_ctx(vec![
                method_target("filesystem", "read_file"),
                method_target("filesystem", "write_file"),
            ]),
        ))
        .await
        .unwrap();

    assert!(response.actions.is_empty());
    assert!(component.store.scope_for_call(child).is_some());
}

#[tokio::test]
async fn denied_review_rejects_call() {
    let component = new_component_with_config(DynamicPolicyConfig::default());
    let root = CallId::new();
    let parent = CallId::new();
    let child = CallId::new();
    seed_parent_scope(
        &component.store,
        parent,
        root,
        vec![method_target("filesystem", "read_file")],
    );

    let current = current_context(
        child,
        root,
        Some(parent),
        method_target("filesystem", "read_file"),
    );
    let history = vec![vec![
        resolved_query_current(current.clone()),
        resolved_request_review(&review_key(child), "denied"),
    ]];

    let response = component
        .interceptor
        .intercept(&outbound_request(
            current.clone(),
            history,
            detached_ctx(vec![
                method_target("filesystem", "read_file"),
                method_target("filesystem", "write_file"),
            ]),
        ))
        .await
        .unwrap();

    assert_eq!(response.actions[0].kind, RejectCall::action_kind());
    assert!(component.store.scope_for_call(child).is_none());
}

#[tokio::test]
async fn malformed_ctx_no_parent_rejects() {
    let component = new_component_with_config(DynamicPolicyConfig::default());
    let current = current_context(CallId::new(), CallId::new(), None, default_target("run"));
    let response = component
        .interceptor
        .intercept(&outbound_request(
            current.clone(),
            vec![vec![resolved_query_current(current.clone())]],
            actrpc_core::InterceptionContext {
                private: Some(json!({ "mode": "unknown" })),
                ..Default::default()
            },
        ))
        .await
        .unwrap();

    assert_eq!(response.actions[0].kind, RejectCall::action_kind());
}

#[tokio::test]
async fn malformed_ctx_parent_allowed_inherits_parent() {
    let component = new_component_with_config(DynamicPolicyConfig::default());
    let root = CallId::new();
    let parent = CallId::new();
    let child = CallId::new();
    seed_parent_scope(
        &component.store,
        parent,
        root,
        vec![method_target("test-provider", "child_method")],
    );

    let current = current_context(
        child,
        root,
        Some(parent),
        method_target("test-provider", "child_method"),
    );
    let response = component
        .interceptor
        .intercept(&outbound_request(
            current.clone(),
            vec![vec![resolved_query_current(current.clone())]],
            actrpc_core::InterceptionContext {
                private: Some(json!({ "unexpected": true })),
                ..Default::default()
            },
        ))
        .await
        .unwrap();

    assert!(response.actions.is_empty());
    assert_eq!(
        component.store.scope_for_call(child),
        component.store.scope_for_call(parent)
    );
}

#[tokio::test]
async fn inbound_releases_call_and_created_scope() {
    let component = new_component_with_config(DynamicPolicyConfig::default());
    let call_id = CallId::new();
    let root = CallId::new();
    let scope_id = component
        .store
        .create_scope_for_call(call_id, root, vec![method_target("agents", "invoke")])
        .unwrap();
    component.store.bind_call(call_id, scope_id);

    let current = current_context(call_id, root, None, default_target("run"));
    let request = InterceptionRequest {
        origin: external_origin("caller"),
        target: current.target.clone(),
        message: JsonRpcMessage::Single(JsonRpcSingleMessage::Response(
            actrpc_core::json_rpc::JsonRpcResponse::Success(
                actrpc_core::json_rpc::JsonRpcSuccessResponse {
                    jsonrpc: JsonRpcVersion::V2_0,
                    id: actrpc_core::json_rpc::JsonRpcId::Number(1.into()),
                    result: json!("ok"),
                },
            ),
        )),
        call_id,
        interception_id: InterceptionId::new(),
        resolved_action_history: vec![vec![resolved_query_current(current.clone())]],
        ctx: Default::default(),
    };

    let response = component.interceptor.intercept(&request).await.unwrap();
    assert!(response.actions.is_empty());
    assert!(component.store.scope_for_call(call_id).is_none());
    assert!(component.store.get_scope(scope_id).is_none());
}

#[tokio::test]
async fn root_inbound_cleanup_removes_remaining_scopes() {
    let component = new_component_with_config(DynamicPolicyConfig::default());
    let root = CallId::new();
    let child = CallId::new();

    let root_scope = component
        .store
        .create_scope_for_call(root, root, vec![method_target("tools", "read")])
        .unwrap();
    component.store.bind_call(root, root_scope);

    let child_scope = component
        .store
        .create_scope_for_call(child, root, vec![method_target("tools", "write")])
        .unwrap();
    component.store.bind_call(child, child_scope);

    let current = current_context(root, root, None, default_target("run"));
    let request = InterceptionRequest {
        origin: external_origin("caller"),
        target: current.target.clone(),
        message: JsonRpcMessage::Single(JsonRpcSingleMessage::Response(
            actrpc_core::json_rpc::JsonRpcResponse::Success(
                actrpc_core::json_rpc::JsonRpcSuccessResponse {
                    jsonrpc: JsonRpcVersion::V2_0,
                    id: actrpc_core::json_rpc::JsonRpcId::Number(1.into()),
                    result: json!("ok"),
                },
            ),
        )),
        call_id: root,
        interception_id: InterceptionId::new(),
        resolved_action_history: vec![vec![resolved_query_current(current.clone())]],
        ctx: Default::default(),
    };

    component.interceptor.intercept(&request).await.unwrap();

    assert!(component.store.get_scope(root_scope).is_none());
    assert!(component.store.get_scope(child_scope).is_none());
    assert!(component.store.scope_for_call(child).is_none());
}

