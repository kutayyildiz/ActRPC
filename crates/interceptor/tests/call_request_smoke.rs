mod common;

use actrpc_core::{
    action::{ActionSpec, ResolvedActionRecord},
    interception::{InterceptionRequest, InterceptorContinuation},
    json_rpc::{
        JsonRpcId, JsonRpcMessage, JsonRpcParams, JsonRpcRequest, JsonRpcResponse,
        JsonRpcSingleMessage, JsonRpcSuccessResponse, JsonRpcVersion,
    },
};
use actrpc_interceptor::interceptors::call_request::{
    CallRequestExecutor, CallRequestInstructor, ExecutorConfig, InstructorConfig,
    config::CallRequestConfigFormat,
};
use actrpc_orchestrator::{
    action::actions::{
        call_method::CallMethod, modify_params::ModifyParams, modify_result::ModifyResult,
    },
    interceptor::Interceptor,
};
use common::support::external_origin;
use serde_json::json;

const INSTRUCTOR_CONFIG_TOML: &str = r#"
version = 1

[[rules]]
name = "agent_invoke"
provider = "agents"
method = "invoke"
prompt_field = "prompt"

[rules.injection]
prepend = "PREPEND_SMOKE"
append = "APPEND_SMOKE"
"#;

const EXECUTOR_CONFIG_TOML: &str = r#"
version = 1
call_requests_field = "_actrpc_call_requests"
results_field = "_actrpc_call_results"
"#;

fn outbound_invoke(prompt: &str) -> InterceptionRequest {
    let mut params = serde_json::Map::new();
    params.insert("prompt".to_owned(), json!(prompt));

    InterceptionRequest {
        origin: external_origin("caller"),
        target: actrpc_core::MethodTarget {
            provider: "agents".to_owned(),
            method: "invoke".to_owned(),
        },
        message: JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
            jsonrpc: JsonRpcVersion::V2_0,
            id: JsonRpcId::Number(1.into()),
            method: "invoke".to_owned(),
            params: Some(JsonRpcParams::Object(params)),
        })),
        call_id: actrpc_core::CallId::new(),
        interception_id: actrpc_core::InterceptionId::new(),
        resolved_action_history: vec![],
        ctx: Default::default(),
    }
}

fn inbound_result(
    result: serde_json::Value,
    history: Vec<Vec<ResolvedActionRecord>>,
) -> InterceptionRequest {
    InterceptionRequest {
        origin: external_origin("caller"),
        target: actrpc_core::MethodTarget {
            provider: "agents".to_owned(),
            method: "invoke".to_owned(),
        },
        message: JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Success(
            JsonRpcSuccessResponse {
                jsonrpc: JsonRpcVersion::V2_0,
                id: JsonRpcId::Number(1.into()),
                result,
            },
        ))),
        call_id: actrpc_core::CallId::new(),
        interception_id: actrpc_core::InterceptionId::new(),
        resolved_action_history: history,
        ctx: Default::default(),
    }
}

#[tokio::test]
async fn call_request_instructor_config_smoke() {
    let config = InstructorConfig::from_str_with_format(
        INSTRUCTOR_CONFIG_TOML,
        CallRequestConfigFormat::Toml,
        "instructor_smoke.toml",
    )
    .expect("instructor config should load");

    let instructor = CallRequestInstructor::new(config);
    let response = instructor
        .intercept(&outbound_invoke("ORIGINAL"))
        .await
        .expect("instructor intercept should succeed");

    assert_eq!(response.continuation, InterceptorContinuation::Stop);
    assert_eq!(response.actions.len(), 1);
    assert_eq!(response.actions[0].kind, ModifyParams::action_kind());

    let prompt = response.actions[0].params.as_ref().unwrap()["params"]["prompt"]
        .as_str()
        .expect("modified prompt should be a string");
    assert_eq!(prompt, "PREPEND_SMOKE\n\nORIGINAL\n\nAPPEND_SMOKE");
    assert!(!prompt.contains("_actrpc_call_requests"));
    assert!(!prompt.contains("dynamic_policy"));
}

#[tokio::test]
async fn call_request_executor_config_smoke() {
    let config = ExecutorConfig::from_str_with_format(
        EXECUTOR_CONFIG_TOML,
        CallRequestConfigFormat::Toml,
        "executor_smoke.toml",
    )
    .expect("executor config should load");

    let executor = CallRequestExecutor::new(config);
    let first_result = json!({
        "_actrpc_call_requests": [{
            "target": "math::sum",
            "params": { "x": 1, "y": 2 }
        }]
    });

    let first_response = executor
        .intercept(&inbound_result(first_result.clone(), vec![]))
        .await
        .expect("first executor intercept should succeed");

    assert_eq!(first_response.continuation, InterceptorContinuation::Reinvoke);
    assert_eq!(first_response.actions.len(), 1);
    assert_eq!(first_response.actions[0].kind, CallMethod::action_kind());

    let call_method_params = first_response.actions[0].params.as_ref().unwrap();
    assert_eq!(call_method_params["provider"], "math");
    assert_eq!(call_method_params["method"], "sum");
    assert_eq!(call_method_params["params"], json!({ "x": 1, "y": 2 }));
    assert!(call_method_params.get("target").is_none());

    let history = vec![vec![ResolvedActionRecord {
        kind: CallMethod::action_kind(),
        params: Some(json!({
            "provider": "math",
            "method": "sum",
            "params": { "x": 1, "y": 2 }
        })),
        result: Ok(Some(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": 3
        }))),
    }]];

    let second_response = executor
        .intercept(&inbound_result(first_result, history))
        .await
        .expect("second executor intercept should succeed");

    assert_eq!(second_response.continuation, InterceptorContinuation::Stop);
    assert_eq!(second_response.actions.len(), 1);
    assert_eq!(second_response.actions[0].kind, ModifyResult::action_kind());

    let updated_result = &second_response.actions[0].params.as_ref().unwrap()["result"];
    let results = &updated_result["_actrpc_call_results"];
    assert_eq!(results.as_array().unwrap().len(), 1);
    assert_eq!(results[0]["request"]["target"], "math::sum");
    assert_eq!(results[0]["request"]["params"], json!({ "x": 1, "y": 2 }));
    assert_eq!(results[0]["response"]["jsonrpc"], "2.0");
    assert_eq!(results[0]["response"]["id"], 1);
    assert_eq!(results[0]["response"]["result"], 3);
    assert!(results[0].get("error").is_none());
}