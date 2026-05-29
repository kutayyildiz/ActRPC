use actrpc_core::{
    action::{ActionKind, ResolvedActionRecord},
    interception::{InterceptionRequest, InterceptorContinuation},
    json_rpc::{
        JsonRpcId, JsonRpcMessage, JsonRpcParams, JsonRpcRequest, JsonRpcSingleMessage,
        JsonRpcVersion,
    },
    participant::{Participant, ParticipantType},
};
use actrpc_interceptor::interceptors::policy::{
    PolicyInterceptor,
    config::{
        MatchExpr, PolicyApply, PolicyConfig, PolicyEffect, PolicyMatcher, PolicyReview,
        PolicyReviewSeverity, PolicyRule,
    },
};
use actrpc_orchestrator::{
    action::actions::request_review::{
        REVIEW_DECISION_APPROVED, REVIEW_DECISION_DENIED, REVIEW_SEVERITY_HIGH,
    },
    interceptor::Interceptor,
};
use serde_json::json;

#[tokio::test]
async fn no_matching_rule_emits_no_actions_and_stops() {
    let interceptor = PolicyInterceptor::new(PolicyConfig {
        rules: vec![PolicyRule {
            name: "reject_write_file".to_owned(),
            match_expr: MatchExpr::condition("message.method", PolicyMatcher::exact("write_file")),
            apply: PolicyApply {
                immediate: vec![PolicyEffect::reject_call(json_rpc_error(-32010, "denied"))],
                review: None,
            },
        }],
    })
    .unwrap();

    let request = outbound_request("read_file", json!({ "path": "/tmp/a.txt" }));

    let response = interceptor.intercept(&request).await.unwrap();

    assert_eq!(response.continuation, InterceptorContinuation::Stop);
    assert!(response.actions.is_empty());
}

#[tokio::test]
async fn matching_reject_call_rule_emits_reject_call_action() {
    let interceptor = PolicyInterceptor::new(PolicyConfig {
        rules: vec![PolicyRule {
            name: "reject_write_file".to_owned(),
            match_expr: MatchExpr::condition("message.method", PolicyMatcher::exact("write_file")),
            apply: PolicyApply {
                immediate: vec![PolicyEffect::reject_call(json_rpc_error(
                    -32010,
                    "write denied",
                ))],
                review: None,
            },
        }],
    })
    .unwrap();

    let request = outbound_request("write_file", json!({ "path": "/tmp/a.txt" }));

    let response = interceptor.intercept(&request).await.unwrap();

    assert_eq!(response.continuation, InterceptorContinuation::Stop);
    assert_eq!(response.actions.len(), 1);

    let action = &response.actions[0];

    assert_eq!(action.kind, ActionKind::from("reject_call"));
    assert_eq!(
        action.params,
        Some(json!({
            "error": {
                "code": -32010,
                "message": "write denied"
            }
        }))
    );
}

#[tokio::test]
async fn matching_exclude_interceptors_rule_emits_exclude_interceptors_action() {
    let interceptor = PolicyInterceptor::new(PolicyConfig {
        rules: vec![PolicyRule {
            name: "exclude_loggers".to_owned(),
            match_expr: MatchExpr::condition("message.method", PolicyMatcher::exact("read_file")),
            apply: PolicyApply {
                immediate: vec![PolicyEffect::exclude_interceptors(vec![
                    "audit_logger".to_owned(),
                    "transcript_logger".to_owned(),
                ])],
                review: None,
            },
        }],
    })
    .unwrap();

    let request = outbound_request("read_file", json!({ "path": "/secrets/key.txt" }));

    let response = interceptor.intercept(&request).await.unwrap();

    assert_eq!(response.continuation, InterceptorContinuation::Stop);
    assert_eq!(response.actions.len(), 1);

    let action = &response.actions[0];

    assert_eq!(action.kind, ActionKind::from("exclude_interceptors"));
    assert_eq!(
        action.params,
        Some(json!({
            "names": ["audit_logger", "transcript_logger"]
        }))
    );
}

#[tokio::test]
async fn review_rule_emits_request_review_and_reinvokes() {
    let interceptor = PolicyInterceptor::new(review_policy_config(
        vec![],
        vec![PolicyEffect::reject_call(json_rpc_error(
            -32051,
            "user denied sensitive write",
        ))],
    ))
    .unwrap();

    let request = outbound_request("write_file", json!({ "path": "/home/mortal/file.txt" }));

    let response = interceptor.intercept(&request).await.unwrap();

    assert_eq!(response.continuation, InterceptorContinuation::Reinvoke);
    assert_eq!(response.actions.len(), 1);

    let action = &response.actions[0];

    assert_eq!(action.kind, ActionKind::from("request_review"));
    assert_eq!(
        action.params,
        Some(json!({
            "rule_name": "review_sensitive_write",
            "title": "Sensitive file write",
            "reason": "Agent wants to write inside a user-owned directory.",
            "severity": REVIEW_SEVERITY_HIGH
        }))
    );
}

#[tokio::test]
async fn approved_review_emits_on_approve_effects_and_stops() {
    let interceptor = PolicyInterceptor::new(review_policy_config(
        vec![PolicyEffect::exclude_interceptors(vec![
            "audit_logger".to_owned(),
        ])],
        vec![PolicyEffect::reject_call(json_rpc_error(
            -32051,
            "user denied sensitive write",
        ))],
    ))
    .unwrap();

    let mut request = outbound_request("write_file", json!({ "path": "/home/mortal/file.txt" }));
    request.resolved_action_history = vec![vec![resolved_request_review(
        "review_sensitive_write",
        REVIEW_DECISION_APPROVED,
    )]];

    let response = interceptor.intercept(&request).await.unwrap();

    assert_eq!(response.continuation, InterceptorContinuation::Stop);
    assert_eq!(response.actions.len(), 1);

    let action = &response.actions[0];

    assert_eq!(action.kind, ActionKind::from("exclude_interceptors"));
    assert_eq!(
        action.params,
        Some(json!({
            "names": ["audit_logger"]
        }))
    );
}

#[tokio::test]
async fn denied_review_emits_on_deny_effects_and_stops() {
    let interceptor = PolicyInterceptor::new(review_policy_config(
        vec![],
        vec![PolicyEffect::reject_call(json_rpc_error(
            -32051,
            "user denied sensitive write",
        ))],
    ))
    .unwrap();

    let mut request = outbound_request("write_file", json!({ "path": "/home/mortal/file.txt" }));
    request.resolved_action_history = vec![vec![resolved_request_review(
        "review_sensitive_write",
        REVIEW_DECISION_DENIED,
    )]];

    let response = interceptor.intercept(&request).await.unwrap();

    assert_eq!(response.continuation, InterceptorContinuation::Stop);
    assert_eq!(response.actions.len(), 1);

    let action = &response.actions[0];

    assert_eq!(action.kind, ActionKind::from("reject_call"));
    assert_eq!(
        action.params,
        Some(json!({
            "error": {
                "code": -32051,
                "message": "user denied sensitive write"
            }
        }))
    );
}

#[tokio::test]
async fn denied_review_without_on_deny_uses_default_reject_call() {
    let interceptor = PolicyInterceptor::new(review_policy_config(vec![], vec![])).unwrap();

    let mut request = outbound_request("write_file", json!({ "path": "/home/mortal/file.txt" }));
    request.resolved_action_history = vec![vec![resolved_request_review(
        "review_sensitive_write",
        REVIEW_DECISION_DENIED,
    )]];

    let response = interceptor.intercept(&request).await.unwrap();

    assert_eq!(response.continuation, InterceptorContinuation::Stop);
    assert_eq!(response.actions.len(), 1);

    let action = &response.actions[0];

    assert_eq!(action.kind, ActionKind::from("reject_call"));
    assert_eq!(
        action.params,
        Some(json!({
            "error": {
                "code": -32050,
                "message": "operation denied by user review"
            }
        }))
    );
}

#[tokio::test]
async fn glob_matcher_matches_nested_param_path() {
    let interceptor = PolicyInterceptor::new(PolicyConfig {
        rules: vec![PolicyRule {
            name: "reject_secret_read".to_owned(),
            match_expr: MatchExpr::all(vec![
                MatchExpr::condition("message.method", PolicyMatcher::exact("read_file")),
                MatchExpr::condition("message.params.path", PolicyMatcher::glob("/secrets/**")),
            ]),
            apply: PolicyApply {
                immediate: vec![PolicyEffect::reject_call(json_rpc_error(
                    -32020,
                    "secret read denied",
                ))],
                review: None,
            },
        }],
    })
    .unwrap();

    let request = outbound_request("read_file", json!({ "path": "/secrets/key.txt" }));

    let response = interceptor.intercept(&request).await.unwrap();

    assert_eq!(response.continuation, InterceptorContinuation::Stop);
    assert_eq!(response.actions.len(), 1);
    assert_eq!(response.actions[0].kind, ActionKind::from("reject_call"));
}

#[tokio::test]
async fn regex_matcher_matches_method() {
    let interceptor = PolicyInterceptor::new(PolicyConfig {
        rules: vec![PolicyRule {
            name: "reject_write_methods".to_owned(),
            match_expr: MatchExpr::condition("message.method", PolicyMatcher::regex("^write_.*")),
            apply: PolicyApply {
                immediate: vec![PolicyEffect::reject_call(json_rpc_error(
                    -32021,
                    "write method denied",
                ))],
                review: None,
            },
        }],
    })
    .unwrap();

    let request = outbound_request("write_file", json!({ "path": "/tmp/a.txt" }));

    let response = interceptor.intercept(&request).await.unwrap();

    assert_eq!(response.continuation, InterceptorContinuation::Stop);
    assert_eq!(response.actions.len(), 1);
    assert_eq!(response.actions[0].kind, ActionKind::from("reject_call"));
}

#[tokio::test]
async fn negated_matcher_works() {
    let interceptor = PolicyInterceptor::new(PolicyConfig {
        rules: vec![PolicyRule {
            name: "reject_non_read".to_owned(),
            match_expr: MatchExpr::condition(
                "message.method",
                PolicyMatcher::exact_not("read_file"),
            ),
            apply: PolicyApply {
                immediate: vec![PolicyEffect::reject_call(json_rpc_error(
                    -32022,
                    "non-read denied",
                ))],
                review: None,
            },
        }],
    })
    .unwrap();

    let request = outbound_request("write_file", json!({ "path": "/tmp/a.txt" }));

    let response = interceptor.intercept(&request).await.unwrap();

    assert_eq!(response.continuation, InterceptorContinuation::Stop);
    assert_eq!(response.actions.len(), 1);
    assert_eq!(response.actions[0].kind, ActionKind::from("reject_call"));
}

#[test]
fn invalid_config_rejects_empty_rule_name() {
    let err = PolicyInterceptor::new(PolicyConfig {
        rules: vec![PolicyRule {
            name: "".to_owned(),
            match_expr: MatchExpr::condition("message.method", PolicyMatcher::exact("write_file")),
            apply: PolicyApply {
                immediate: vec![PolicyEffect::reject_call(json_rpc_error(-32010, "denied"))],
                review: None,
            },
        }],
    })
    .unwrap_err();

    assert!(err.to_string().contains("policy rule name cannot be empty"));
}

#[test]
fn invalid_config_rejects_duplicate_rule_names() {
    let rule = PolicyRule {
        name: "duplicate".to_owned(),
        match_expr: MatchExpr::condition("message.method", PolicyMatcher::exact("write_file")),
        apply: PolicyApply {
            immediate: vec![PolicyEffect::reject_call(json_rpc_error(-32010, "denied"))],
            review: None,
        },
    };

    let err = PolicyInterceptor::new(PolicyConfig {
        rules: vec![rule.clone(), rule],
    })
    .unwrap_err();

    assert!(err.to_string().contains("duplicate policy rule name"));
}

#[test]
fn invalid_config_rejects_empty_exclude_interceptors() {
    let err = PolicyInterceptor::new(PolicyConfig {
        rules: vec![PolicyRule {
            name: "bad_exclude".to_owned(),
            match_expr: MatchExpr::condition("message.method", PolicyMatcher::exact("write_file")),
            apply: PolicyApply {
                immediate: vec![PolicyEffect::exclude_interceptors(vec![])],
                review: None,
            },
        }],
    })
    .unwrap_err();

    assert!(
        err.to_string()
            .contains("exclude_interceptors effect with empty names")
    );
}

#[test]
fn invalid_config_rejects_empty_all_expression() {
    let err = PolicyInterceptor::new(PolicyConfig {
        rules: vec![PolicyRule {
            name: "bad_all".to_owned(),
            match_expr: MatchExpr::all(vec![]),
            apply: PolicyApply {
                immediate: vec![PolicyEffect::reject_call(json_rpc_error(-32010, "denied"))],
                review: None,
            },
        }],
    })
    .unwrap_err();

    assert!(err.to_string().contains("empty all expression"));
}

#[test]
fn invalid_config_rejects_empty_any_expression() {
    let err = PolicyInterceptor::new(PolicyConfig {
        rules: vec![PolicyRule {
            name: "bad_any".to_owned(),
            match_expr: MatchExpr::any(vec![]),
            apply: PolicyApply {
                immediate: vec![PolicyEffect::reject_call(json_rpc_error(-32010, "denied"))],
                review: None,
            },
        }],
    })
    .unwrap_err();

    assert!(err.to_string().contains("empty any expression"));
}

#[test]
fn invalid_config_rejects_rule_with_no_effects() {
    let err = PolicyInterceptor::new(PolicyConfig {
        rules: vec![PolicyRule {
            name: "no_effects".to_owned(),
            match_expr: MatchExpr::condition("message.method", PolicyMatcher::exact("write_file")),
            apply: PolicyApply {
                immediate: vec![],
                review: None,
            },
        }],
    })
    .unwrap_err();

    assert!(err.to_string().contains("has no effects"));
}

#[test]
fn invalid_config_rejects_invalid_regex() {
    let err = PolicyInterceptor::new(PolicyConfig {
        rules: vec![PolicyRule {
            name: "bad_regex".to_owned(),
            match_expr: MatchExpr::condition("message.method", PolicyMatcher::regex("[")),
            apply: PolicyApply {
                immediate: vec![PolicyEffect::reject_call(json_rpc_error(-32010, "denied"))],
                review: None,
            },
        }],
    })
    .unwrap_err();

    assert!(err.to_string().contains("invalid regex"));
}

fn review_policy_config(on_approve: Vec<PolicyEffect>, on_deny: Vec<PolicyEffect>) -> PolicyConfig {
    PolicyConfig {
        rules: vec![PolicyRule {
            name: "review_sensitive_write".to_owned(),
            match_expr: MatchExpr::all(vec![
                MatchExpr::condition("phase", PolicyMatcher::exact("outbound")),
                MatchExpr::condition("message.method", PolicyMatcher::exact("write_file")),
                MatchExpr::condition("message.params.path", PolicyMatcher::glob("/home/*/**")),
            ]),
            apply: PolicyApply {
                immediate: vec![],
                review: Some(PolicyReview {
                    title: "Sensitive file write".to_owned(),
                    reason: "Agent wants to write inside a user-owned directory.".to_owned(),
                    severity: PolicyReviewSeverity::High,
                    on_approve,
                    on_deny,
                }),
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

    InterceptionRequest {
        origin: Participant {
            kind: ParticipantType::Orchestrator,
            id: "orchestrator".to_owned(),
        },
        message: JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
            jsonrpc: JsonRpcVersion::V2_0,
            id: JsonRpcId::Number(1.into()),
            method: method.to_owned(),
            params,
        })),
        resolved_action_history: vec![],
    }
}

fn resolved_request_review(rule_name: &str, decision: &str) -> ResolvedActionRecord {
    ResolvedActionRecord {
        kind: ActionKind::from("request_review"),
        params: Some(json!({
            "rule_name": rule_name,
            "title": "Sensitive file write",
            "reason": "Agent wants to write inside a user-owned directory.",
            "severity": REVIEW_SEVERITY_HIGH
        })),
        result: Ok(Some(json!({
            "decision": decision
        }))),
    }
}

fn json_rpc_error(code: i32, message: &str) -> actrpc_core::json_rpc::JsonRpcError {
    actrpc_core::json_rpc::JsonRpcError {
        code,
        message: message.to_owned(),
        data: None,
    }
}
