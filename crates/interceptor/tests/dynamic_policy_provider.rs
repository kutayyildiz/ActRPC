mod support;

use actrpc_core::{CallId, json_rpc::JsonRpcParams};
use actrpc_interceptor::interceptors::dynamic_policy::{
    CreateScopeParams, RelationMode, TargetSelector, provider::call_method,
};
use actrpc_orchestrator::method::MethodProvider;
use serde_json::json;
use support::{dynamic_policy_fixture, interceptor_origin, method_target};

fn sample_create_params(owner: CallId) -> CreateScopeParams {
    CreateScopeParams {
        owner_call_id: owner,
        root_call_id: Some(owner),
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
    }
}

#[tokio::test]
async fn create_scope_returns_scope_id() {
    let component = dynamic_policy_fixture();
    let owner = CallId::new();

    let result = call_method(
        &component.provider,
        "create_scope",
        Some(serde_json::to_value(sample_create_params(owner)).unwrap()),
    )
    .await
    .unwrap();

    assert!(result.get("scope_id").is_some());
}

#[tokio::test]
async fn release_get_and_list_scope_work() {
    let component = dynamic_policy_fixture();
    let owner = CallId::new();

    let created = call_method(
        &component.provider,
        "create_scope",
        Some(serde_json::to_value(sample_create_params(owner)).unwrap()),
    )
    .await
    .unwrap();

    let scope_id = created["scope_id"].as_str().unwrap();

    let scope = call_method(
        &component.provider,
        "get_scope",
        Some(json!({ "scope_id": scope_id })),
    )
    .await
    .unwrap();

    assert_eq!(scope["owner_call_id"], serde_json::to_value(owner).unwrap());

    let listed = call_method(
        &component.provider,
        "list_scopes",
        Some(json!({ "owner_call_id": owner })),
    )
    .await
    .unwrap();

    assert_eq!(listed["scopes"].as_array().unwrap().len(), 1);

    let released = call_method(
        &component.provider,
        "release_scope",
        Some(json!({
            "scope_id": scope_id,
            "creator": {
                "kind": "interceptor",
                "id": "planner"
            }
        })),
    )
    .await
    .unwrap();

    assert_eq!(released["released"], true);

    let err = call_method(
        &component.provider,
        "get_scope",
        Some(json!({ "scope_id": scope_id })),
    )
    .await
    .unwrap_err();

    assert!(err.to_string().contains("scope not found"));
}

#[tokio::test]
async fn empty_allowlist_is_rejected() {
    let component = dynamic_policy_fixture();
    let owner = CallId::new();
    let mut params = sample_create_params(owner);
    params.allowed_method_targets.clear();

    let err = call_method(
        &component.provider,
        "create_scope",
        Some(serde_json::to_value(params).unwrap()),
    )
    .await
    .unwrap_err();

    assert!(err.to_string().contains("allowed_method_targets"));
}

#[tokio::test]
async fn invalid_glob_is_rejected() {
    let component = dynamic_policy_fixture();
    let owner = CallId::new();
    let mut params = sample_create_params(owner);
    params.target_selector.method = "[invalid".to_owned();

    let err = call_method(
        &component.provider,
        "create_scope",
        Some(serde_json::to_value(params).unwrap()),
    )
    .await
    .unwrap_err();

    assert!(err.to_string().contains("glob"));
}

#[tokio::test]
async fn release_scope_creator_mismatch_is_consistency_failure() {
    let component = dynamic_policy_fixture();
    let owner = CallId::new();

    let created = call_method(
        &component.provider,
        "create_scope",
        Some(serde_json::to_value(sample_create_params(owner)).unwrap()),
    )
    .await
    .unwrap();

    let scope_id = created["scope_id"].as_str().unwrap();

    let err = call_method(
        &component.provider,
        "release_scope",
        Some(json!({
            "scope_id": scope_id,
            "creator": {
                "kind": "interceptor",
                "id": "other"
            }
        })),
    )
    .await
    .unwrap_err();

    assert!(err.to_string().contains("creator mismatch"));
    assert_eq!(err.json_rpc_code(), -32013);
}

#[tokio::test]
async fn scope_not_found_uses_stable_error_code() {
    let component = dynamic_policy_fixture();

    let err = call_method(
        &component.provider,
        "get_scope",
        Some(json!({ "scope_id": "550e8400-e29b-41d4-a716-446655440000" })),
    )
    .await
    .unwrap_err();

    assert!(err.to_string().contains("scope not found"));
    assert_eq!(err.json_rpc_code(), -32012);
}

#[tokio::test]
async fn invalid_params_use_invalid_params_error_code() {
    let component = dynamic_policy_fixture();
    let owner = CallId::new();
    let mut params = sample_create_params(owner);
    params.allowed_method_targets.clear();

    let err = call_method(
        &component.provider,
        "create_scope",
        Some(serde_json::to_value(params).unwrap()),
    )
    .await
    .unwrap_err();

    assert_eq!(err.json_rpc_code(), -32602);
}

#[tokio::test]
async fn list_scopes_accepts_null_params() {
    let component = dynamic_policy_fixture();
    let owner = CallId::new();

    call_method(
        &component.provider,
        "create_scope",
        Some(serde_json::to_value(sample_create_params(owner)).unwrap()),
    )
    .await
    .unwrap();

    let listed = call_method(&component.provider, "list_scopes", None)
        .await
        .unwrap();

    assert_eq!(listed["scopes"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn provider_snapshot_lists_four_methods() {
    let component = dynamic_policy_fixture();
    let snapshot = component.provider.snapshot();

    assert_eq!(snapshot.provider.as_str(), "dynamic_policy");
    assert_eq!(snapshot.methods.len(), 4);
}

#[tokio::test]
async fn missing_owner_call_id_is_rejected_by_deserialization() {
    let component = dynamic_policy_fixture();

    let err = call_method(
        &component.provider,
        "create_scope",
        Some(json!({
            "root_call_id": CallId::new(),
            "creator": { "kind": "interceptor", "id": "planner" },
            "target_selector": { "provider": "tools", "method": "agent_x" },
            "allowed_method_targets": [{ "provider": "demo", "method": "method_1" }],
            "relation_mode": "direct_child"
        })),
    )
    .await
    .unwrap_err();

    assert!(err.to_string().contains("missing field") || err.to_string().contains("owner_call_id"));
}

#[tokio::test]
async fn method_provider_call_round_trip() {
    let component = dynamic_policy_fixture();
    let owner = CallId::new();

    let params: JsonRpcParams =
        serde_json::from_value(serde_json::to_value(sample_create_params(owner)).unwrap()).unwrap();

    let value = component
        .provider
        .call(&"create_scope".into(), Some(params))
        .await
        .unwrap();

    assert!(value.get("scope_id").is_some());
}
