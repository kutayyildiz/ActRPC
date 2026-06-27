use actrpc_core::action::ActionSpec;
use actrpc_interceptor::interceptors::dynamic_policy::new_component;
use actrpc_orchestrator::{
    action::actions::{
        query_execution_context::QueryExecutionContext, reject_call::RejectCall,
        request_review::RequestReview,
    },
    interceptor::Interceptor,
};

#[tokio::test]
async fn initialize_advertises_supported_dynamic_policy_actions() {
    let component = new_component();
    let init = component.interceptor.initialize().await.unwrap();

    assert!(init.supports_outbound);
    assert!(init.supports_inbound);
    assert!(init.actions.contains_key(&RejectCall::action_kind()));
    assert!(
        init.actions
            .contains_key(&QueryExecutionContext::action_kind())
    );
    assert!(init.actions.contains_key(&RequestReview::action_kind()));
    assert_eq!(init.actions.len(), 3);
}
