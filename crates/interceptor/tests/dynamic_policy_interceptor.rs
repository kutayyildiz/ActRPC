mod support;

use actrpc_core::{
    CallId, CallRelation,
    action::ActionKind,
    interception::InterceptionRequest,
    interception::InterceptorContinuation,
    json_rpc::{JsonRpcMessage, JsonRpcRequest, JsonRpcSingleMessage, JsonRpcVersion},
};
use actrpc_interceptor::interceptors::dynamic_policy::{
    CreateScopeParams, ListScopesParams, RelationMode, TargetSelector, provider::call_method,
};
use actrpc_orchestrator::interceptor::Interceptor;
use serde_json::json;
use support::{
    current_context, default_target, dynamic_policy_fixture, interceptor_origin, method_target,
    resolved_query_current, resolved_query_relation, sample_request,
};

fn make_request(
    call_id: CallId,
    target: actrpc_core::MethodTarget,
    history: Vec<Vec<actrpc_core::action::ResolvedActionRecord>>,
) -> InterceptionRequest {
    let mut request = sample_request(
        interceptor_origin("planner"),
        target,
        JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
            jsonrpc: JsonRpcVersion::V2_0,
            id: actrpc_core::json_rpc::JsonRpcId::Number(1_u64.into()),
            method: "noop".to_owned(),
            params: None,
        })),
        history,
    );
    request.call_id = call_id;
    request
}

#[tokio::test]
async fn missing_current_query_requests_current_and_reinvokes() {
    let component = dynamic_policy_fixture();
    let call_id = CallId::new();
    let request = make_request(call_id, default_target("method_1"), vec![]);

    let response = component.interceptor.intercept(&request).await.unwrap();

    assert_eq!(response.continuation, InterceptorContinuation::Reinvoke);
    assert_eq!(response.actions.len(), 1);
    assert_eq!(
        response.actions[0].kind,
        ActionKind::from("query_execution_context")
    );
    assert_eq!(
        response.actions[0].params,
        Some(json!({ "query": { "kind": "current" } }))
    );
}

#[tokio::test]
async fn no_dynamic_scope_means_no_actions() {
    let component = dynamic_policy_fixture();
    let root = CallId::new();
    let call_id = CallId::new();

    let current = current_context(call_id, root, None, default_target("method_1"));

    let request = make_request(
        call_id,
        default_target("method_1"),
        vec![vec![resolved_query_current(current)]],
    );

    let response = component.interceptor.intercept(&request).await.unwrap();

    assert_eq!(response.continuation, InterceptorContinuation::Stop);
    assert!(response.actions.is_empty());
}

#[tokio::test]
async fn principal_binding_call_is_allowed_even_when_not_in_allowlist() {
    let component = dynamic_policy_fixture();
    let root = CallId::new();
    let principal = CallId::new();

    call_method(
        &component.provider,
        "create_scope",
        Some(
            serde_json::to_value(CreateScopeParams {
                owner_call_id: root,
                root_call_id: Some(root),
                creator: interceptor_origin("planner"),
                target_selector: TargetSelector {
                    provider: "tools".to_owned(),
                    method: "agent_x".to_owned(),
                },
                allowed_method_targets: vec![
                    method_target("demo", "method_1"),
                    method_target("demo", "method_2"),
                ],
                relation_mode: RelationMode::DirectChild,
                label: None,
            })
            .unwrap(),
        ),
    )
    .await
    .unwrap();

    let principal_target = method_target("tools", "agent_x");
    let current = current_context(principal, root, Some(root), principal_target.clone());

    let request = make_request(
        principal,
        principal_target,
        vec![vec![resolved_query_current(current)]],
    );

    let response = component.interceptor.intercept(&request).await.unwrap();

    assert_eq!(response.continuation, InterceptorContinuation::Stop);
    assert!(response.actions.is_empty());

    let scope = component
        .store
        .list_scopes(ListScopesParams {
            owner_call_id: Some(root),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(scope.scopes[0].bound_call_id, Some(principal));
}

#[tokio::test]
async fn descendant_outside_scope_is_rejected() {
    let component = dynamic_policy_fixture();
    let root = CallId::new();
    let principal = CallId::new();
    let descendant = CallId::new();

    call_method(
        &component.provider,
        "create_scope",
        Some(
            serde_json::to_value(CreateScopeParams {
                owner_call_id: root,
                root_call_id: Some(root),
                creator: interceptor_origin("planner"),
                target_selector: TargetSelector {
                    provider: "tools".to_owned(),
                    method: "agent_x".to_owned(),
                },
                allowed_method_targets: vec![
                    method_target("demo", "method_1"),
                    method_target("demo", "method_2"),
                ],
                relation_mode: RelationMode::DirectChild,
                label: None,
            })
            .unwrap(),
        ),
    )
    .await
    .unwrap();

    component.store.bind_scope(
        component
            .store
            .list_scopes(Default::default())
            .unwrap()
            .scopes[0]
            .scope_id,
        principal,
    );

    let descendant_target = method_target("demo", "method_7");
    let current = current_context(descendant, root, Some(principal), descendant_target.clone());

    let request = make_request(
        descendant,
        descendant_target,
        vec![vec![
            resolved_query_current(current),
            resolved_query_relation(descendant, principal, CallRelation::Parent),
        ]],
    );

    let response = component.interceptor.intercept(&request).await.unwrap();

    assert_eq!(response.continuation, InterceptorContinuation::Stop);
    assert_eq!(response.actions.len(), 1);
    assert_eq!(response.actions[0].kind, ActionKind::from("reject_call"));
}

#[tokio::test]
async fn descendant_inside_scope_is_allowed() {
    let component = dynamic_policy_fixture();
    let root = CallId::new();
    let principal = CallId::new();
    let descendant = CallId::new();

    call_method(
        &component.provider,
        "create_scope",
        Some(
            serde_json::to_value(CreateScopeParams {
                owner_call_id: root,
                root_call_id: Some(root),
                creator: interceptor_origin("planner"),
                target_selector: TargetSelector {
                    provider: "tools".to_owned(),
                    method: "agent_x".to_owned(),
                },
                allowed_method_targets: vec![method_target("demo", "method_3")],
                relation_mode: RelationMode::DirectChild,
                label: None,
            })
            .unwrap(),
        ),
    )
    .await
    .unwrap();

    let scope_id = component
        .store
        .list_scopes(Default::default())
        .unwrap()
        .scopes[0]
        .scope_id;
    component.store.bind_scope(scope_id, principal);

    let descendant_target = method_target("demo", "method_3");
    let current = current_context(descendant, root, Some(principal), descendant_target.clone());

    let request = make_request(
        descendant,
        descendant_target,
        vec![vec![
            resolved_query_current(current),
            resolved_query_relation(descendant, principal, CallRelation::Parent),
        ]],
    );

    let response = component.interceptor.intercept(&request).await.unwrap();

    assert_eq!(response.continuation, InterceptorContinuation::Stop);
    assert!(response.actions.is_empty());
}

#[tokio::test]
async fn relation_query_direction_uses_subject_other_semantics() {
    let component = dynamic_policy_fixture();
    let root = CallId::new();
    let principal = CallId::new();
    let descendant = CallId::new();

    call_method(
        &component.provider,
        "create_scope",
        Some(
            serde_json::to_value(CreateScopeParams {
                owner_call_id: root,
                root_call_id: Some(root),
                creator: interceptor_origin("planner"),
                target_selector: TargetSelector {
                    provider: "tools".to_owned(),
                    method: "agent_x".to_owned(),
                },
                allowed_method_targets: vec![method_target("demo", "method_1")],
                relation_mode: RelationMode::DirectChild,
                label: None,
            })
            .unwrap(),
        ),
    )
    .await
    .unwrap();

    component.store.bind_scope(
        component
            .store
            .list_scopes(Default::default())
            .unwrap()
            .scopes[0]
            .scope_id,
        principal,
    );

    let current = current_context(
        descendant,
        root,
        Some(principal),
        method_target("demo", "method_7"),
    );

    let wrong_direction = make_request(
        descendant,
        method_target("demo", "method_7"),
        vec![vec![
            resolved_query_current(current.clone()),
            resolved_query_relation(principal, descendant, CallRelation::Child),
        ]],
    );

    let wrong_response = component
        .interceptor
        .intercept(&wrong_direction)
        .await
        .unwrap();
    assert!(
        wrong_response
            .actions
            .iter()
            .all(|action| action.kind != ActionKind::from("reject_call")),
        "wrong relation direction must not trigger rejection"
    );

    let correct = make_request(
        descendant,
        method_target("demo", "method_7"),
        vec![vec![
            resolved_query_current(current),
            resolved_query_relation(descendant, principal, CallRelation::Parent),
        ]],
    );

    let correct_response = component.interceptor.intercept(&correct).await.unwrap();
    assert_eq!(
        correct_response.actions[0].kind,
        ActionKind::from("reject_call")
    );
}

#[tokio::test]
async fn unrelated_owner_scope_does_not_bind_or_enforce() {
    let component = dynamic_policy_fixture();
    let root = CallId::new();
    let unrelated_owner = CallId::new();
    let current_call = CallId::new();

    call_method(
        &component.provider,
        "create_scope",
        Some(
            serde_json::to_value(CreateScopeParams {
                owner_call_id: unrelated_owner,
                root_call_id: Some(root),
                creator: interceptor_origin("planner"),
                target_selector: TargetSelector {
                    provider: "tools".to_owned(),
                    method: "agent_x".to_owned(),
                },
                allowed_method_targets: vec![method_target("demo", "method_1")],
                relation_mode: RelationMode::DirectChild,
                label: None,
            })
            .unwrap(),
        ),
    )
    .await
    .unwrap();

    let principal_target = method_target("tools", "agent_x");
    let current = current_context(current_call, root, Some(root), principal_target.clone());

    let request = make_request(
        current_call,
        principal_target,
        vec![vec![resolved_query_current(current)]],
    );

    let response = component.interceptor.intercept(&request).await.unwrap();

    assert!(response.actions.is_empty());

    let scope = component.store.list_scopes(Default::default()).unwrap();
    assert!(scope.scopes[0].bound_call_id.is_none());
}

#[tokio::test]
async fn unbound_descendant_scope_with_nonmatching_selector_does_not_request_relation() {
    let component = dynamic_policy_fixture();
    let root = CallId::new();
    let current_call = CallId::new();

    call_method(
        &component.provider,
        "create_scope",
        Some(
            serde_json::to_value(CreateScopeParams {
                owner_call_id: root,
                root_call_id: Some(root),
                creator: interceptor_origin("planner"),
                target_selector: TargetSelector {
                    provider: "tools".to_owned(),
                    method: "agent_x".to_owned(),
                },
                allowed_method_targets: vec![method_target("demo", "method_1")],
                relation_mode: RelationMode::Descendant,
                label: None,
            })
            .unwrap(),
        ),
    )
    .await
    .unwrap();

    let current = current_context(
        current_call,
        root,
        Some(root),
        method_target("tools", "agent_y"),
    );

    let request = make_request(
        current_call,
        method_target("tools", "agent_y"),
        vec![vec![resolved_query_current(current)]],
    );

    let response = component.interceptor.intercept(&request).await.unwrap();

    assert_eq!(response.continuation, InterceptorContinuation::Stop);
    assert!(response.actions.is_empty());

    let scope = component.store.list_scopes(Default::default()).unwrap();
    assert!(scope.scopes[0].bound_call_id.is_none());
}

#[tokio::test]
async fn multiple_scopes_with_same_bound_call_id_request_one_relation_query() {
    let component = dynamic_policy_fixture();
    let root = CallId::new();
    let principal = CallId::new();
    let descendant = CallId::new();

    for label in ["scope_a", "scope_b"] {
        call_method(
            &component.provider,
            "create_scope",
            Some(
                serde_json::to_value(CreateScopeParams {
                    owner_call_id: root,
                    root_call_id: Some(root),
                    creator: interceptor_origin("planner"),
                    target_selector: TargetSelector {
                        provider: "tools".to_owned(),
                        method: "agent_x".to_owned(),
                    },
                    allowed_method_targets: vec![method_target("demo", "method_1")],
                    relation_mode: RelationMode::DirectChild,
                    label: Some(label.to_owned()),
                })
                .unwrap(),
            ),
        )
        .await
        .unwrap();
    }

    for scope in component
        .store
        .list_scopes(Default::default())
        .unwrap()
        .scopes
    {
        component.store.bind_scope(scope.scope_id, principal);
    }

    let descendant_target = method_target("demo", "method_7");
    let current = current_context(descendant, root, Some(principal), descendant_target.clone());

    let request = make_request(
        descendant,
        descendant_target,
        vec![vec![resolved_query_current(current)]],
    );

    let response = component.interceptor.intercept(&request).await.unwrap();

    assert_eq!(response.continuation, InterceptorContinuation::Reinvoke);
    assert_eq!(response.actions.len(), 1);
    assert_eq!(
        response.actions[0].kind,
        ActionKind::from("query_execution_context")
    );
    assert_eq!(
        response.actions[0].params,
        Some(json!({
            "query": {
                "kind": "relation",
                "subject": descendant,
                "other": principal
            }
        }))
    );
}
