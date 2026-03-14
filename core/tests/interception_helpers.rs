use actrpc_core::{
    interception::{InterceptionDecision, InterceptionRequest},
    interception_helpers::*,
    json_rpc::{JsonRpcMessage, JsonRpcRequest, JsonRpcResponse},
    participant::{Participant, ParticipantType},
};
use serde_json::{Value, json};

fn dummy_outbound_request() -> InterceptionRequest {
    InterceptionRequest {
        origin: Participant {
            kind: ParticipantType::User,
            id: "user-cli-001".to_string(),
        },
        message: JsonRpcMessage::Request(JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::Number(7.into())),
            method: "chat.completions.create".to_string(),
            params: json!({
                "model": "gpt-4o-mini",
                "messages": [{"role": "user", "content": "Hello"}]
            }),
        }),
        executed_actions: None,
    }
}

#[test]
fn next_jsonrpc_id_increments_and_returns_number() {
    let id1 = new_jsonrpc_id();
    let id2 = new_jsonrpc_id();
    let id3 = new_jsonrpc_id();

    assert!(matches!(id1, Value::Number(_)));
    assert!(matches!(id2, Value::Number(_)));
    assert!(matches!(id3, Value::Number(_)));

    if let (Value::Number(n1), Value::Number(n2), Value::Number(n3)) = (id1, id2, id3) {
        let v1 = n1.as_u64().unwrap();
        let v2 = n2.as_u64().unwrap();
        let v3 = n3.as_u64().unwrap();

        assert!(v2 == v1 + 1);
        assert!(v3 == v2 + 1);
    } else {
        panic!("IDs should be JSON numbers");
    }
}

#[test]
fn create_interception_request_sets_correct_structure() {
    let interception = dummy_outbound_request();
    let custom_id = Some(json!("test-abc-123"));

    let req = create_interception_request(interception.clone(), custom_id.clone());

    assert_eq!(req.jsonrpc, "2.0");
    assert_eq!(req.method, "intercept");
    assert_eq!(req.id, custom_id);

    let params = req.params;
    assert!(params.is_object());

    let obj = params.as_object().unwrap();
    assert!(obj.contains_key("origin"));
    assert!(obj.contains_key("message"));
    assert!(!obj.contains_key("executed_actions")); // None → skipped

    let origin = obj.get("origin").unwrap().as_object().unwrap();
    assert_eq!(origin.get("kind").unwrap(), "user");
    assert_eq!(origin.get("id").unwrap(), "user-cli-001");
}

#[test]
fn interception_request_into_interception_request_works() {
    let interception = dummy_outbound_request();
    let id = Some(json!(9001));

    let req = interception.into_rpc_request(id.clone());

    assert_eq!(req.id, id);
    assert_eq!(req.method, "intercept");
    assert!(req.params.get("origin").is_some());
}

#[test]
fn create_interception_request_with_executed_actions() {
    let mut interception = dummy_outbound_request();
    interception.executed_actions = Some(vec![]); // empty vec → should be serialized

    let req = interception.into_rpc_request(None);

    let params = req.params.as_object().unwrap();
    let actions = params.get("executed_actions").unwrap().as_array().unwrap();
    assert_eq!(actions.len(), 0);
}

#[test]
fn create_interception_response_basic_success() {
    let decision = InterceptionDecision {
        actions: Some(vec![
            "modify_params".to_string(),
            "call_external_method".to_string(),
        ]),
        is_final: false,
    };

    let request_id = Some(json!(1337));

    let resp = create_interception_response(decision.clone(), request_id.clone());

    match resp {
        JsonRpcResponse::Success {
            jsonrpc,
            id,
            result,
        } => {
            assert_eq!(jsonrpc, "2.0");
            assert_eq!(id, request_id);

            let obj = result.as_object().unwrap();
            assert_eq!(obj.get("is_final").unwrap().as_bool().unwrap(), false);

            let acts = obj.get("actions").unwrap().as_array().unwrap();
            assert_eq!(acts.len(), 2);
            assert_eq!(acts[0].as_str().unwrap(), "modify_params");
        }
        _ => panic!("Expected Success variant"),
    }
}

#[test]
fn interception_decision_into_interception_response_preserves_id() {
    let decision = InterceptionDecision {
        actions: None,
        is_final: true,
    };

    let resp = decision.into_rpc_response(Some(json!("from-incoming-request")));

    match resp {
        JsonRpcResponse::Success { id, result, .. } => {
            assert_eq!(id, Some(json!("from-incoming-request")));
            assert_eq!(result.get("is_final").unwrap().as_bool().unwrap(), true);
            assert!(result.get("actions").is_none());
        }
        _ => panic!("Expected Success"),
    }
}

#[test]
fn create_interception_response_with_no_actions() {
    let decision = InterceptionDecision {
        actions: None,
        is_final: true,
    };

    let resp = create_interception_response(decision, None);

    if let JsonRpcResponse::Success { result, .. } = resp {
        assert!(result.get("actions").is_none());
        assert!(result.get("is_final").unwrap().as_bool().unwrap());
    } else {
        panic!("should be success");
    }
}
