use actrpc_core::{
    InterceptorInitialization,
    action::ActionSpec,
    interception::{InterceptionRequest, InterceptionResponse, InterceptorContinuation},
};
use actrpc_orchestrator::{
    action::{
        ActionRegistry,
        actions::get_working_interceptor_catalog::{
            GetWorkingInterceptorCatalog, GetWorkingInterceptorCatalogHandler,
        },
    },
    error::InterceptorRuntimeError,
    interceptor::{
        ImmutableInterceptorPipeline, Interceptor, InterceptorCatalog, InterceptorCatalogEntry,
        InterceptorFuture, InterceptorPolicy, WorkingInterceptorPipeline,
    },
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use super::helpers::{invocation_context, dummy_request, no_params_action_record};

struct DummyInterceptor;

impl Interceptor for DummyInterceptor {
    fn initialize<'a>(
        &'a self,
    ) -> InterceptorFuture<'a, Result<InterceptorInitialization, InterceptorRuntimeError>>
    where
        Self: 'a,
    {
        Box::pin(async move {
            Ok(InterceptorInitialization {
                supports_outbound: true,
                supports_inbound: true,
                actions: HashMap::new(),
            })
        })
    }

    fn intercept<'a>(
        &'a self,
        _request: &'a InterceptionRequest,
    ) -> InterceptorFuture<'a, Result<InterceptionResponse, InterceptorRuntimeError>>
    where
        Self: 'a,
    {
        Box::pin(async move {
            Ok(InterceptionResponse {
                continuation: InterceptorContinuation::Stop,
                actions: vec![],
            })
        })
    }
}

#[tokio::test]
async fn get_working_interceptor_catalog_returns_only_pipeline_entries_in_pipeline_order() {
    let catalog = Arc::new(test_catalog());
    let pipeline = Arc::new(WorkingInterceptorPipeline::new(vec![
        "logger".to_owned(),
        "firewall".to_owned(),
    ]));

    let mut registry = ActionRegistry::new();
    registry
        .register::<GetWorkingInterceptorCatalog, _>(GetWorkingInterceptorCatalogHandler::new(
            catalog, pipeline,
        ))
        .unwrap();

    let resolved = registry
        .get(&GetWorkingInterceptorCatalog::action_kind())
        .unwrap()
        .handle(
            &dummy_request(),
            no_params_action_record::<GetWorkingInterceptorCatalog>(),
            &invocation_context("test"),
        )
        .await
        .unwrap();

    let names: Vec<String> = resolved
        .result
        .unwrap()
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["name"].as_str().unwrap().to_owned())
        .collect();

    assert_eq!(names, vec!["logger".to_owned(), "firewall".to_owned()]);
}

#[tokio::test]
async fn get_working_interceptor_catalog_fails_when_pipeline_references_unknown_interceptor() {
    let catalog = Arc::new(test_catalog());
    let pipeline = Arc::new(WorkingInterceptorPipeline::new(vec![
        "logger".to_owned(),
        "missing".to_owned(),
    ]));

    let mut registry = ActionRegistry::new();
    registry
        .register::<GetWorkingInterceptorCatalog, _>(GetWorkingInterceptorCatalogHandler::new(
            catalog, pipeline,
        ))
        .unwrap();

    let err = registry
        .get(&GetWorkingInterceptorCatalog::action_kind())
        .unwrap()
        .handle(
            &dummy_request(),
            no_params_action_record::<GetWorkingInterceptorCatalog>(),
            &invocation_context("test"),
        )
        .await
        .unwrap_err();

    match err {
        actrpc_orchestrator::error::ActionHandlerError::Execution(
            actrpc_orchestrator::error::ActionExecutionError::NotFound { target },
        ) => {
            assert_eq!(target, "missing");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

fn test_catalog() -> InterceptorCatalog {
    let mut entries = HashMap::new();

    entries.insert("firewall".to_owned(), entry("firewall"));
    entries.insert("logger".to_owned(), entry("logger"));

    InterceptorCatalog::new(
        entries,
        ImmutableInterceptorPipeline::new(vec!["firewall".to_owned(), "logger".to_owned()]),
        ImmutableInterceptorPipeline::new(vec!["logger".to_owned()]),
    )
}

fn entry(name: &str) -> InterceptorCatalogEntry {
    InterceptorCatalogEntry {
        name: name.to_owned(),
        policy: InterceptorPolicy {
            outbound: HashSet::new(),
            inbound: HashSet::new(),
        },
        interceptor: Arc::new(DummyInterceptor),
        runtime_limits: None,
    }
}
