use actrpc_core::{
    interception::{InterceptionDecision, InterceptionRequest},
    json_rpc::{JsonRpcMessage, JsonRpcRequest, JsonRpcResponse},
    participant::{Participant, ParticipantType},
    phase::Phase,
};
use serde_json::json;

#[cfg(test)]
#[test]
fn test_interception_request_phase_detection() {
    let req = InterceptionRequest {
        origin: Participant {
            kind: ParticipantType::User,
            id: "cli-123".to_string(),
        },
        message: JsonRpcMessage::Request(JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(1.into()),
            method: "say_hello".to_string(),
            params: json!({}),
        }),
        executed_actions: None,
    };

    assert_eq!(req.phase(), Phase::Outbound);
    assert!(!req.has_previous_actions());

    let resp = InterceptionRequest {
        origin: Participant {
            kind: ParticipantType::Orchestrator,
            id: "main".to_string(),
        },
        message: JsonRpcMessage::Response(JsonRpcResponse::Success {
            jsonrpc: "2.0".to_string(),
            id: Some(1.into()),
            result: json!("world"),
        }),
        executed_actions: Some(vec![]),
    };

    assert_eq!(resp.phase(), Phase::Inbound);
    assert!(!resp.has_previous_actions());
}

#[test]
fn test_interception_decision_serde_minimal() {
    let decision = InterceptionDecision {
        actions: None,
        is_final: true,
    };

    let ser = serde_json::to_string(&decision).unwrap();
    assert_eq!(ser, r#"{"is_final":true}"#); // actions skipped because None

    let de: InterceptionDecision = serde_json::from_str(&ser).unwrap();
    assert!(de.actions.is_none());
    assert!(de.is_final);
}
