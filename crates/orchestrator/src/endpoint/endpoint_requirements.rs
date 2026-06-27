use super::config::EndpointName;
use crate::{
    interceptor::InterceptorConfig,
    method::{JsonRpcMethodDiscoveryConfig, MethodSourceConfig},
};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonRpc2Requirement {
    RequestResponse,
    Session,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointRequirement {
    JsonRpc2(JsonRpc2Requirement),
    RestHttp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndpointConsumer {
    JsonRpcMethodProvider { provider: String, watchable: bool },
    McpMethodProvider { provider: String },
    RestMethodProvider { provider: String },
    Interceptor { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointConsumerRequirement {
    pub requirement: EndpointRequirement,
    pub consumer: EndpointConsumer,
}

pub fn endpoint_consumer_requirements(
    methods: &[MethodSourceConfig],
    interceptors: &[InterceptorConfig],
) -> HashMap<EndpointName, Vec<EndpointConsumerRequirement>> {
    let mut requirements = HashMap::new();

    for method in methods {
        match method {
            MethodSourceConfig::JsonRpc(config) => {
                let watchable = matches!(
                    config.discovery,
                    JsonRpcMethodDiscoveryConfig::Watchable { .. }
                );
                let requirement = if watchable {
                    EndpointRequirement::JsonRpc2(JsonRpc2Requirement::Session)
                } else {
                    EndpointRequirement::JsonRpc2(JsonRpc2Requirement::RequestResponse)
                };
                push_requirement(
                    &mut requirements,
                    config.endpoint.clone(),
                    EndpointConsumerRequirement {
                        requirement,
                        consumer: EndpointConsumer::JsonRpcMethodProvider {
                            provider: config.provider.as_str().to_owned(),
                            watchable,
                        },
                    },
                );
            }
            MethodSourceConfig::Mcp(config) => {
                push_requirement(
                    &mut requirements,
                    config.endpoint.clone(),
                    EndpointConsumerRequirement {
                        requirement: EndpointRequirement::JsonRpc2(
                            JsonRpc2Requirement::RequestResponse,
                        ),
                        consumer: EndpointConsumer::McpMethodProvider {
                            provider: config.name.as_str().to_owned(),
                        },
                    },
                );
            }
            MethodSourceConfig::Rest(config) => {
                push_requirement(
                    &mut requirements,
                    config.endpoint.clone(),
                    EndpointConsumerRequirement {
                        requirement: EndpointRequirement::RestHttp,
                        consumer: EndpointConsumer::RestMethodProvider {
                            provider: config.provider.as_str().to_owned(),
                        },
                    },
                );
            }
        }
    }

    for interceptor in interceptors {
        push_requirement(
            &mut requirements,
            interceptor.endpoint.clone(),
            EndpointConsumerRequirement {
                requirement: EndpointRequirement::JsonRpc2(JsonRpc2Requirement::RequestResponse),
                consumer: EndpointConsumer::Interceptor {
                    name: interceptor.name.clone(),
                },
            },
        );
    }

    requirements
}

pub fn merged_json_rpc2_requirement(
    entries: &[EndpointConsumerRequirement],
) -> Option<JsonRpc2Requirement> {
    let mut requirement = None;
    for entry in entries {
        let EndpointRequirement::JsonRpc2(req) = entry.requirement else {
            continue;
        };
        requirement = Some(match requirement {
            Some(JsonRpc2Requirement::Session) => JsonRpc2Requirement::Session,
            Some(JsonRpc2Requirement::RequestResponse) => req,
            None => req,
        });
    }
    requirement
}

pub fn has_rest_http_requirement(entries: &[EndpointConsumerRequirement]) -> bool {
    entries
        .iter()
        .any(|e| e.requirement == EndpointRequirement::RestHttp)
}

pub fn has_json_rpc2_requirement(entries: &[EndpointConsumerRequirement]) -> bool {
    entries
        .iter()
        .any(|e| matches!(e.requirement, EndpointRequirement::JsonRpc2(_)))
}

fn push_requirement(
    requirements: &mut HashMap<EndpointName, Vec<EndpointConsumerRequirement>>,
    endpoint: EndpointName,
    entry: EndpointConsumerRequirement,
) {
    requirements.entry(endpoint).or_default().push(entry);
}
