use crate::runtime::{CurrentCallRejection, InFlightMessageState, TranscriptState};
use actrpc_core::json_rpc::JsonRpcMessage;
use std::sync::Arc;

#[derive(Debug)]
pub struct CallRuntime {
    pub in_flight_message: Arc<InFlightMessageState>,
    pub rejection: Arc<CurrentCallRejection>,
    pub transcript: Arc<TranscriptState>,
    depth: usize,
}

impl CallRuntime {
    pub fn root(message: JsonRpcMessage) -> Self {
        Self::new(message, Arc::new(TranscriptState::new()), 0)
    }

    pub fn nested(
        message: JsonRpcMessage,
        transcript: Arc<TranscriptState>,
        parent_depth: usize,
    ) -> Self {
        Self::new(message, transcript, parent_depth + 1)
    }

    fn new(message: JsonRpcMessage, transcript: Arc<TranscriptState>, depth: usize) -> Self {
        let in_flight_message = Arc::new(InFlightMessageState::new());
        in_flight_message.set_message(message);

        Self {
            in_flight_message,
            rejection: Arc::new(CurrentCallRejection::new()),
            transcript,
            depth,
        }
    }

    pub fn depth(&self) -> usize {
        self.depth
    }
}
