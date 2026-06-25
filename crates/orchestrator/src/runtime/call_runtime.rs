use crate::{
    error::OrchestratorError,
    runtime::{CurrentCallRejection, InFlightMessageState, TranscriptState},
    transcript::{TranscriptEntryInput, TranscriptParticipant},
};
use actrpc_core::{
    CallContext, CallId, MethodTarget, json_rpc::JsonRpcMessage, participant::Participant,
};
use std::sync::Arc;

#[derive(Debug)]
pub struct CallRuntime {
    pub in_flight_message: Arc<InFlightMessageState>,
    pub rejection: Arc<CurrentCallRejection>,
    pub transcript: Arc<TranscriptState>,
    call_id: CallId,
    parent_call_id: Option<CallId>,
    root_call_id: CallId,
    depth: usize,
    origin: Participant,
    target: MethodTarget,
    call_ctx: Option<CallContext>,
}

impl CallRuntime {
    pub fn root(
        message: JsonRpcMessage,
        transcript: Arc<TranscriptState>,
        call_id: CallId,
        origin: Participant,
        target: MethodTarget,
    ) -> Self {
        Self::new(
            message, transcript, call_id, None, call_id, 0, origin, target, None,
        )
    }

    pub fn nested(
        message: JsonRpcMessage,
        transcript: Arc<TranscriptState>,
        call_id: CallId,
        parent_call_id: CallId,
        root_call_id: CallId,
        depth: usize,
        origin: Participant,
        target: MethodTarget,
        call_ctx: Option<CallContext>,
    ) -> Self {
        Self::new(
            message,
            transcript,
            call_id,
            Some(parent_call_id),
            root_call_id,
            depth,
            origin,
            target,
            call_ctx,
        )
    }

    fn new(
        message: JsonRpcMessage,
        transcript: Arc<TranscriptState>,
        call_id: CallId,
        parent_call_id: Option<CallId>,
        root_call_id: CallId,
        depth: usize,
        origin: Participant,
        target: MethodTarget,
        call_ctx: Option<CallContext>,
    ) -> Self {
        let in_flight_message = Arc::new(InFlightMessageState::new());
        in_flight_message.set_message(message);

        Self {
            in_flight_message,
            rejection: Arc::new(CurrentCallRejection::new()),
            transcript,
            call_id,
            parent_call_id,
            root_call_id,
            depth,
            origin,
            target,
            call_ctx,
        }
    }

    pub fn call_ctx(&self) -> Option<&CallContext> {
        self.call_ctx.as_ref()
    }

    pub fn call_id(&self) -> CallId {
        self.call_id
    }

    pub fn parent_call_id(&self) -> Option<CallId> {
        self.parent_call_id
    }

    pub fn root_call_id(&self) -> CallId {
        self.root_call_id
    }

    pub fn depth(&self) -> usize {
        self.depth
    }

    pub fn origin(&self) -> &Participant {
        &self.origin
    }

    pub fn target(&self) -> &MethodTarget {
        &self.target
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
