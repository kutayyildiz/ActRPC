use actrpc_core::{action::Action, json_rpc::JsonRpcError};
use serde_json::{Value, json};

#[test]
fn test_action_serde_roundtrip() {
    let cases: Vec<(Action, Value)> = vec![
        (
            Action::ModifyParams(json!({"temperature": 0.03, "max_tokens": 2048})),
            json!({
                "action": "modify_params",
                "action_params": {"temperature": 0.03, "max_tokens": 2048}
            }),
        ),
        (
            Action::ModifyError(JsonRpcError {
                code: 5,
                message: "error".to_string(),
                data: None,
            }),
            json!({
                "action": "modify_error",
                "action_params": {"code": 5, "message": "error"}
            }),
        ),
        (
            Action::CallExternalMethod {
                method_name: "run_prompt".to_string(),
                method_params: json!({"prompt": "What is 2 + 2?", "model": "o2-mini"}),
            },
            json!({
                "action": "call_external_method",
                "action_params": {
                    "method_name": "run_prompt",
                    "method_params": {"prompt": "What is 2 + 2?", "model": "o2-mini"}
                }
            }),
        ),
        (
            Action::EnableInterceptors(vec!["audit".to_string(), "safety".to_string()]),
            json!({
                "action": "enable_interceptors",
                "action_params": ["audit", "safety"]
            }),
        ),
        (
            Action::ListExternalMethods,
            json!({"action": "list_external_methods"}),
        ),
    ];

    for (action, expected) in cases {
        let ser = serde_json::to_value(&action).unwrap();
        assert_eq!(ser, expected);

        let de: Action = serde_json::from_value(expected).unwrap();
        assert_eq!(de, action);
    }
}

#[test]
fn test_unknown_action_fails() {
    let bad = json!({"action": "delete_everything", "action_params": null});
    let err = serde_json::from_value::<Action>(bad);
    assert!(err.is_err());
}
