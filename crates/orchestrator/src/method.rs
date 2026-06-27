mod catalog;
mod config;
mod method_provider_changed;
mod provider;
mod providers;
mod rpc_bridge;
mod target;
mod types;
mod watchable_listener;

pub use catalog::MethodCatalog;
pub use config::MethodSourceConfig;
pub use provider::{MethodProvider, MethodProviderFuture};
pub use providers::json_rpc::{
    JsonRpcMethodDiscoveryConfig, JsonRpcMethodProvider, JsonRpcMethodSourceConfig,
};
pub use target::method_target_from_names;
pub use types::{MethodInfo, MethodName, MethodProviderSnapshot, ProviderName};
pub use watchable_listener::spawn_watchable_listeners;

pub use providers::mcp::{McpMethodProvider, McpMethodSourceConfig};
pub use providers::rest::{
    RestMethodDefinition, RestMethodProvider, RestMethodSourceConfig, RestRequestMapping,
    RestResponseMapping,
};
