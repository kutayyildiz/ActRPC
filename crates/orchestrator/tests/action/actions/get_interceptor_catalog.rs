use actrpc_core::{
    InterceptorInitialization,
    action::ActionSpec,
    interception::{InterceptionRequest, InterceptionResponse, InterceptorContinuation},
};
use actrpc_orchestrator::{
    action::{
        ActionRegistry,
        actions::get_interceptor_catalog::{GetInterceptorCatalog, GetInterceptorCatalogHandler},
    },
    error::InterceptorRuntimeError,
    interceptor::{
        ImmutableInterceptorPipeline, Interceptor, InterceptorCatalog, InterceptorCatalogEntry,
        InterceptorFuture, InterceptorPolicy,
    },
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use super::super::helpers::{dummy_request, no_params_action_record};

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
async fn get_interceptor_catalog_returns_all_catalog_entries() {
    let catalog = Arc::new(test_catalog());

    let mut registry = ActionRegistry::new();
    registry
        .register::<GetInterceptorCatalog, _>(GetInterceptorCatalogHandler::new(catalog))
        .unwrap();

    let resolved = registry
        .get(&GetInterceptorCatalog::action_kind())
        .unwrap()
        .handle(
            &dummy_request(),
            no_params_action_record::<GetInterceptorCatalog>(),
        )
        .await
        .unwrap();

    let mut names: Vec<String> = resolved
        .result
        .unwrap()
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["name"].as_str().unwrap().to_owned())
        .collect();

    names.sort();

    assert_eq!(names, vec!["firewall".to_owned(), "logger".to_owned()]);
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
