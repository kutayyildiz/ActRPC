use actrpc_core::action::ActionSpec;
use actrpc_orchestrator::{
    CallId, PROTOCOL_METHOD_REQUEST, TranscriptEntryInput, TranscriptEntryView,
    TranscriptParticipant, TranscriptParticipantKind,
    action::{
        ActionRegistry,
        actions::get_transcript::{GetTranscript, GetTranscriptHandler},
    },
    runtime::TranscriptState,
};
use serde_json::json;
use std::sync::Arc;

use super::super::helpers::{dummy_request, invocation_context, no_params_action_record};

#[tokio::test]
async fn get_transcript_returns_transcript_snapshot() {
    let transcript = Arc::new(TranscriptState::new());

    let call_id = CallId::new();

    transcript
        .append(TranscriptEntryInput {
            call_id,
            parent_call_id: None,
            depth: 0,
            from: TranscriptParticipant {
                kind: TranscriptParticipantKind::User,
                id: "cli".to_owned(),
            },
            to: TranscriptParticipant::orchestrator_main(),
            protocol: PROTOCOL_METHOD_REQUEST,
            message: json!({ "jsonrpc": "2.0", "method": "ping" }),
        })
        .unwrap();

    let mut registry = ActionRegistry::new();
    registry
        .register::<GetTranscript, _>(GetTranscriptHandler::new(transcript))
        .unwrap();

    let resolved = registry
        .get(&GetTranscript::action_kind())
        .unwrap()
        .handle(
            &dummy_request(),
            no_params_action_record::<GetTranscript>(),
            &invocation_context("test"),
        )
        .await
        .unwrap();

    let value: Vec<TranscriptEntryView> =
        serde_json::from_value(resolved.result.unwrap().unwrap()).unwrap();

    assert_eq!(value[0].from, "user:cli");
    assert_eq!(value[0].to, "orchestrator:main");
    assert_eq!(value[0].seq, 1);
    assert_eq!(value[0].protocol, PROTOCOL_METHOD_REQUEST);
}
