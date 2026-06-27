use super::{
    config::{EndpointConfig, JsonRpc2Mode, ProtocolConfig},
    endpoint_requirements::{
        EndpointConsumer, EndpointConsumerRequirement, EndpointRequirement, JsonRpc2Requirement,
        endpoint_consumer_requirements, has_json_rpc2_requirement, has_rest_http_requirement,
        merged_json_rpc2_requirement,
    },
    json_rpc2::JsonRpc2RequestHandle,
    rest_http::RestHttpEndpointImpl,
};
use crate::error::{ConfigError, OrchestratorError};
use actrpc_transport::{
    HttpRestClient, JsonRpcClient, JsonRpcClientProvider, JsonRpcSession, JsonRpcSessionProvider,
    TransportError, TransportTarget,
};
use std::sync::Arc;

pub enum BuiltEndpoint {
    JsonRpc2Request {
        request: Arc<dyn super::json_rpc2::JsonRpc2RequestEndpoint>,
    },
    JsonRpc2Session {
        request: Arc<dyn super::json_rpc2::JsonRpc2RequestEndpoint>,
        session: Arc<dyn super::json_rpc2::JsonRpc2SessionEndpoint>,
    },
    RestHttp {
        endpoint: Arc<dyn super::rest_http::RestHttpEndpoint>,
    },
}

pub async fn build_endpoint<CP, SP>(
    config: &EndpointConfig,
    consumer_requirements: &[EndpointConsumerRequirement],
    client_provider: &CP,
    session_provider: &SP,
) -> Result<BuiltEndpoint, OrchestratorError>
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
    validate_consumer_protocol_alignment(config, consumer_requirements)?;

    match &config.protocol {
        ProtocolConfig::RestHttp(_) => build_rest_http(config).await,
        ProtocolConfig::JsonRpc2(protocol) => {
            let json_rpc2_requirement =
                resolve_json_rpc2_requirement(protocol.mode, consumer_requirements)?;
            validate_json_rpc2_mode(
                config,
                protocol.mode,
                json_rpc2_requirement,
                consumer_requirements,
            )?;
            build_json_rpc2(
                config,
                json_rpc2_requirement,
                client_provider,
                session_provider,
            )
            .await
        }
    }
}

fn validate_consumer_protocol_alignment(
    config: &EndpointConfig,
    consumer_requirements: &[EndpointConsumerRequirement],
) -> Result<(), OrchestratorError> {
    let wants_rest = has_rest_http_requirement(consumer_requirements);
    let wants_json_rpc = has_json_rpc2_requirement(consumer_requirements);

    if wants_rest && wants_json_rpc {
        let details = consumer_requirements
            .iter()
            .map(format_consumer_requirement)
            .collect::<Vec<_>>()
            .join("; ");
        return Err(OrchestratorError::Config(
            ConfigError::EndpointRequirementConflict {
                endpoint: config.name.clone(),
                message: format!(
                    "endpoint '{}' is referenced by consumers with incompatible protocol requirements: {}",
                    config.name, details
                ),
            },
        ));
    }

    match &config.protocol {
        ProtocolConfig::RestHttp(_) if wants_json_rpc => {
            let consumer = consumer_requirements
                .iter()
                .find(|e| matches!(e.requirement, EndpointRequirement::JsonRpc2(_)))
                .map(format_consumer_requirement)
                .unwrap_or_else(|| "json_rpc2 consumer".to_owned());
            return Err(OrchestratorError::Config(
                ConfigError::EndpointRequirementConflict {
                    endpoint: config.name.clone(),
                    message: format!(
                        "endpoint '{}' configured as rest_http, but {consumer} requires json_rpc2 request_response",
                        config.name
                    ),
                },
            ));
        }
        ProtocolConfig::JsonRpc2(_) if wants_rest => {
            let consumer = consumer_requirements
                .iter()
                .find(|e| e.requirement == EndpointRequirement::RestHttp)
                .map(format_consumer_requirement)
                .unwrap_or_else(|| "rest_http consumer".to_owned());
            return Err(OrchestratorError::Config(
                ConfigError::EndpointRequirementConflict {
                    endpoint: config.name.clone(),
                    message: format!(
                        "endpoint '{}' configured as json_rpc2, but {consumer} requires rest_http",
                        config.name
                    ),
                },
            ));
        }
        _ => {}
    }

    Ok(())
}

fn resolve_json_rpc2_requirement(
    mode: JsonRpc2Mode,
    consumer_requirements: &[EndpointConsumerRequirement],
) -> Result<JsonRpc2Requirement, OrchestratorError> {
    match mode {
        JsonRpc2Mode::Auto => Ok(merged_json_rpc2_requirement(consumer_requirements)
            .unwrap_or(JsonRpc2Requirement::RequestResponse)),
        JsonRpc2Mode::RequestResponse => Ok(JsonRpc2Requirement::RequestResponse),
        JsonRpc2Mode::Session => Ok(JsonRpc2Requirement::Session),
    }
}

fn validate_json_rpc2_mode(
    config: &EndpointConfig,
    mode: JsonRpc2Mode,
    resolved: JsonRpc2Requirement,
    consumer_requirements: &[EndpointConsumerRequirement],
) -> Result<(), OrchestratorError> {
    let required_by_consumers = merged_json_rpc2_requirement(consumer_requirements);
    if mode == JsonRpc2Mode::RequestResponse
        && required_by_consumers == Some(JsonRpc2Requirement::Session)
    {
        let consumer = consumer_requirements
            .iter()
            .find(|entry| {
                matches!(
                    entry.requirement,
                    EndpointRequirement::JsonRpc2(JsonRpc2Requirement::Session)
                )
            })
            .map(format_consumer_requirement)
            .unwrap_or_else(|| "watchable method provider".to_owned());
        return Err(OrchestratorError::Config(
            ConfigError::EndpointRequirementConflict {
                endpoint: config.name.clone(),
                message: format!(
                    "endpoint '{}' configured as json_rpc2 request_response, but {consumer} requires json_rpc2 session",
                    config.name
                ),
            },
        ));
    }

    if mode == JsonRpc2Mode::RequestResponse && resolved == JsonRpc2Requirement::Session {
        let consumer = consumer_requirements
            .iter()
            .find(|entry| {
                matches!(
                    entry.requirement,
                    EndpointRequirement::JsonRpc2(JsonRpc2Requirement::Session)
                )
            })
            .map(format_consumer_requirement)
            .unwrap_or_else(|| "watchable method provider".to_owned());
        return Err(OrchestratorError::Config(
            ConfigError::EndpointRequirementConflict {
                endpoint: config.name.clone(),
                message: format!(
                    "endpoint '{}' configured as json_rpc2 request_response, but {consumer} requires json_rpc2 session",
                    config.name
                ),
            },
        ));
    }
    Ok(())
}

async fn build_json_rpc2<CP, SP>(
    config: &EndpointConfig,
    requirement: JsonRpc2Requirement,
    client_provider: &CP,
    session_provider: &SP,
) -> Result<BuiltEndpoint, OrchestratorError>
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
    match requirement {
        JsonRpc2Requirement::RequestResponse => {
            let client = client_provider
                .get_client(&config.transport)
                .await
                .map_err(OrchestratorError::Transport)?;
            let request = Arc::new(JsonRpc2RequestHandle::from_client(
                config.name.clone(),
                client,
            ));
            Ok(BuiltEndpoint::JsonRpc2Request { request })
        }
        JsonRpc2Requirement::Session => {
            if matches!(config.transport, TransportTarget::Http(_)) {
                return Err(OrchestratorError::WatchableUnsupportedEndpoint {
                    endpoint: config.name.clone(),
                    message: "HTTP does not support persistent JSON-RPC sessions".to_owned(),
                });
            }
            let session = session_provider
                .get_session(&config.transport)
                .await
                .map_err(|source| OrchestratorError::WatchableUnsupportedEndpoint {
                    endpoint: config.name.clone(),
                    message: source.to_string(),
                })?;
            let (request, session) =
                JsonRpc2RequestHandle::from_session(config.name.clone(), session);
            Ok(BuiltEndpoint::JsonRpc2Session {
                request: Arc::new(request),
                session: Arc::new(session),
            })
        }
    }
}

async fn build_rest_http(config: &EndpointConfig) -> Result<BuiltEndpoint, OrchestratorError> {
    let TransportTarget::Http(target) = &config.transport else {
        return Err(OrchestratorError::Config(
            ConfigError::EndpointRequirementConflict {
                endpoint: config.name.clone(),
                message: format!(
                    "endpoint '{}' configured as rest_http but transport {:?} is not HTTP",
                    config.name, config.transport
                ),
            },
        ));
    };

    let client = HttpRestClient::new(target.clone()).map_err(OrchestratorError::Transport)?;
    let endpoint = Arc::new(RestHttpEndpointImpl::new(
        config.name.clone(),
        config.transport.clone(),
        client,
    ));
    Ok(BuiltEndpoint::RestHttp { endpoint })
}

pub fn consumer_requirements_for_endpoint(
    all: &std::collections::HashMap<super::config::EndpointName, Vec<EndpointConsumerRequirement>>,
    endpoint: &super::config::EndpointName,
) -> Vec<EndpointConsumerRequirement> {
    all.get(endpoint).cloned().unwrap_or_default()
}

pub fn all_consumer_requirements(
    methods: &[crate::method::MethodSourceConfig],
    interceptors: &[crate::interceptor::InterceptorConfig],
) -> std::collections::HashMap<super::config::EndpointName, Vec<EndpointConsumerRequirement>> {
    endpoint_consumer_requirements(methods, interceptors)
}

fn format_consumer_requirement(entry: &EndpointConsumerRequirement) -> String {
    match &entry.consumer {
        EndpointConsumer::JsonRpcMethodProvider {
            provider,
            watchable,
        } => {
            if *watchable {
                format!(
                    "method provider '{provider}' requires json_rpc2 session for watchable discovery"
                )
            } else {
                format!("method provider '{provider}' requires json_rpc2 request_response")
            }
        }
        EndpointConsumer::McpMethodProvider { provider } => {
            format!("MCP method provider '{provider}' requires json_rpc2 request_response")
        }
        EndpointConsumer::RestMethodProvider { provider } => {
            format!("REST method provider '{provider}' requires rest_http")
        }
        EndpointConsumer::Interceptor { name } => {
            format!("interceptor '{name}' requires json_rpc2 request_response")
        }
    }
}
