use super::{
    builder::{
        BuiltEndpoint, all_consumer_requirements, build_endpoint,
        consumer_requirements_for_endpoint,
    },
    catalog_error::EndpointCatalogError,
    config::{EndpointConfig, EndpointName},
    json_rpc2::{JsonRpc2RequestEndpoint, JsonRpc2SessionEndpoint},
    kind::EndpointKind,
    rest_http::RestHttpEndpoint,
};
use crate::{error::OrchestratorError, interceptor::InterceptorConfig, method::MethodSourceConfig};
use actrpc_transport::{
    JsonRpcClient, JsonRpcClientProvider, JsonRpcSession, JsonRpcSessionProvider, TransportError,
};
use std::{collections::HashMap, sync::Arc};

/// Test-only catalog entry wrapper. Public for integration tests; not a stable API.
#[doc(hidden)]
pub enum EndpointEntry {
    JsonRpc2Request {
        request: Arc<dyn JsonRpc2RequestEndpoint>,
    },
    JsonRpc2Session {
        request: Arc<dyn JsonRpc2RequestEndpoint>,
        session: Arc<dyn JsonRpc2SessionEndpoint>,
    },
    RestHttp {
        endpoint: Arc<dyn RestHttpEndpoint>,
    },
}

pub struct EndpointCatalog {
    entries: HashMap<EndpointName, EndpointEntry>,
}

/// Builds a catalog from pre-built endpoint entries. Public for integration tests only.
#[doc(hidden)]
pub fn test_catalog(entries: HashMap<EndpointName, EndpointEntry>) -> EndpointCatalog {
    EndpointCatalog { entries }
}

impl EndpointCatalog {
    pub async fn from_configs<CP, SP>(
        configs: Vec<EndpointConfig>,
        methods: &[MethodSourceConfig],
        interceptors: &[InterceptorConfig],
        client_provider: &CP,
        session_provider: &SP,
    ) -> Result<Self, OrchestratorError>
    where
        CP: JsonRpcClientProvider<
                Client = Arc<dyn JsonRpcClient<Error = TransportError>>,
                Error = TransportError,
            > + Send
            + Sync,
        SP: JsonRpcSessionProvider<
                Session = Arc<dyn JsonRpcSession<Error = TransportError>>,
                Error = TransportError,
            > + Send
            + Sync,
    {
        let all_requirements = all_consumer_requirements(methods, interceptors);
        let mut entries = HashMap::new();

        for config in configs {
            if entries.contains_key(&config.name) {
                return Err(OrchestratorError::Config(
                    crate::error::ConfigError::DuplicateEndpoint { name: config.name },
                ));
            }

            let consumer_requirements =
                consumer_requirements_for_endpoint(&all_requirements, &config.name);

            let built = build_endpoint(
                &config,
                &consumer_requirements,
                client_provider,
                session_provider,
            )
            .await?;

            let entry = match built {
                BuiltEndpoint::JsonRpc2Request { request } => {
                    EndpointEntry::JsonRpc2Request { request }
                }
                BuiltEndpoint::JsonRpc2Session { request, session } => {
                    EndpointEntry::JsonRpc2Session { request, session }
                }
                BuiltEndpoint::RestHttp { endpoint } => EndpointEntry::RestHttp { endpoint },
            };

            entries.insert(config.name, entry);
        }

        Ok(Self { entries })
    }

    pub fn get_json_rpc2(
        &self,
        name: &EndpointName,
    ) -> Result<Arc<dyn JsonRpc2RequestEndpoint>, EndpointCatalogError> {
        match self.entries.get(name) {
            Some(EndpointEntry::JsonRpc2Request { request }) => Ok(request.clone()),
            Some(EndpointEntry::JsonRpc2Session { request, .. }) => Ok(request.clone()),
            Some(EndpointEntry::RestHttp { .. }) => Err(EndpointCatalogError::KindMismatch {
                name: name.clone(),
                expected: EndpointKind::JsonRpc2,
                actual: EndpointKind::RestHttp,
            }),
            None => Err(EndpointCatalogError::NotFound { name: name.clone() }),
        }
    }

    pub fn get_json_rpc2_session(
        &self,
        name: &EndpointName,
    ) -> Result<Arc<dyn JsonRpc2SessionEndpoint>, EndpointCatalogError> {
        match self.entries.get(name) {
            Some(EndpointEntry::JsonRpc2Session { session, .. }) => Ok(session.clone()),
            Some(EndpointEntry::JsonRpc2Request { .. }) => {
                Err(EndpointCatalogError::SessionRequired { name: name.clone() })
            }
            Some(EndpointEntry::RestHttp { .. }) => Err(EndpointCatalogError::KindMismatch {
                name: name.clone(),
                expected: EndpointKind::JsonRpc2,
                actual: EndpointKind::RestHttp,
            }),
            None => Err(EndpointCatalogError::NotFound { name: name.clone() }),
        }
    }

    pub fn get_rest_http(
        &self,
        name: &EndpointName,
    ) -> Result<Arc<dyn RestHttpEndpoint>, EndpointCatalogError> {
        match self.entries.get(name) {
            Some(EndpointEntry::RestHttp { endpoint }) => Ok(endpoint.clone()),
            Some(EndpointEntry::JsonRpc2Request { .. } | EndpointEntry::JsonRpc2Session { .. }) => {
                Err(EndpointCatalogError::KindMismatch {
                    name: name.clone(),
                    expected: EndpointKind::RestHttp,
                    actual: EndpointKind::JsonRpc2,
                })
            }
            None => Err(EndpointCatalogError::NotFound { name: name.clone() }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endpoint::json_rpc2::JsonRpc2RequestHandle;
    use actrpc_core::json_rpc::{
        JsonRpcId, JsonRpcRequest, JsonRpcResponse, JsonRpcSuccessResponse, JsonRpcVersion,
    };
    use actrpc_transport::{
        JsonRpcClient, JsonRpcClientFuture, JsonRpcSession, JsonRpcSessionEvent,
        JsonRpcSessionFuture, RestHttpExecuteRequest, RestHttpExecuteResponse,
    };
    use std::{future::Future, pin::Pin};

    struct MockJsonRpcClient;

    impl JsonRpcClient for MockJsonRpcClient {
        type Error = TransportError;

        fn send<'a>(
            &'a self,
            _message: actrpc_core::json_rpc::JsonRpcMessage,
        ) -> JsonRpcClientFuture<'a, Result<actrpc_core::json_rpc::JsonRpcMessage, Self::Error>>
        {
            Box::pin(async move {
                Err(TransportError::Internal {
                    message: "not used".to_owned(),
                })
            })
        }
    }

    struct MockSession;

    impl JsonRpcSession for MockSession {
        type Error = TransportError;

        fn request<'a>(
            &'a self,
            _request: JsonRpcRequest,
        ) -> JsonRpcSessionFuture<'a, Result<JsonRpcResponse, Self::Error>> {
            Box::pin(async move {
                Ok(JsonRpcResponse::Success(JsonRpcSuccessResponse {
                    jsonrpc: JsonRpcVersion::V2_0,
                    id: JsonRpcId::Number(1.into()),
                    result: serde_json::Value::Null,
                }))
            })
        }

        fn notify<'a>(
            &'a self,
            _notification: actrpc_core::json_rpc::JsonRpcNotification,
        ) -> JsonRpcSessionFuture<'a, Result<(), Self::Error>> {
            Box::pin(async move { Ok(()) })
        }

        fn subscribe(&self) -> tokio::sync::broadcast::Receiver<JsonRpcSessionEvent> {
            tokio::sync::broadcast::channel(1).1
        }
    }

    struct MockRestEndpoint {
        name: EndpointName,
    }

    impl RestHttpEndpoint for MockRestEndpoint {
        fn endpoint_name(&self) -> &EndpointName {
            &self.name
        }

        fn endpoint_kind(&self) -> EndpointKind {
            EndpointKind::RestHttp
        }

        fn endpoint_capabilities(&self) -> super::super::kind::EndpointCapabilities {
            super::super::kind::EndpointCapabilities::REST_HTTP
        }

        fn execute<'a>(
            &'a self,
            _request: RestHttpExecuteRequest,
        ) -> Pin<
            Box<dyn Future<Output = Result<RestHttpExecuteResponse, TransportError>> + Send + 'a>,
        > {
            Box::pin(async move {
                Ok(RestHttpExecuteResponse {
                    status: 200,
                    body: b"{}".to_vec(),
                })
            })
        }
    }

    fn catalog() -> EndpointCatalog {
        let request = Arc::new(JsonRpc2RequestHandle::from_client(
            EndpointName::from("json_req"),
            Arc::new(MockJsonRpcClient) as Arc<dyn JsonRpcClient<Error = TransportError>>,
        ));

        let (session_request, session) = JsonRpc2RequestHandle::from_session(
            EndpointName::from("json_sess"),
            Arc::new(MockSession) as Arc<dyn JsonRpcSession<Error = TransportError>>,
        );

        let rest = Arc::new(MockRestEndpoint {
            name: EndpointName::from("rest"),
        });

        EndpointCatalog {
            entries: HashMap::from([
                (
                    EndpointName::from("json_req"),
                    EndpointEntry::JsonRpc2Request { request },
                ),
                (
                    EndpointName::from("json_sess"),
                    EndpointEntry::JsonRpc2Session {
                        request: Arc::new(session_request),
                        session: Arc::new(session),
                    },
                ),
                (
                    EndpointName::from("rest"),
                    EndpointEntry::RestHttp { endpoint: rest },
                ),
            ]),
        }
    }

    #[test]
    fn missing_endpoint_returns_not_found() {
        let catalog = catalog();
        assert!(matches!(
            catalog.get_json_rpc2(&EndpointName::from("missing")),
            Err(EndpointCatalogError::NotFound { .. })
        ));
    }

    #[test]
    fn rest_lookup_on_json_rpc_returns_kind_mismatch() {
        let catalog = catalog();
        assert!(matches!(
            catalog.get_rest_http(&EndpointName::from("json_req")),
            Err(EndpointCatalogError::KindMismatch {
                expected: EndpointKind::RestHttp,
                actual: EndpointKind::JsonRpc2,
                ..
            })
        ));
    }

    #[test]
    fn session_lookup_on_request_only_returns_session_required() {
        let catalog = catalog();
        assert!(matches!(
            catalog.get_json_rpc2_session(&EndpointName::from("json_req")),
            Err(EndpointCatalogError::SessionRequired { .. })
        ));
    }

    #[test]
    fn request_lookup_on_session_endpoint_succeeds() {
        let catalog = catalog();
        assert!(
            catalog
                .get_json_rpc2(&EndpointName::from("json_sess"))
                .is_ok()
        );
    }
}
