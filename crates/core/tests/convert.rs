use actrpc_core::{
    action::{ActionKind, RequestedActionRecord},
    error::{ActRpcError, ProtocolError},
    interception::{InterceptionRequest, InterceptionResponse, InterceptorContinuation},
    json_rpc::{
        JsonRpcError, JsonRpcErrorResponse, JsonRpcId, JsonRpcMessage, JsonRpcParams,
        JsonRpcRequest, JsonRpcResponse, JsonRpcSingleMessage, JsonRpcVersion,
    },
    participant::{Participant, ParticipantType},
};
use serde_json::json;

#[test]
fn test_interception_request_into_json_rpc_request() {
    let payload = InterceptionRequest {
        origin: Participant {
            kind: ParticipantType::User,
            id: "cli-123".to_string(),
        },
        message: JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
            jsonrpc: JsonRpcVersion::V2_0,
            id: JsonRpcId::Number(99.into()),
            method: "subtract".to_string(),
            params: Some(JsonRpcParams::Array(vec![json!(10), json!(3)])),
        })),
        resolved_action_history: Default::default(),
    };

    let req: JsonRpcRequest = (JsonRpcId::Number(1.into()), payload.clone()).into();

    assert_eq!(req.jsonrpc, JsonRpcVersion::V2_0);
    assert_eq!(req.id, JsonRpcId::Number(1.into()));
    assert_eq!(req.method, "intercept");

    let (roundtrip_id, roundtrip_payload): (JsonRpcId, InterceptionRequest) =
        req.try_into().unwrap();

    assert_eq!(roundtrip_id, JsonRpcId::Number(1.into()));
    assert_eq!(roundtrip_payload, payload);
}

#[test]
fn test_json_rpc_request_try_into_interception_request_rejects_wrong_method() {
    let req = JsonRpcRequest {
        jsonrpc: JsonRpcVersion::V2_0,
        id: JsonRpcId::Number(1.into()),
        method: "not_intercept".to_string(),
        params: Some(JsonRpcParams::Object(serde_json::Map::new())),
    };

    let err = <(JsonRpcId, InterceptionRequest)>::try_from(req).unwrap_err();

    match err {
        ActRpcError::Protocol(ProtocolError::UnexpectedMethod { expected, actual }) => {
            assert_eq!(expected, "intercept");
            assert_eq!(actual, "not_intercept");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn test_json_rpc_request_try_into_interception_request_rejects_missing_params() {
    let req = JsonRpcRequest {
        jsonrpc: JsonRpcVersion::V2_0,
        id: JsonRpcId::Number(1.into()),
        method: "intercept".to_string(),
        params: None,
    };

    let err = <(JsonRpcId, InterceptionRequest)>::try_from(req).unwrap_err();
    assert!(matches!(
        err,
        ActRpcError::Protocol(ProtocolError::InvalidRequestParams)
    ));
}

#[test]
fn test_json_rpc_request_try_into_interception_request_rejects_array_params() {
    let req = JsonRpcRequest {
        jsonrpc: JsonRpcVersion::V2_0,
        id: JsonRpcId::Number(1.into()),
        method: "intercept".to_string(),
        params: Some(JsonRpcParams::Array(vec![json!(1)])),
    };

    let err = <(JsonRpcId, InterceptionRequest)>::try_from(req).unwrap_err();
    assert!(matches!(
        err,
        ActRpcError::Protocol(ProtocolError::InvalidRequestParams)
    ));
}

#[test]
fn test_interception_response_into_json_rpc_response() {
    let payload = InterceptionResponse {
        actions: vec![RequestedActionRecord {
            kind: ActionKind::from("log"),
            params: Some(json!({ "message": "ok" })),
        }],
        continuation: InterceptorContinuation::Reinvoke,
    };

    let resp: JsonRpcResponse = (JsonRpcId::Number(5.into()), payload.clone()).into();

    let (roundtrip_id, roundtrip_payload): (JsonRpcId, InterceptionResponse) =
        resp.try_into().unwrap();

    assert_eq!(roundtrip_id, JsonRpcId::Number(5.into()));
    assert_eq!(roundtrip_payload, payload);
}

#[test]
fn test_json_rpc_response_try_into_interception_response_returns_remote_error() {
    let resp = JsonRpcResponse::Error(JsonRpcErrorResponse {
        jsonrpc: JsonRpcVersion::V2_0,
        id: JsonRpcId::Number(5.into()),
        error: JsonRpcError {
            code: -32000,
            message: "boom".to_string(),
            data: Some(json!({ "detail": "failed" })),
        },
    });

    let err = <(JsonRpcId, InterceptionResponse)>::try_from(resp).unwrap_err();

    match err {
        ActRpcError::RemoteJsonRpc(error) => {
            assert_eq!(error.code, -32000);
            assert_eq!(error.message, "boom");
            assert_eq!(error.data, Some(json!({ "detail": "failed" })));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn test_json_rpc_response_try_into_interception_response_rejects_invalid_payload() {
    let resp = JsonRpcResponse::Success(actrpc_core::json_rpc::JsonRpcSuccessResponse {
        jsonrpc: JsonRpcVersion::V2_0,
        id: JsonRpcId::Number(5.into()),
        result: json!({ "actions": "not-an-array", "continuation": "stop" }),
    });

    let err = <(JsonRpcId, InterceptionResponse)>::try_from(resp).unwrap_err();

    match err {
        ActRpcError::Codec(_) => {}
        other => panic!("unexpected error: {other:?}"),
    }
}
