mod common;

use actrpc_core::{
    action::ActionSpec,
    interception::{InterceptionRequest, InterceptorContinuation},
    json_rpc::{
        JsonRpcMessage, JsonRpcParams, JsonRpcRequest, JsonRpcResponse, JsonRpcSingleMessage,
        JsonRpcSuccessResponse, JsonRpcVersion,
    },
};
use actrpc_interceptor::interceptors::policy::{
    PolicyInterceptor,
    config::{
        MatchExpr, PolicyApply, PolicyConfig, PolicyEffect, PolicyMatcher, PolicyRule,
        RejectCallEffect,
    },
};
use actrpc_orchestrator::{action::actions::reject_call::RejectCall, interceptor::Interceptor};
use common::support::{default_target, external_origin, sample_request};
use serde_json::json;

#[tokio::test]
async fn phase_fact_matches_outbound_only() {
    let interceptor = PolicyInterceptor::new(outbound_write_reject_policy()).unwrap();

    let request = outbound_request("write_file", json!({ "path": "/tmp/a.txt" }));

    let response = interceptor.intercept(&request).await.unwrap();

    assert_eq!(response.continuation, InterceptorContinuation::Stop);
    assert_eq!(response.actions.len(), 1);
    assert_eq!(response.actions[0].kind, RejectCall::action_kind());
}

#[tokio::test]
async fn outbound_phase_rule_does_not_match_inbound_response() {
    let interceptor = PolicyInterceptor::new(outbound_write_reject_policy()).unwrap();

    let request = inbound_success_response(json!({ "ok": true }));

    let response = interceptor.intercept(&request).await.unwrap();

    assert_eq!(response.continuation, InterceptorContinuation::Stop);
    assert!(response.actions.is_empty());
}

fn outbound_write_reject_policy() -> PolicyConfig {
    PolicyConfig {
        rules: vec![PolicyRule {
            name: "reject_outbound_write".to_owned(),
            match_expr: MatchExpr::all(vec![
                MatchExpr::condition("phase", PolicyMatcher::exact("outbound")),
                MatchExpr::condition("message.method", PolicyMatcher::exact("write_file")),
            ]),
            apply: PolicyApply {
                immediate: vec![PolicyEffect::RejectCall {
                    reject_call: RejectCallEffect {
                        error: actrpc_core::json_rpc::JsonRpcError {
                            code: -32010,
                            message: "outbound write denied".to_owned(),
                            data: None,
                        },
                    },
                }],
                review: None,
            },
        }],
    }
}

fn outbound_request(method: &str, params: serde_json::Value) -> InterceptionRequest {
    let params = match params {
        serde_json::Value::Array(values) => Some(JsonRpcParams::Array(values)),
        serde_json::Value::Object(map) => Some(JsonRpcParams::Object(map)),
        other => panic!("test params must be array or object, got {other}"),
    };

    sample_request(
        external_origin("caller"),
        default_target(method),
        JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
            jsonrpc: JsonRpcVersion::V2_0,
            id: actrpc_core::json_rpc::JsonRpcId::Number(1.into()),
            method: method.to_owned(),
            params,
        })),
        vec![],
    )
}

fn inbound_success_response(result: serde_json::Value) -> InterceptionRequest {
    sample_request(
        external_origin("test-provider"),
        default_target("write_file"),
        JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Success(
            JsonRpcSuccessResponse {
                jsonrpc: JsonRpcVersion::V2_0,
                id: actrpc_core::json_rpc::JsonRpcId::Number(1.into()),
                result,
            },
        ))),
        vec![],
    )
}
