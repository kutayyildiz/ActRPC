use actrpc_core::action::ActionSpec;
use actrpc_orchestrator::{
    action::{
        ActionRegistry,
        actions::get_working_pipeline::{GetWorkingPipeline, GetWorkingPipelineHandler},
    },
    interceptor::WorkingInterceptorPipeline,
};
use std::sync::Arc;

use super::super::helpers::{dummy_request, no_params_action_record};

#[tokio::test]
async fn get_working_pipeline_returns_current_pipeline_snapshot() {
    let pipeline = Arc::new(WorkingInterceptorPipeline::new(vec![
        "firewall".to_owned(),
        "logger".to_owned(),
    ]));

    let mut registry = ActionRegistry::new();
    registry
        .register::<GetWorkingPipeline, _>(GetWorkingPipelineHandler::new(pipeline))
        .unwrap();

    let resolved = registry
        .get(&GetWorkingPipeline::action_kind())
        .unwrap()
        .handle(
            &dummy_request(),
            no_params_action_record::<GetWorkingPipeline>(),
        )
        .await
        .unwrap();

    assert_eq!(
        resolved.result,
        Ok(Some(serde_json::json!([
            { "name": "firewall" },
            { "name": "logger" }
        ])))
    );
}
