use actrpc_core::{
    action::{ActionKind, RequestedActionRecord, ResolvedActionRecord},
    error::ProtocolError,
    interception::{
        InterceptionPhase, InterceptionRequest, InterceptionResponse, InterceptorContinuation,
    },
    json_rpc::{
        JsonRpcBatch, JsonRpcId, JsonRpcMessage, JsonRpcNotification, JsonRpcParams,
        JsonRpcRequest, JsonRpcResponse, JsonRpcSingleMessage, JsonRpcSuccessResponse,
        JsonRpcVersion,
    },
    participant::{Participant, ParticipantType},
};
use serde_json::json;

#[test]
fn test_interception_request_phase_detection_single_messages() {
    let req = InterceptionRequest {
        origin: Participant {
            kind: ParticipantType::User,
            id: "cli-123".to_string(),
        },
        message: JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
            jsonrpc: JsonRpcVersion::V2_0,
            id: JsonRpcId::Number(1.into()),
            method: "say_hello".to_string(),
            params: Some(JsonRpcParams::Array(vec![json!(1), json!("asd")])),
        })),
        resolved_action_history: Default::default(),
    };

    assert_eq!(req.phase(), Ok(InterceptionPhase::Outbound));
    assert!(!req.has_prior_actions());

    let notif = InterceptionRequest {
        origin: Participant {
            kind: ParticipantType::User,
            id: "cli-123".to_string(),
        },
        message: JsonRpcMessage::Single(JsonRpcSingleMessage::Notification(JsonRpcNotification {
            jsonrpc: JsonRpcVersion::V2_0,
            method: "ping".to_string(),
            params: None,
        })),
        resolved_action_history: Default::default(),
    };

    assert_eq!(notif.phase(), Ok(InterceptionPhase::Outbound));

    let resp = InterceptionRequest {
        origin: Participant {
            kind: ParticipantType::Orchestrator,
            id: "main".to_string(),
        },
        message: JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Success(
            JsonRpcSuccessResponse {
                jsonrpc: JsonRpcVersion::V2_0,
                id: JsonRpcId::Number(1.into()),
                result: json!("world"),
            },
        ))),
        resolved_action_history: Default::default(),
    };

    assert_eq!(resp.phase(), Ok(InterceptionPhase::Inbound));
    assert!(!resp.has_prior_actions());
}

#[test]
fn test_interception_request_phase_detection_batch_outbound() {
    let req = InterceptionRequest {
        origin: Participant {
            kind: ParticipantType::User,
            id: "cli-123".to_string(),
        },
        message: JsonRpcMessage::Batch(JsonRpcBatch(vec![
            JsonRpcSingleMessage::Request(JsonRpcRequest {
                jsonrpc: JsonRpcVersion::V2_0,
                id: JsonRpcId::Number(1.into()),
                method: "sum".to_string(),
                params: Some(JsonRpcParams::Array(vec![json!(1), json!(2)])),
            }),
            JsonRpcSingleMessage::Notification(JsonRpcNotification {
                jsonrpc: JsonRpcVersion::V2_0,
                method: "ping".to_string(),
                params: None,
            }),
        ])),
        resolved_action_history: Default::default(),
    };

    assert_eq!(req.phase(), Ok(InterceptionPhase::Outbound));
}

#[test]
fn test_interception_request_phase_detection_batch_inbound() {
    let req = InterceptionRequest {
        origin: Participant {
            kind: ParticipantType::Orchestrator,
            id: "main".to_string(),
        },
        message: JsonRpcMessage::Batch(JsonRpcBatch(vec![
            JsonRpcSingleMessage::Response(JsonRpcResponse::Success(JsonRpcSuccessResponse {
                jsonrpc: JsonRpcVersion::V2_0,
                id: JsonRpcId::Number(1.into()),
                result: json!(3),
            })),
            JsonRpcSingleMessage::Response(JsonRpcResponse::Success(JsonRpcSuccessResponse {
                jsonrpc: JsonRpcVersion::V2_0,
                id: JsonRpcId::Number(2.into()),
                result: json!("ok"),
            })),
        ])),
        resolved_action_history: Default::default(),
    };

    assert_eq!(req.phase(), Ok(InterceptionPhase::Inbound));
}

#[test]
fn test_interception_request_phase_detection_batch_mixed_is_invalid() {
    let req = InterceptionRequest {
        origin: Participant {
            kind: ParticipantType::Orchestrator,
            id: "main".to_string(),
        },
        message: JsonRpcMessage::Batch(JsonRpcBatch(vec![
            JsonRpcSingleMessage::Request(JsonRpcRequest {
                jsonrpc: JsonRpcVersion::V2_0,
                id: JsonRpcId::Number(1.into()),
                method: "sum".to_string(),
                params: None,
            }),
            JsonRpcSingleMessage::Response(JsonRpcResponse::Success(JsonRpcSuccessResponse {
                jsonrpc: JsonRpcVersion::V2_0,
                id: JsonRpcId::Number(1.into()),
                result: json!(3),
            })),
        ])),
        resolved_action_history: Default::default(),
    };

    assert_eq!(req.phase(), Err(ProtocolError::MixedBatch));
}

#[test]
fn test_interception_request_has_prior_actions() {
    let req = InterceptionRequest {
        origin: Participant {
            kind: ParticipantType::Interceptor,
            id: "safety-v3".to_string(),
        },
        message: JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
            jsonrpc: JsonRpcVersion::V2_0,
            id: JsonRpcId::Number(7.into()),
            method: "do_work".to_string(),
            params: None,
        })),
        resolved_action_history: vec![vec![ResolvedActionRecord {
            kind: ActionKind::from("log"),
            params: Some(json!({ "message": "checked" })),
            result: Err(ProtocolError::InvalidMessageDirection {
                reason: "noop".to_string(),
            }),
        }]],
    };

    assert!(req.has_prior_actions());
}

#[test]
fn test_interception_request_serde_minimal_skips_empty_resolved_action_history() {
    let req = InterceptionRequest {
        origin: Participant {
            kind: ParticipantType::User,
            id: "cli-123".to_string(),
        },
        message: JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
            jsonrpc: JsonRpcVersion::V2_0,
            id: JsonRpcId::Number(1.into()),
            method: "subtract".to_string(),
            params: Some(JsonRpcParams::Array(vec![json!(4), json!(2)])),
        })),
        resolved_action_history: Default::default(),
    };

    let ser = serde_json::to_string(&req).unwrap();
    assert!(!ser.contains("resolved_action_history"));

    let de: InterceptionRequest = serde_json::from_str(&ser).unwrap();
    assert!(de.resolved_action_history.is_empty());
    assert_eq!(de.phase(), Ok(InterceptionPhase::Outbound));
}

#[test]
fn test_interception_response_serde_minimal_skips_empty_actions() {
    let decision = InterceptionResponse {
        actions: vec![],
        continuation: InterceptorContinuation::Stop,
    };

    let ser = serde_json::to_string(&decision).unwrap();
    assert_eq!(ser, r#"{"continuation":"stop"}"#);

    let de: InterceptionResponse = serde_json::from_str(&ser).unwrap();
    assert!(de.actions.is_empty());
    assert!(de.should_stop());
    assert!(!de.should_reinvoke());
}

#[test]
fn test_interception_response_helpers_with_actions() {
    let response = InterceptionResponse {
        actions: vec![RequestedActionRecord {
            kind: ActionKind::from("notify"),
            params: Some(json!({ "channel": "audit" })),
        }],
        continuation: InterceptorContinuation::Reinvoke,
    };

    assert!(response.has_actions());
    assert!(response.should_reinvoke());
    assert!(!response.should_stop());

    let ser = serde_json::to_string(&response).unwrap();
    let de: InterceptionResponse = serde_json::from_str(&ser).unwrap();
    assert_eq!(de, response);
}
