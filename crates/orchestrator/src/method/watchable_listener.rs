use crate::{
    endpoint::EndpointCatalog,
    error::MethodCatalogError,
    method::{catalog::MethodCatalog, method_provider_changed::try_parse_method_provider_changed},
};
use actrpc_transport::JsonRpcSessionEvent;
use std::{collections::HashSet, sync::Arc};

pub fn spawn_watchable_listeners(
    catalog: Arc<MethodCatalog>,
    endpoints: &EndpointCatalog,
) -> Result<Vec<tokio::task::JoinHandle<()>>, MethodCatalogError> {
    let mut endpoint_names = HashSet::new();
    for provider in catalog.providers() {
        if !provider.is_watchable() {
            continue;
        }
        if let Some(endpoint) = provider.endpoint() {
            endpoint_names.insert(endpoint.clone());
        }
    }

    let mut handles = Vec::new();
    for endpoint_name in endpoint_names {
        let Some(endpoint) = endpoints.get(&endpoint_name) else {
            continue;
        };
        let provider_for_error = catalog
            .providers()
            .find(|p| p.is_watchable() && p.endpoint() == Some(&endpoint_name))
            .map(|p| p.name().clone());
        let mut receiver = endpoint.subscribe().map_err(|_| {
            MethodCatalogError::EndpointDoesNotSupportSession {
                endpoint: endpoint_name.clone(),
                provider: provider_for_error.unwrap_or_else(|| {
                    catalog
                        .providers()
                        .next()
                        .expect("catalog has watchable provider for endpoint")
                        .name()
                        .clone()
                }),
            }
        })?;
        let catalog = catalog.clone();
        let endpoint_name = endpoint_name.clone();
        handles.push(tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(JsonRpcSessionEvent::Notification(notification)) => {
                        let Some(params) = try_parse_method_provider_changed(&notification) else {
                            continue;
                        };
                        let _ = catalog
                            .handle_method_provider_changed(
                                &endpoint_name,
                                &params.provider,
                                params.version,
                            )
                            .await;
                    }
                    Ok(JsonRpcSessionEvent::Closed) => break,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                }
            }
        }));
    }

    Ok(handles)
}
