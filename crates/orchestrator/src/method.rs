mod catalog;
mod config;
mod provider;
mod providers;
mod types;

pub use catalog::MethodCatalog;
pub use config::MethodSourceConfig;
pub use provider::{MethodProvider, MethodProviderFuture};
pub use types::{MethodInfo, MethodName, ProviderName};

pub use providers::{
    mcp::{McpMethodProvider, McpMethodSourceConfig},
    native::{NativeMethodConfig, NativeMethodProvider, NativeMethodSourceConfig},
};
