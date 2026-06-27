mod builder;
pub(crate) mod catalog;
pub(crate) mod catalog_error;
pub(crate) mod config;
pub(crate) mod endpoint_requirements;
pub(crate) mod json_rpc2;
pub(crate) mod kind;
pub(crate) mod rest_http;

pub use builder::BuiltEndpoint;
pub use catalog::EndpointEntry;
pub use catalog::{EndpointCatalog, test_catalog};
pub use catalog_error::EndpointCatalogError;
pub use config::{EndpointConfig, EndpointName, JsonRpc2Mode, ProtocolConfig};
pub use endpoint_requirements::{
    EndpointConsumer, EndpointConsumerRequirement, EndpointRequirement, JsonRpc2Requirement,
};
pub use json_rpc2::{
    JsonRpc2RequestEndpoint, JsonRpc2RequestHandle, JsonRpc2SessionEndpoint, JsonRpc2SessionHandle,
};
pub use kind::{EndpointCapabilities, EndpointKind};
pub use rest_http::{RestHttpEndpoint, RestHttpEndpointImpl};
