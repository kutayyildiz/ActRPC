use crate::{
    endpoint::config::EndpointName,
    interceptor::InterceptorConfig,
    method::{JsonRpcMethodDiscoveryConfig, MethodSourceConfig},
};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointConnectionRequirement {
    Client,
    Session,
}

pub fn endpoint_connection_requirements(
    methods: &[MethodSourceConfig],
    interceptors: &[InterceptorConfig],
) -> HashMap<EndpointName, EndpointConnectionRequirement> {
    let mut requirements = HashMap::new();

    for method in methods {
        let (endpoint, requirement) = match method {
            MethodSourceConfig::JsonRpc(config) => {
                let requirement = match &config.discovery {
                    JsonRpcMethodDiscoveryConfig::Watchable { .. } => {
                        EndpointConnectionRequirement::Session
                    }
                    JsonRpcMethodDiscoveryConfig::Static { .. }
                    | JsonRpcMethodDiscoveryConfig::Initialize { .. }
                    | JsonRpcMethodDiscoveryConfig::Refreshable { .. } => {
                        EndpointConnectionRequirement::Client
                    }
                };
                (config.endpoint.clone(), requirement)
            }
            MethodSourceConfig::Mcp(config) => (
                config.endpoint.clone(),
                EndpointConnectionRequirement::Client,
            ),
        };
        merge_requirement(&mut requirements, endpoint, requirement);
    }

    for interceptor in interceptors {
        merge_requirement(
            &mut requirements,
            interceptor.endpoint.clone(),
            EndpointConnectionRequirement::Client,
        );
    }

    requirements
}

fn merge_requirement(
    requirements: &mut HashMap<EndpointName, EndpointConnectionRequirement>,
    endpoint: EndpointName,
    requirement: EndpointConnectionRequirement,
) {
    match requirements.get(&endpoint) {
        Some(EndpointConnectionRequirement::Session) => {}
        Some(EndpointConnectionRequirement::Client) => {
            if requirement == EndpointConnectionRequirement::Session {
                requirements.insert(endpoint, EndpointConnectionRequirement::Session);
            }
        }
        None => {
            requirements.insert(endpoint, requirement);
        }
    }
}

pub fn requirement_for_endpoint(
    requirements: &HashMap<EndpointName, EndpointConnectionRequirement>,
    endpoint: &EndpointName,
) -> EndpointConnectionRequirement {
    requirements
        .get(endpoint)
        .copied()
        .unwrap_or(EndpointConnectionRequirement::Client)
}
