use super::{
    config::EndpointConfig,
    connection_mode::{
        EndpointConnectionRequirement, endpoint_connection_requirements, requirement_for_endpoint,
    },
    types::{EndpointConnection, JsonRpcEndpoint},
};
use crate::{error::OrchestratorError, interceptor::InterceptorConfig, method::MethodSourceConfig};
use actrpc_transport::{
    JsonRpcClient, JsonRpcClientProvider, JsonRpcSession, JsonRpcSessionProvider, TransportError,
    TransportTarget,
};
use std::{collections::HashMap, sync::Arc};

pub struct EndpointCatalog {
    entries: HashMap<crate::endpoint::config::EndpointName, Arc<JsonRpcEndpoint>>,
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
        let requirements = endpoint_connection_requirements(methods, interceptors);
        let mut entries = HashMap::new();

        for config in configs {
            if entries.contains_key(&config.name) {
                return Err(OrchestratorError::Config(
                    crate::error::ConfigError::DuplicateEndpoint { name: config.name },
                ));
            }

            let requirement = requirement_for_endpoint(&requirements, &config.name);
            let conn =
                match requirement {
                    EndpointConnectionRequirement::Client => {
                        let client = client_provider
                            .get_client(&config.target)
                            .await
                            .map_err(OrchestratorError::Transport)?;
                        EndpointConnection::Client(client)
                    }
                    EndpointConnectionRequirement::Session => {
                        if matches!(config.target, TransportTarget::Http(_)) {
                            return Err(OrchestratorError::WatchableUnsupportedEndpoint {
                                endpoint: config.name.clone(),
                                message: "HTTP does not support persistent JSON-RPC sessions"
                                    .to_owned(),
                            });
                        }
                        let session = session_provider.get_session(&config.target).await.map_err(
                            |source| OrchestratorError::WatchableUnsupportedEndpoint {
                                endpoint: config.name.clone(),
                                message: source.to_string(),
                            },
                        )?;
                        EndpointConnection::Session(session)
                    }
                };

            let ep = Arc::new(JsonRpcEndpoint::new(
                config.name.clone(),
                config.target,
                conn,
            ));
            entries.insert(config.name, ep);
        }

        Ok(Self { entries })
    }

    pub fn get(
        &self,
        name: &crate::endpoint::config::EndpointName,
    ) -> Option<Arc<JsonRpcEndpoint>> {
        self.entries.get(name).cloned()
    }
}
