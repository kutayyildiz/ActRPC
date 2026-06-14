use actrpc_core::action::ActionSpec;
use actrpc_orchestrator::action::{
    actions::{
        call_method::CallMethod, exclude_interceptors::ExcludeInterceptors,
        get_interceptor_catalog::GetInterceptorCatalog, get_transcript::GetTranscript,
        get_working_interceptor_catalog::GetWorkingInterceptorCatalog,
        get_working_pipeline::GetWorkingPipeline, modify_error::ModifyError,
        modify_params::ModifyParams, modify_result::ModifyResult,
        query_execution_context::QueryExecutionContext, reject_call::RejectCall,
        request_review::RequestReview,
    },
    available_actions,
};

#[test]
fn available_actions_contains_all_builtin_actions() {
    let actions = available_actions();

    assert!(actions.contains_key(&ExcludeInterceptors::action_kind()));
    assert!(actions.contains_key(&GetTranscript::action_kind()));
    assert!(actions.contains_key(&GetWorkingPipeline::action_kind()));
    assert!(actions.contains_key(&ModifyError::action_kind()));
    assert!(actions.contains_key(&ModifyParams::action_kind()));
    assert!(actions.contains_key(&ModifyResult::action_kind()));
    assert!(actions.contains_key(&RejectCall::action_kind()));
    assert!(actions.contains_key(&RequestReview::action_kind()));
    assert!(actions.contains_key(&GetInterceptorCatalog::action_kind()));
    assert!(actions.contains_key(&GetWorkingInterceptorCatalog::action_kind()));
    assert!(actions.contains_key(&CallMethod::action_kind()));
    assert!(actions.contains_key(&QueryExecutionContext::action_kind()));

    assert_eq!(actions.len(), 12);
}
