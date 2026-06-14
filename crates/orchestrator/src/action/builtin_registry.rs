use crate::{
    action::{
        ActionRegistry,
        actions::{
            call_method::{CallMethod, CallMethodHandler},
            exclude_interceptors::{ExcludeInterceptors, ExcludeInterceptorsHandler},
            get_interceptor_catalog::{GetInterceptorCatalog, GetInterceptorCatalogHandler},
            get_transcript::{GetTranscript, GetTranscriptHandler},
            get_working_interceptor_catalog::{
                GetWorkingInterceptorCatalog, GetWorkingInterceptorCatalogHandler,
            },
            get_working_pipeline::{GetWorkingPipeline, GetWorkingPipelineHandler},
            modify_error::{ModifyError, ModifyErrorHandler},
            modify_params::{ModifyParams, ModifyParamsHandler},
            modify_result::{ModifyResult, ModifyResultHandler},
            query_execution_context::{QueryExecutionContext, QueryExecutionContextHandler},
            reject_call::{RejectCall, RejectCallHandler},
            request_review::{RequestReview, RequestReviewHandler},
        },
    },
    error::OrchestratorError,
    runtime::{CallExecutionFactory, OrchestratorResources, PhaseRuntime},
};
use actrpc_core::action::{ActionDescriptor, ActionKind, ActionSpec};
use std::{collections::HashMap, sync::Arc};

pub fn available_actions() -> HashMap<ActionKind, ActionDescriptor> {
    let mut actions = HashMap::new();

    insert_action::<ModifyParams>(&mut actions);
    insert_action::<ModifyResult>(&mut actions);
    insert_action::<ModifyError>(&mut actions);
    insert_action::<RejectCall>(&mut actions);
    insert_action::<ExcludeInterceptors>(&mut actions);
    insert_action::<GetTranscript>(&mut actions);
    insert_action::<GetInterceptorCatalog>(&mut actions);
    insert_action::<GetWorkingInterceptorCatalog>(&mut actions);
    insert_action::<GetWorkingPipeline>(&mut actions);
    insert_action::<CallMethod>(&mut actions);
    insert_action::<RequestReview>(&mut actions);
    insert_action::<QueryExecutionContext>(&mut actions);

    actions
}

pub fn build_builtin_action_registry(
    factory: Arc<CallExecutionFactory>,
    resources: &OrchestratorResources,
    phase: &PhaseRuntime,
) -> Result<ActionRegistry, OrchestratorError> {
    let mut registry = ActionRegistry::new();

    registry.register::<ModifyParams, _>(ModifyParamsHandler::new(
        phase.call.in_flight_message.clone(),
    ))?;

    registry.register::<ModifyResult, _>(ModifyResultHandler::new(
        phase.call.in_flight_message.clone(),
    ))?;

    registry.register::<ModifyError, _>(ModifyErrorHandler::new(
        phase.call.in_flight_message.clone(),
    ))?;

    registry.register::<RejectCall, _>(RejectCallHandler::new(phase.call.rejection.clone()))?;

    registry.register::<ExcludeInterceptors, _>(ExcludeInterceptorsHandler::new(
        phase.pipeline.clone(),
    ))?;

    registry
        .register::<GetTranscript, _>(GetTranscriptHandler::new(phase.call.transcript.clone()))?;

    registry.register::<GetInterceptorCatalog, _>(GetInterceptorCatalogHandler::new(
        resources.interceptor_catalog.clone(),
    ))?;

    registry.register::<GetWorkingInterceptorCatalog, _>(
        GetWorkingInterceptorCatalogHandler::new(
            resources.interceptor_catalog.clone(),
            phase.pipeline.clone(),
        ),
    )?;

    registry.register::<GetWorkingPipeline, _>(GetWorkingPipelineHandler::new(
        phase.pipeline.clone(),
    ))?;

    registry.register::<CallMethod, _>(CallMethodHandler::new(factory, phase.call.clone()))?;

    registry.register::<RequestReview, _>(RequestReviewHandler::new(
        resources.review_provider.clone(),
    ))?;

    registry.register::<QueryExecutionContext, _>(QueryExecutionContextHandler::new(
        phase.call.transcript.clone(),
    ))?;

    Ok(registry)
}

fn insert_action<A>(actions: &mut HashMap<ActionKind, ActionDescriptor>)
where
    A: ActionSpec,
{
    actions.insert(A::action_kind(), A::descriptor());
}
