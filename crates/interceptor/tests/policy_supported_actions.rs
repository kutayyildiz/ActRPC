use actrpc_core::action::ActionSpec;
use actrpc_interceptor::interceptors::policy::{PolicyInterceptor, config::PolicyConfig};
use actrpc_orchestrator::{
    action::actions::{
        exclude_interceptors::ExcludeInterceptors, reject_call::RejectCall,
        request_review::RequestReview,
    },
    interceptor::Interceptor,
};

#[tokio::test]
async fn initialize_advertises_only_supported_policy_actions() {
    let interceptor = PolicyInterceptor::new(PolicyConfig::default()).unwrap();

    let init = interceptor.initialize().await.unwrap();

    assert!(init.actions.contains_key(&RejectCall::action_kind()));
    assert!(
        init.actions
            .contains_key(&ExcludeInterceptors::action_kind())
    );
    assert!(init.actions.contains_key(&RequestReview::action_kind()));
    assert_eq!(init.actions.len(), 3);
}
