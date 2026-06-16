mod support;

use actrpc_core::{
    CallId, CallRelation,
    action::ActionKind,
    interception::InterceptorContinuation,
    json_rpc::{JsonRpcMessage, JsonRpcRequest, JsonRpcSingleMessage, JsonRpcVersion},
};
use actrpc_interceptor::interceptors::dynamic_policy::{
    CreateScopeParams, ListScopesParams, RelationMode, TargetSelector, provider::call_method,
};
use actrpc_orchestrator::interceptor::Interceptor;
use support::{
    current_context, dynamic_policy_fixture, interceptor_origin, method_target,
    resolved_query_current, resolved_query_relation, sample_request,
};

fn make_request(
    call_id: CallId,
    target: actrpc_core::MethodTarget,
    history: Vec<Vec<actrpc_core::action::ResolvedActionRecord>>,
) -> actrpc_core::interception::InterceptionRequest {
    let mut request = sample_request(
        interceptor_origin("planner"),
        target.clone(),
        JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
            jsonrpc: JsonRpcVersion::V2_0,
            id: actrpc_core::json_rpc::JsonRpcId::Number(1_u64.into()),
            method: target.method.clone(),
            params: None,
        })),
        history,
    );
    request.call_id = call_id;
    request
}

async fn create_scope_for_agent(
    component: &actrpc_interceptor::interceptors::dynamic_policy::DynamicPolicyComponent,
    root: CallId,
    agent_method: &str,
    allowed: Vec<actrpc_core::MethodTarget>,
    label: Option<&str>,
) {
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
                    method: agent_method.to_owned(),
                },
                allowed_method_targets: allowed,
                relation_mode: RelationMode::DirectChild,
                label: label.map(str::to_owned),
            })
            .unwrap(),
        ),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn multiple_scopes_for_x_and_y_narrow_descendants_independently() {
    let component = dynamic_policy_fixture();
    let root = CallId::new();
    let principal_x = CallId::new();
    let principal_y = CallId::new();
    let child_x = CallId::new();
    let child_y = CallId::new();

    create_scope_for_agent(
        &component,
        root,
        "agent_x",
        vec![
            method_target("demo", "method_1"),
            method_target("demo", "method_2"),
        ],
        Some("scope_x"),
    )
    .await;

    create_scope_for_agent(
        &component,
        root,
        "agent_y",
        vec![
            method_target("demo", "method_3"),
            method_target("demo", "method_4"),
        ],
        Some("scope_y"),
    )
    .await;

    let scopes = component
        .store
        .list_scopes(Default::default())
        .unwrap()
        .scopes;
    let scope_x = scopes
        .iter()
        .find(|scope| scope.label.as_deref() == Some("scope_x"))
        .unwrap()
        .scope_id;
    let scope_y = scopes
        .iter()
        .find(|scope| scope.label.as_deref() == Some("scope_y"))
        .unwrap()
        .scope_id;

    component.store.bind_scope(scope_x, principal_x);
    component.store.bind_scope(scope_y, principal_y);

    let reject_x = make_request(
        child_x,
        method_target("demo", "method_7"),
        vec![vec![
            resolved_query_current(current_context(
                child_x,
                root,
                Some(principal_x),
                method_target("demo", "method_7"),
            )),
            resolved_query_relation(child_x, principal_x, CallRelation::Parent),
            resolved_query_relation(child_x, principal_y, CallRelation::Unrelated),
        ]],
    );

    let reject_response = component.interceptor.intercept(&reject_x).await.unwrap();
    assert_eq!(
        reject_response.actions[0].kind,
        ActionKind::from("reject_call")
    );

    let allow_y = make_request(
        child_y,
        method_target("demo", "method_3"),
        vec![vec![
            resolved_query_current(current_context(
                child_y,
                root,
                Some(principal_y),
                method_target("demo", "method_3"),
            )),
            resolved_query_relation(child_y, principal_y, CallRelation::Parent),
            resolved_query_relation(child_y, principal_x, CallRelation::Unrelated),
        ]],
    );

    let allow_response = component.interceptor.intercept(&allow_y).await.unwrap();
    assert_eq!(allow_response.continuation, InterceptorContinuation::Stop);
    assert!(allow_response.actions.is_empty());
}

#[tokio::test]
async fn scopes_are_isolated_across_different_roots() {
    let component = dynamic_policy_fixture();
    let root_a = CallId::new();
    let root_b = CallId::new();
    let principal_a = CallId::new();
    let descendant = CallId::new();

    create_scope_for_agent(
        &component,
        root_a,
        "agent_x",
        vec![method_target("demo", "method_1")],
        None,
    )
    .await;

    let scope = component
        .store
        .list_scopes(ListScopesParams {
            root_call_id: Some(root_a),
            ..Default::default()
        })
        .unwrap()
        .scopes[0]
        .clone();
    component.store.bind_scope(scope.scope_id, principal_a);

    let request = make_request(
        descendant,
        method_target("demo", "method_7"),
        vec![vec![
            resolved_query_current(current_context(
                descendant,
                root_b,
                Some(principal_a),
                method_target("demo", "method_7"),
            )),
            resolved_query_relation(descendant, principal_a, CallRelation::Parent),
        ]],
    );

    let response = component.interceptor.intercept(&request).await.unwrap();
    assert!(response.actions.is_empty());
}

#[tokio::test]
async fn multiple_applying_scopes_use_intersection() {
    let component = dynamic_policy_fixture();
    let root = CallId::new();
    let principal = CallId::new();
    let descendant = CallId::new();

    create_scope_for_agent(
        &component,
        root,
        "agent_x",
        vec![
            method_target("demo", "method_1"),
            method_target("demo", "method_2"),
        ],
        Some("wide"),
    )
    .await;

    create_scope_for_agent(
        &component,
        root,
        "agent_x",
        vec![method_target("demo", "method_2")],
        Some("narrow"),
    )
    .await;

    let scopes = component
        .store
        .list_scopes(Default::default())
        .unwrap()
        .scopes;
    for scope in scopes {
        component.store.bind_scope(scope.scope_id, principal);
    }

    let allowed = make_request(
        descendant,
        method_target("demo", "method_2"),
        vec![vec![
            resolved_query_current(current_context(
                descendant,
                root,
                Some(principal),
                method_target("demo", "method_2"),
            )),
            resolved_query_relation(descendant, principal, CallRelation::Parent),
        ]],
    );

    let allowed_response = component.interceptor.intercept(&allowed).await.unwrap();
    assert!(allowed_response.actions.is_empty());

    let rejected = make_request(
        descendant,
        method_target("demo", "method_1"),
        vec![vec![
            resolved_query_current(current_context(
                descendant,
                root,
                Some(principal),
                method_target("demo", "method_1"),
            )),
            resolved_query_relation(descendant, principal, CallRelation::Parent),
        ]],
    );

    let rejected_response = component.interceptor.intercept(&rejected).await.unwrap();
    assert_eq!(
        rejected_response.actions[0].kind,
        ActionKind::from("reject_call")
    );
}

#[tokio::test]
async fn dynamic_policy_is_reject_only_and_does_not_emit_allow_actions() {
    let component = dynamic_policy_fixture();
    let init = component.interceptor.initialize().await.unwrap();

    assert_eq!(init.actions.len(), 2);
    assert!(init.actions.contains_key(&ActionKind::from("reject_call")));
    assert!(
        init.actions
            .contains_key(&ActionKind::from("query_execution_context"))
    );
}
