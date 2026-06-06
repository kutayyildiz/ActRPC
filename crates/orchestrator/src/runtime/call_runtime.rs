use crate::{
    error::OrchestratorError,
    runtime::{CurrentCallRejection, InFlightMessageState, TranscriptState},
    transcript::{CallId, TranscriptEntryInput, TranscriptParticipant},
};
use actrpc_core::json_rpc::JsonRpcMessage;
use std::sync::Arc;

#[derive(Debug)]
pub struct CallRuntime {
    pub in_flight_message: Arc<InFlightMessageState>,
    pub rejection: Arc<CurrentCallRejection>,
    pub transcript: Arc<TranscriptState>,
    call_id: CallId,
    parent_call_id: Option<CallId>,
    depth: usize,
}

impl CallRuntime {
    pub fn root(
        message: JsonRpcMessage,
        transcript: Arc<TranscriptState>,
        call_id: CallId,
    ) -> Self {
        Self::new(message, transcript, call_id, None, 0)
    }

    pub fn nested(
        message: JsonRpcMessage,
        transcript: Arc<TranscriptState>,
        call_id: CallId,
        parent_call_id: CallId,
        depth: usize,
    ) -> Self {
        Self::new(message, transcript, call_id, Some(parent_call_id), depth)
    }

    fn new(
        message: JsonRpcMessage,
        transcript: Arc<TranscriptState>,
        call_id: CallId,
        parent_call_id: Option<CallId>,
        depth: usize,
    ) -> Self {
        let in_flight_message = Arc::new(InFlightMessageState::new());
        in_flight_message.set_message(message);

        Self {
            in_flight_message,
            rejection: Arc::new(CurrentCallRejection::new()),
            transcript,
            call_id,
            parent_call_id,
            depth,
        }
    }

    pub fn call_id(&self) -> CallId {
        self.call_id
    }

    pub fn parent_call_id(&self) -> Option<CallId> {
        self.parent_call_id
    }

    pub fn depth(&self) -> usize {
        self.depth
    }

    pub fn record_exchange(
        &self,
        from: TranscriptParticipant,
        to: TranscriptParticipant,
        protocol: &'static str,
        message: serde_json::Value,
    ) -> Result<(), OrchestratorError> {
        self.transcript
            .append(TranscriptEntryInput {
                call_id: self.call_id,
                parent_call_id: self.parent_call_id,
                depth: self.depth,
                from,
                to,
                protocol,
                message,
            })
            .map_err(OrchestratorError::from)
    }
}
