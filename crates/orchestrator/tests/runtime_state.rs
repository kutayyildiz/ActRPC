use actrpc_core::json_rpc::{
    JsonRpcError, JsonRpcId, JsonRpcMessage, JsonRpcRequest, JsonRpcSingleMessage, JsonRpcVersion,
};
use actrpc_orchestrator::{
    CallId, PROTOCOL_INTERCEPTOR_REQUEST, TranscriptEntryInput, TranscriptParticipant,
    runtime::{CurrentCallRejection, InFlightMessageState, TranscriptState},
};
use serde_json::json;

#[test]
fn in_flight_message_state_set_snapshot_replace_clear() {
    let state = InFlightMessageState::new();

    assert_eq!(state.snapshot(), None);

    let first = request_message("first");
    let second = request_message("second");

    state.set_message(first.clone());

    assert_eq!(state.snapshot(), Some(first));
    assert!(state.replace_message(second.clone()));
    assert_eq!(state.snapshot(), Some(second));

    state.clear();

    assert_eq!(state.snapshot(), None);
    assert!(!state.replace_message(request_message("third")));
}

#[test]
fn current_call_rejection_set_snapshot_clear() {
    let rejection = CurrentCallRejection::new();

    assert!(!rejection.is_rejected());
    assert_eq!(rejection.snapshot(), None);

    let error = JsonRpcError {
        code: -32000,
        message: "blocked".to_owned(),
        data: Some(json!({ "reason": "test" })),
    };

    rejection.set(error.clone());

    assert!(rejection.is_rejected());
    assert_eq!(rejection.snapshot(), Some(error));

    rejection.clear();

    assert!(!rejection.is_rejected());
    assert_eq!(rejection.snapshot(), None);
}

#[test]
fn transcript_state_appends_and_snapshots_entries() {
    let state = TranscriptState::new();

    assert!(state.is_empty().unwrap());
    assert_eq!(state.len().unwrap(), 0);

    let call_id = CallId::new();

    state
        .append(TranscriptEntryInput {
            call_id,
            parent_call_id: None,
            depth: 0,
            from: TranscriptParticipant::orchestrator_main(),
            to: TranscriptParticipant::interceptor("policy"),
            protocol: PROTOCOL_INTERCEPTOR_REQUEST,
            message: json!({ "origin": { "kind": "orchestrator", "id": "main" } }),
        })
        .unwrap();

    assert!(!state.is_empty().unwrap());
    assert_eq!(state.len().unwrap(), 1);

    let snapshot = state.snapshot().unwrap();

    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot[0].from, "orchestrator:main");
    assert_eq!(snapshot[0].to, "interceptor:policy");
    assert_eq!(snapshot[0].seq, 1);
    assert!(snapshot[0].ts_ms > 0);
    assert_eq!(snapshot[0].call_id, call_id);
    assert_eq!(snapshot[0].parent_call_id, None);
    assert_eq!(snapshot[0].depth, 0);
    assert_eq!(snapshot[0].protocol, PROTOCOL_INTERCEPTOR_REQUEST);
}

fn request_message(method: &str) -> JsonRpcMessage {
    JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
        jsonrpc: JsonRpcVersion::V2_0,
        id: JsonRpcId::Number(1.into()),
        method: method.to_owned(),
        params: None,
    }))
}
