use actrpc_core::action::ActionSpec;
use actrpc_orchestrator::{
    action::{
        ActionRegistry,
        actions::exclude_interceptors::{ExcludeInterceptors, ExcludeInterceptorsHandler},
    },
    error::{ActionExecutionError, ActionHandlerError},
    interceptor::WorkingInterceptorPipeline,
};
use serde_json::json;
use std::sync::Arc;

use super::super::helpers::{action_record, dummy_request};

#[tokio::test]
async fn exclude_interceptors_removes_matching_names_and_deduplicates_params() {
    let pipeline = Arc::new(WorkingInterceptorPipeline::new(vec![
        "a".to_owned(),
        "b".to_owned(),
        "c".to_owned(),
    ]));

    let mut registry = ActionRegistry::new();
    registry
        .register::<ExcludeInterceptors, _>(ExcludeInterceptorsHandler::new(pipeline.clone()))
        .unwrap();

    let action = action_record::<ExcludeInterceptors>(json!({
        "names": [" b ", "b", "c"]
    }));

    let resolved = registry
        .get(&ExcludeInterceptors::action_kind())
        .unwrap()
        .handle(&dummy_request(), action)
        .await
        .unwrap();

    assert_eq!(resolved.result, Ok(Some(json!(null))));
    assert_eq!(pipeline.snapshot(), vec!["a".to_owned()]);
}

#[tokio::test]
async fn exclude_interceptors_rejects_empty_names() {
    let pipeline = Arc::new(WorkingInterceptorPipeline::new(vec!["a".to_owned()]));

    let mut registry = ActionRegistry::new();
    registry
        .register::<ExcludeInterceptors, _>(ExcludeInterceptorsHandler::new(pipeline))
        .unwrap();

    let action = action_record::<ExcludeInterceptors>(json!({
        "names": []
    }));

    let err = registry
        .get(&ExcludeInterceptors::action_kind())
        .unwrap()
        .handle(&dummy_request(), action)
        .await
        .unwrap_err();

    match err {
        ActionHandlerError::Execution(ActionExecutionError::InvalidParams { action }) => {
            assert_eq!(action, ExcludeInterceptors::action_kind());
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn exclude_interceptors_rejects_blank_name() {
    let pipeline = Arc::new(WorkingInterceptorPipeline::new(vec!["a".to_owned()]));

    let mut registry = ActionRegistry::new();
    registry
        .register::<ExcludeInterceptors, _>(ExcludeInterceptorsHandler::new(pipeline))
        .unwrap();

    let action = action_record::<ExcludeInterceptors>(json!({
        "names": ["a", "   "]
    }));

    let err = registry
        .get(&ExcludeInterceptors::action_kind())
        .unwrap()
        .handle(&dummy_request(), action)
        .await
        .unwrap_err();

    match err {
        ActionHandlerError::Execution(ActionExecutionError::InvalidParams { action }) => {
            assert_eq!(action, ExcludeInterceptors::action_kind());
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
