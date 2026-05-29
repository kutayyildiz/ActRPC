use actrpc_core::{
    action::ActionSpec,
    participant::{Participant, ParticipantType},
};
use actrpc_orchestrator::{
    TranscriptEntry,
    action::{
        ActionRegistry,
        actions::get_transcript::{GetTranscript, GetTranscriptHandler},
    },
    runtime::TranscriptState,
};
use std::sync::Arc;

use super::super::helpers::{dummy_request, no_params_action_record, request_message};

#[tokio::test]
async fn get_transcript_returns_transcript_snapshot() {
    let transcript = Arc::new(TranscriptState::new());

    transcript
        .append(
            TranscriptEntry {
                from: Participant {
                    kind: ParticipantType::User,
                    id: "cli".to_owned(),
                },
                to: Participant {
                    kind: ParticipantType::Orchestrator,
                    id: "main".to_owned(),
                },
                seq: 1,
                ts: 123.0,
                message: request_message("ping", None),
            }
            .into(),
        )
        .unwrap();

    let mut registry = ActionRegistry::new();
    registry
        .register::<GetTranscript, _>(GetTranscriptHandler::new(transcript))
        .unwrap();

    let resolved = registry
        .get(&GetTranscript::action_kind())
        .unwrap()
        .handle(&dummy_request(), no_params_action_record::<GetTranscript>())
        .await
        .unwrap();

    let value = resolved.result.unwrap().unwrap();

    assert_eq!(value[0]["from"], "user:cli");
    assert_eq!(value[0]["to"], "orchestrator:main");
    assert_eq!(value[0]["seq"], 1);
}
