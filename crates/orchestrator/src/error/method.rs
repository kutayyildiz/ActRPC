use crate::method::{MethodName, ProviderName};
use actrpc_core::json_rpc::JsonRpcError;
use actrpc_transport::TransportError;

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum MethodCatalogError {
    #[error("duplicate method provider: {provider}")]
    DuplicateProvider { provider: ProviderName },

    #[error("failed to build method provider {provider}: {source}")]
    ProviderBuild {
        provider: ProviderName,
        #[source]
        source: MethodProviderBuildError,
    },
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum MethodProviderBuildError {
    #[error("failed to create client for method provider {provider}: {source}")]
    ClientCreate {
        provider: ProviderName,
        #[source]
        source: TransportError,
    },

    #[error("method provider discovery transport failed for {provider}: {source}")]
    DiscoveryTransport {
        provider: ProviderName,
        #[source]
        source: TransportError,
    },

    #[error("method provider discovery failed for {provider}: {message}")]
    DiscoveryFailed {
        provider: ProviderName,
        message: String,
    },

    #[error("duplicate method {method} in provider {provider}")]
    DuplicateMethod {
        provider: ProviderName,
        method: MethodName,
    },

    #[error("invalid method provider config for {provider}: {message}")]
    InvalidConfig {
        provider: ProviderName,
        message: String,
    },
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum MethodCallError {
    #[error("method provider not found: {provider}")]
    ProviderNotFound { provider: ProviderName },

    #[error("method {method} not found in provider {provider}")]
    MethodNotFound {
        provider: ProviderName,
        method: MethodName,
    },

    #[error("method call transport failed for {provider}/{method}: {source}")]
    Transport {
        provider: ProviderName,
        method: MethodName,
        #[source]
        source: TransportError,
    },

    #[error("method {provider}/{method} returned remote JSON-RPC error: {error:?}")]
    RemoteError {
        provider: ProviderName,
        method: MethodName,
        error: JsonRpcError,
    },

    #[error("method {provider}/{method} returned invalid response: {message}")]
    InvalidResponse {
        provider: ProviderName,
        method: MethodName,
        message: String,
    },

    #[error("invalid params for method {provider}/{method}: {message}")]
    InvalidParams {
        provider: ProviderName,
        method: MethodName,
        message: String,
    },
}
