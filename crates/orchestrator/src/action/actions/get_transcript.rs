use crate::{
    action::{ActionHandlerFuture, TypedActionHandler},
    error::ActionExecutionError,
    runtime::TranscriptState,
    transcript::TranscriptEntryView,
};
use actrpc_core::{
    action::{ActionSpec, NoParams, RequestedAction, ResolvedAction},
    interception::InterceptionRequest,
};
use std::sync::Arc;

pub struct GetTranscript;

impl ActionSpec for GetTranscript {
    type Params = NoParams;
    type Result = Vec<TranscriptEntryView>;

    const KIND: &'static str = "get_transcript";
}

pub struct GetTranscriptHandler {
    transcript: Arc<TranscriptState>,
}

impl GetTranscriptHandler {
    pub fn new(transcript: Arc<TranscriptState>) -> Self {
        Self { transcript }
    }
}

impl TypedActionHandler<GetTranscript> for GetTranscriptHandler {
    fn handle_typed<'a>(
        &'a self,
        _request: &'a InterceptionRequest,
        action: RequestedAction<GetTranscript>,
    ) -> ActionHandlerFuture<'a, Result<ResolvedAction<GetTranscript>, ActionExecutionError>>
    where
        Self: 'a,
    {
        Box::pin(async move {
            let entries = self.transcript.snapshot();

            Ok(ResolvedAction {
                params: action.params,
                result: Ok(entries),
            })
        })
    }
}
