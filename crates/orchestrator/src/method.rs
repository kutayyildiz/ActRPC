mod catalog;
mod config;
mod method_provider_changed;
mod provider;
mod providers;
mod types;
mod watchable_listener;

pub use catalog::MethodCatalog;
pub use config::MethodSourceConfig;
pub use provider::{MethodProvider, MethodProviderFuture};
pub use providers::json_rpc::{JsonRpcMethodDiscoveryConfig, JsonRpcMethodSourceConfig};
pub use types::{MethodInfo, MethodName, MethodProviderSnapshot, ProviderName};
pub use watchable_listener::spawn_watchable_listeners;

pub use providers::mcp::{McpMethodProvider, McpMethodSourceConfig};
