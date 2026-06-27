mod common;

use actrpc_core::{
    action::{ActionKind, ActionSpec, ResolvedActionRecord},
    interception::{InterceptionRequest, InterceptorContinuation},
    json_rpc::{
        JsonRpcError, JsonRpcErrorResponse, JsonRpcId, JsonRpcMessage, JsonRpcResponse,
        JsonRpcSingleMessage, JsonRpcSuccessResponse, JsonRpcVersion,
    },
};
use actrpc_interceptor::interceptors::call_request::{
    CallRequestExecutor, ExecutorConfig,
    schema::{ExecutedCallRequest, canonical_executed_call_request_key},
};
use actrpc_orchestrator::action::actions::{call_method::CallMethod, modify_result::ModifyResult};
use actrpc_orchestrator::interceptor::Interceptor;
use common::support::{default_target, external_origin};
use serde_json::{Map, Value, json};

fn inbound_result(
    result: serde_json::Value,
    history: Vec<Vec<ResolvedActionRecord>>,
) -> InterceptionRequest {
    InterceptionRequest {
        origin: external_origin("caller"),
        target: default_target("invoke"),
        message: JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Success(
            JsonRpcSuccessResponse {
                jsonrpc: JsonRpcVersion::V2_0,
                id: actrpc_core::json_rpc::JsonRpcId::Number(1.into()),
                result,
            },
        ))),
        call_id: actrpc_core::CallId::new(),
        interception_id: actrpc_core::InterceptionId::new(),
        resolved_action_history: history,
        ctx: Default::default(),
    }
}

fn executor() -> CallRequestExecutor {
    CallRequestExecutor::new(ExecutorConfig {
        version: 1,
        call_requests_field: "_actrpc_call_requests".to_owned(),
        results_field: "_actrpc_call_results".to_owned(),
    })
}

fn resolved_call_method(
    provider: &str,
    method: &str,
    call_params: Option<serde_json::Value>,
    response: JsonRpcResponse,
) -> ResolvedActionRecord {
    let mut params = json!({
        "provider": provider,
        "method": method,
    });
    if let Some(call_params) = call_params {
        params["params"] = call_params;
    }

    resolved_call_method_from_params(params, response)
}

fn resolved_call_method_from_params(
    params: Value,
    response: JsonRpcResponse,
) -> ResolvedActionRecord {
    ResolvedActionRecord {
        kind: CallMethod::action_kind(),
        params: Some(params),
        result: Ok(Some(serde_json::to_value(response).unwrap())),
    }
}

fn object_with_order(entries: &[(&str, Value)]) -> Value {
    let mut map = Map::new();
    for (key, value) in entries {
        map.insert((*key).to_owned(), value.clone());
    }
    Value::Object(map)
}

fn results_from_response<'a>(
    response: &'a actrpc_core::interception::InterceptionResponse,
) -> &'a serde_json::Value {
    &response.actions[0].params.as_ref().unwrap()["result"]["_actrpc_call_results"]
}

#[tokio::test]
async fn executor_noop_when_field_absent() {
    let response = executor()
        .intercept(&inbound_result(json!({ "answer": "ok" }), vec![]))
        .await
        .unwrap();
    assert!(response.actions.is_empty());
}

#[tokio::test]
async fn executor_emits_call_method_with_ctx() {
    let response = executor()
        .intercept(&inbound_result(
            json!({
                "_actrpc_call_requests": [{
                    "target": "agents::invoke",
                    "params": { "prompt": "hi" },
                    "ctx": { "interceptors": { "dynamic_policy": { "mode": "detached" } } }
                }]
            }),
            vec![],
        ))
        .await
        .unwrap();

    assert_eq!(response.actions.len(), 1);
    assert_eq!(response.actions[0].kind, CallMethod::action_kind());
    assert_eq!(response.continuation, InterceptorContinuation::Reinvoke);

    let params = response.actions[0].params.as_ref().unwrap();
    assert_eq!(params["provider"], "agents");
    assert_eq!(params["method"], "invoke");
    assert!(params.get("ctx").is_some());
    assert!(params["params"]["prompt"].is_string());
    assert!(params.get("target").is_none());
}

#[tokio::test]
async fn executor_rejects_malformed_target() {
    let err =
        actrpc_interceptor::interceptors::call_request::schema::parse_target("foo").unwrap_err();
    assert!(err.to_string().contains("provider::method"));
}

#[tokio::test]
async fn executor_malformed_present_field_emits_error_result() {
    let response = executor()
        .intercept(&inbound_result(
            json!({ "_actrpc_call_requests": "not-an-array" }),
            vec![],
        ))
        .await
        .unwrap();

    assert_eq!(response.actions[0].kind, ModifyResult::action_kind());
    let result = &response.actions[0].params.as_ref().unwrap()["result"];
    assert!(result["_actrpc_call_results"][0]["error"].is_string());
}

#[tokio::test]
async fn executor_success_result_entry_contains_request_and_response() {
    let history = vec![vec![resolved_call_method(
        "filesystem",
        "read_file",
        None,
        JsonRpcResponse::Success(JsonRpcSuccessResponse {
            jsonrpc: JsonRpcVersion::V2_0,
            id: JsonRpcId::Number(1.into()),
            result: json!({ "content": "x" }),
        }),
    )]];

    let response = executor()
        .intercept(&inbound_result(
            json!({
                "_actrpc_call_requests": [{ "target": "filesystem::read_file" }]
            }),
            history,
        ))
        .await
        .unwrap();

    let results = results_from_response(&response);
    assert_eq!(results[0]["request"]["target"], "filesystem::read_file");
    assert_eq!(results[0]["response"]["jsonrpc"], "2.0");
    assert_eq!(results[0]["response"]["result"]["content"], "x");
}

#[tokio::test]
async fn executor_json_rpc_error_entry_contains_request_and_response() {
    let history = vec![vec![resolved_call_method(
        "filesystem",
        "write_file",
        None,
        JsonRpcResponse::Error(JsonRpcErrorResponse {
            jsonrpc: JsonRpcVersion::V2_0,
            id: JsonRpcId::Number(2.into()),
            error: JsonRpcError {
                code: -32011,
                message: "dynamic policy rejected call".to_owned(),
                data: None,
            },
        }),
    )]];

    let response = executor()
        .intercept(&inbound_result(
            json!({
                "_actrpc_call_requests": [{ "target": "filesystem::write_file" }]
            }),
            history,
        ))
        .await
        .unwrap();

    let results = results_from_response(&response);
    assert_eq!(results[0]["request"]["target"], "filesystem::write_file");
    assert_eq!(results[0]["response"]["jsonrpc"], "2.0");
    assert_eq!(results[0]["response"]["error"]["code"], -32011);
    assert_eq!(
        results[0]["response"]["error"]["message"],
        "dynamic policy rejected call"
    );
    assert!(results[0].get("error").is_none());
}

#[tokio::test]
async fn executor_same_target_different_params_match_correctly() {
    let history = vec![vec![
        resolved_call_method(
            "math",
            "sum",
            Some(json!({ "x": 10, "y": 20 })),
            JsonRpcResponse::Success(JsonRpcSuccessResponse {
                jsonrpc: JsonRpcVersion::V2_0,
                id: JsonRpcId::Number(2.into()),
                result: json!(30),
            }),
        ),
        resolved_call_method(
            "math",
            "sum",
            Some(json!({ "x": 1, "y": 2 })),
            JsonRpcResponse::Success(JsonRpcSuccessResponse {
                jsonrpc: JsonRpcVersion::V2_0,
                id: JsonRpcId::Number(1.into()),
                result: json!(3),
            }),
        ),
    ]];

    let response = executor()
        .intercept(&inbound_result(
            json!({
                "_actrpc_call_requests": [
                    { "target": "math::sum", "params": { "x": 1, "y": 2 } },
                    { "target": "math::sum", "params": { "x": 10, "y": 20 } }
                ]
            }),
            history,
        ))
        .await
        .unwrap();

    let results = results_from_response(&response);
    assert_eq!(results[0]["request"]["params"], json!({ "x": 1, "y": 2 }));
    assert_eq!(results[0]["response"]["result"], 3);
    assert_eq!(results[1]["request"]["params"], json!({ "x": 10, "y": 20 }));
    assert_eq!(results[1]["response"]["result"], 30);
}

#[tokio::test]
async fn executor_identical_requests_emit_two_entries() {
    let history = vec![vec![
        resolved_call_method(
            "math",
            "sum",
            Some(json!({ "x": 1, "y": 2 })),
            JsonRpcResponse::Success(JsonRpcSuccessResponse {
                jsonrpc: JsonRpcVersion::V2_0,
                id: JsonRpcId::Number(1.into()),
                result: json!(3),
            }),
        ),
        resolved_call_method(
            "math",
            "sum",
            Some(json!({ "x": 1, "y": 2 })),
            JsonRpcResponse::Success(JsonRpcSuccessResponse {
                jsonrpc: JsonRpcVersion::V2_0,
                id: JsonRpcId::Number(2.into()),
                result: json!(3),
            }),
        ),
    ]];

    let response = executor()
        .intercept(&inbound_result(
            json!({
                "_actrpc_call_requests": [
                    { "target": "math::sum", "params": { "x": 1, "y": 2 } },
                    { "target": "math::sum", "params": { "x": 1, "y": 2 } }
                ]
            }),
            history,
        ))
        .await
        .unwrap();

    let results = results_from_response(&response);
    assert_eq!(results.as_array().unwrap().len(), 2);
    assert_eq!(results[0]["request"], results[1]["request"]);
    assert_eq!(results[0]["response"]["result"], 3);
    assert_eq!(results[1]["response"]["result"], 3);
}

#[tokio::test]
async fn executor_out_of_order_records_match_by_canonical_request_not_index() {
    let history = vec![vec![
        resolved_call_method(
            "math",
            "sum",
            Some(json!({ "x": 10, "y": 20 })),
            JsonRpcResponse::Success(JsonRpcSuccessResponse {
                jsonrpc: JsonRpcVersion::V2_0,
                id: JsonRpcId::Number(2.into()),
                result: json!(30),
            }),
        ),
        resolved_call_method(
            "math",
            "sum",
            Some(json!({ "x": 1, "y": 2 })),
            JsonRpcResponse::Success(JsonRpcSuccessResponse {
                jsonrpc: JsonRpcVersion::V2_0,
                id: JsonRpcId::Number(1.into()),
                result: json!(3),
            }),
        ),
    ]];

    let response = executor()
        .intercept(&inbound_result(
            json!({
                "_actrpc_call_requests": [
                    { "target": "math::sum", "params": { "x": 1, "y": 2 } },
                    { "target": "math::sum", "params": { "x": 10, "y": 20 } }
                ]
            }),
            history,
        ))
        .await
        .unwrap();

    let results = results_from_response(&response);
    assert_eq!(results[0]["request"]["params"], json!({ "x": 1, "y": 2 }));
    assert_eq!(results[0]["response"]["result"], 3);
    assert_eq!(results[1]["request"]["params"], json!({ "x": 10, "y": 20 }));
    assert_eq!(results[1]["response"]["result"], 30);
}

#[tokio::test]
async fn executor_extra_unrelated_call_method_record_is_ignored() {
    let history = vec![vec![
        resolved_call_method(
            "agents",
            "invoke",
            None,
            JsonRpcResponse::Success(JsonRpcSuccessResponse {
                jsonrpc: JsonRpcVersion::V2_0,
                id: JsonRpcId::Number(99.into()),
                result: json!({ "answer": "unused" }),
            }),
        ),
        resolved_call_method(
            "filesystem",
            "read_file",
            None,
            JsonRpcResponse::Success(JsonRpcSuccessResponse {
                jsonrpc: JsonRpcVersion::V2_0,
                id: JsonRpcId::Number(1.into()),
                result: json!({ "content": "x" }),
            }),
        ),
    ]];

    let response = executor()
        .intercept(&inbound_result(
            json!({
                "_actrpc_call_requests": [{ "target": "filesystem::read_file" }]
            }),
            history,
        ))
        .await
        .unwrap();

    let results = results_from_response(&response);
    assert_eq!(results.as_array().unwrap().len(), 1);
    assert_eq!(results[0]["request"]["target"], "filesystem::read_file");
    assert_eq!(results[0]["response"]["result"]["content"], "x");
}

#[tokio::test]
async fn executor_missing_record_emits_missing_result_for_that_request() {
    let history = vec![vec![resolved_call_method(
        "filesystem",
        "read_file",
        None,
        JsonRpcResponse::Success(JsonRpcSuccessResponse {
            jsonrpc: JsonRpcVersion::V2_0,
            id: JsonRpcId::Number(1.into()),
            result: json!({ "content": "x" }),
        }),
    )]];

    let response = executor()
        .intercept(&inbound_result(
            json!({
                "_actrpc_call_requests": [
                    { "target": "filesystem::read_file" },
                    { "target": "filesystem::write_file", "params": { "path": "/tmp/x" } }
                ]
            }),
            history,
        ))
        .await
        .unwrap();

    let results = results_from_response(&response);
    assert_eq!(results.as_array().unwrap().len(), 2);
    assert_eq!(results[0]["response"]["result"]["content"], "x");
    assert_eq!(results[1]["request"]["target"], "filesystem::write_file");
    assert_eq!(results[1]["request"]["params"]["path"], "/tmp/x");
    assert_eq!(results[1]["error"], "missing CallMethod result");
    assert!(results[1].get("response").is_none());
}

#[tokio::test]
async fn executor_record_with_missing_provider_or_method_does_not_corrupt_other_results() {
    let history = vec![vec![
        ResolvedActionRecord {
            kind: CallMethod::action_kind(),
            params: Some(json!({ "provider": "filesystem" })),
            result: Ok(Some(
                serde_json::to_value(JsonRpcResponse::Success(JsonRpcSuccessResponse {
                    jsonrpc: JsonRpcVersion::V2_0,
                    id: JsonRpcId::Number(99.into()),
                    result: json!({ "content": "orphan" }),
                }))
                .unwrap(),
            )),
        },
        resolved_call_method(
            "filesystem",
            "read_file",
            None,
            JsonRpcResponse::Success(JsonRpcSuccessResponse {
                jsonrpc: JsonRpcVersion::V2_0,
                id: JsonRpcId::Number(1.into()),
                result: json!({ "content": "x" }),
            }),
        ),
    ]];

    let response = executor()
        .intercept(&inbound_result(
            json!({
                "_actrpc_call_requests": [{ "target": "filesystem::read_file" }]
            }),
            history,
        ))
        .await
        .unwrap();

    let results = results_from_response(&response);
    assert_eq!(results.as_array().unwrap().len(), 1);
    assert_eq!(results[0]["request"]["target"], "filesystem::read_file");
    assert_eq!(results[0]["response"]["result"]["content"], "x");
}

#[tokio::test]
async fn executor_preserves_original_request_order() {
    let history = vec![vec![
        resolved_call_method(
            "agents",
            "invoke",
            None,
            JsonRpcResponse::Success(JsonRpcSuccessResponse {
                jsonrpc: JsonRpcVersion::V2_0,
                id: JsonRpcId::Number(3.into()),
                result: json!({ "answer": "c" }),
            }),
        ),
        resolved_call_method(
            "filesystem",
            "read_file",
            None,
            JsonRpcResponse::Success(JsonRpcSuccessResponse {
                jsonrpc: JsonRpcVersion::V2_0,
                id: JsonRpcId::Number(1.into()),
                result: json!({ "content": "a" }),
            }),
        ),
        resolved_call_method(
            "math",
            "sum",
            Some(json!({ "x": 1, "y": 2 })),
            JsonRpcResponse::Success(JsonRpcSuccessResponse {
                jsonrpc: JsonRpcVersion::V2_0,
                id: JsonRpcId::Number(2.into()),
                result: json!(3),
            }),
        ),
    ]];

    let response = executor()
        .intercept(&inbound_result(
            json!({
                "_actrpc_call_requests": [
                    { "target": "filesystem::read_file" },
                    { "target": "math::sum", "params": { "x": 1, "y": 2 } },
                    { "target": "agents::invoke" }
                ]
            }),
            history,
        ))
        .await
        .unwrap();

    let results = results_from_response(&response);
    assert_eq!(results[0]["request"]["target"], "filesystem::read_file");
    assert_eq!(results[0]["response"]["result"]["content"], "a");
    assert_eq!(results[1]["request"]["target"], "math::sum");
    assert_eq!(results[1]["response"]["result"], 3);
    assert_eq!(results[2]["request"]["target"], "agents::invoke");
    assert_eq!(results[2]["response"]["result"]["answer"], "c");
}

#[tokio::test]
async fn executor_ok_none_becomes_missing_result_error() {
    let history = vec![vec![ResolvedActionRecord {
        kind: CallMethod::action_kind(),
        params: Some(json!({ "provider": "tools", "method": "missing" })),
        result: Ok(None),
    }]];

    let response = executor()
        .intercept(&inbound_result(
            json!({ "_actrpc_call_requests": [{ "target": "tools::missing" }] }),
            history,
        ))
        .await
        .unwrap();

    let results = results_from_response(&response);
    assert_eq!(results[0]["request"]["target"], "tools::missing");
    assert_eq!(results[0]["error"], "missing CallMethod result");
    assert!(results[0].get("response").is_none());
}

#[tokio::test]
async fn executor_ignores_non_call_method_records() {
    let history = vec![vec![
        ResolvedActionRecord {
            kind: ActionKind::from("modify_params"),
            params: Some(json!({ "params": {} })),
            result: Ok(None),
        },
        resolved_call_method(
            "filesystem",
            "read_file",
            None,
            JsonRpcResponse::Success(JsonRpcSuccessResponse {
                jsonrpc: JsonRpcVersion::V2_0,
                id: JsonRpcId::Number(1.into()),
                result: json!({ "content": "x" }),
            }),
        ),
    ]];

    let response = executor()
        .intercept(&inbound_result(
            json!({ "_actrpc_call_requests": [{ "target": "filesystem::read_file" }] }),
            history,
        ))
        .await
        .unwrap();

    let results = results_from_response(&response);
    assert_eq!(results.as_array().unwrap().len(), 1);
    assert_eq!(results[0]["request"]["target"], "filesystem::read_file");
}

#[tokio::test]
async fn executor_prior_call_method_records_emit_modify_result_not_call_method() {
    let history = vec![vec![resolved_call_method(
        "agents",
        "invoke",
        None,
        JsonRpcResponse::Success(JsonRpcSuccessResponse {
            jsonrpc: JsonRpcVersion::V2_0,
            id: JsonRpcId::Number(1.into()),
            result: json!({ "answer": "ok" }),
        }),
    )]];

    let response = executor()
        .intercept(&inbound_result(
            json!({ "_actrpc_call_requests": [{ "target": "agents::invoke" }] }),
            history,
        ))
        .await
        .unwrap();

    assert_eq!(response.actions[0].kind, ModifyResult::action_kind());
    assert_ne!(response.actions[0].kind, CallMethod::action_kind());
}

#[test]
fn call_request_rejects_unknown_fields() {
    let err = serde_json::from_str::<actrpc_interceptor::interceptors::call_request::CallRequest>(
        r#"{"target":"a::b","extra":true}"#,
    )
    .unwrap_err();
    assert!(err.to_string().contains("unknown field"));
}

#[test]
fn canonical_key_ignores_top_level_params_object_order() {
    let ordered = ExecutedCallRequest {
        target: "math::sum".to_owned(),
        params: Some(json!({ "x": 1, "y": 2 })),
        ctx: None,
    };
    let reversed = ExecutedCallRequest {
        target: "math::sum".to_owned(),
        params: Some(object_with_order(&[("y", json!(2)), ("x", json!(1))])),
        ctx: None,
    };

    assert_eq!(
        canonical_executed_call_request_key(&ordered).unwrap(),
        canonical_executed_call_request_key(&reversed).unwrap()
    );
}

#[test]
fn canonical_key_ignores_nested_object_order() {
    let ordered = ExecutedCallRequest {
        target: "tools::filter".to_owned(),
        params: Some(json!({
            "query": {
                "a": 1,
                "b": 2
            }
        })),
        ctx: None,
    };
    let reversed = ExecutedCallRequest {
        target: "tools::filter".to_owned(),
        params: Some(json!({
            "query": object_with_order(&[("b", json!(2)), ("a", json!(1))])
        })),
        ctx: None,
    };

    assert_eq!(
        canonical_executed_call_request_key(&ordered).unwrap(),
        canonical_executed_call_request_key(&reversed).unwrap()
    );
}

#[test]
fn canonical_key_preserves_array_order() {
    let first = ExecutedCallRequest {
        target: "math::sum".to_owned(),
        params: Some(json!([1, 2])),
        ctx: None,
    };
    let second = ExecutedCallRequest {
        target: "math::sum".to_owned(),
        params: Some(json!([2, 1])),
        ctx: None,
    };

    assert_ne!(
        canonical_executed_call_request_key(&first).unwrap(),
        canonical_executed_call_request_key(&second).unwrap()
    );
}

#[tokio::test]
async fn executor_matches_same_request_with_different_params_object_order() {
    let history = vec![vec![resolved_call_method_from_params(
        object_with_order(&[
            ("provider", json!("math")),
            ("method", json!("sum")),
            (
                "params",
                object_with_order(&[("y", json!(2)), ("x", json!(1))]),
            ),
        ]),
        JsonRpcResponse::Success(JsonRpcSuccessResponse {
            jsonrpc: JsonRpcVersion::V2_0,
            id: JsonRpcId::Number(1.into()),
            result: json!(3),
        }),
    )]];

    let response = executor()
        .intercept(&inbound_result(
            json!({
                "_actrpc_call_requests": [{
                    "target": "math::sum",
                    "params": { "x": 1, "y": 2 }
                }]
            }),
            history,
        ))
        .await
        .unwrap();

    let results = results_from_response(&response);
    assert_eq!(results[0]["request"]["params"], json!({ "x": 1, "y": 2 }));
    assert_eq!(results[0]["response"]["result"], 3);
    assert!(results[0].get("error").is_none());
}

#[tokio::test]
async fn executor_matches_same_request_with_different_ctx_object_order() {
    let history = vec![vec![resolved_call_method_from_params(
        object_with_order(&[
            ("provider", json!("agents")),
            ("method", json!("invoke")),
            (
                "ctx",
                object_with_order(&[(
                    "shared",
                    object_with_order(&[("b", json!(2)), ("a", json!(1))]),
                )]),
            ),
        ]),
        JsonRpcResponse::Success(JsonRpcSuccessResponse {
            jsonrpc: JsonRpcVersion::V2_0,
            id: JsonRpcId::Number(1.into()),
            result: json!({ "answer": "ok" }),
        }),
    )]];

    let response = executor()
        .intercept(&inbound_result(
            json!({
                "_actrpc_call_requests": [{
                    "target": "agents::invoke",
                    "ctx": {
                        "shared": { "a": 1, "b": 2 }
                    }
                }]
            }),
            history,
        ))
        .await
        .unwrap();

    let results = results_from_response(&response);
    assert_eq!(results[0]["request"]["target"], "agents::invoke");
    assert_eq!(
        results[0]["request"]["ctx"]["shared"],
        json!({ "a": 1, "b": 2 })
    );
    assert_eq!(results[0]["response"]["result"]["answer"], "ok");
    assert!(results[0].get("error").is_none());
}

#[tokio::test]
async fn executor_does_not_match_when_array_param_order_differs() {
    let history = vec![vec![resolved_call_method_from_params(
        json!({
            "provider": "math",
            "method": "sum",
            "params": [2, 1]
        }),
        JsonRpcResponse::Success(JsonRpcSuccessResponse {
            jsonrpc: JsonRpcVersion::V2_0,
            id: JsonRpcId::Number(1.into()),
            result: json!(3),
        }),
    )]];

    let response = executor()
        .intercept(&inbound_result(
            json!({
                "_actrpc_call_requests": [{
                    "target": "math::sum",
                    "params": [1, 2]
                }]
            }),
            history,
        ))
        .await
        .unwrap();

    let results = results_from_response(&response);
    assert_eq!(results[0]["request"]["params"], json!([1, 2]));
    assert_eq!(results[0]["error"], "missing CallMethod result");
    assert!(results[0].get("response").is_none());
}
