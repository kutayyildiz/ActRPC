use crate::{interceptor::WorkingInterceptorPipeline, runtime::CallRuntime};
use actrpc_core::interception::InterceptionPhase;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct PhaseRuntime {
    pub phase: InterceptionPhase,
    pub call: Arc<CallRuntime>,
    pub pipeline: Arc<WorkingInterceptorPipeline>,
}

impl PhaseRuntime {
    pub fn new(
        phase: InterceptionPhase,
        call: Arc<CallRuntime>,
        pipeline: WorkingInterceptorPipeline,
    ) -> Self {
        Self {
            phase,
            call,
            pipeline: Arc::new(pipeline),
        }
    }
}
