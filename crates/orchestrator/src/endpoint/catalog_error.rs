use super::{config::EndpointName, kind::EndpointKind};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EndpointCatalogError {
    #[error("endpoint not found: {name}")]
    NotFound { name: EndpointName },

    #[error("endpoint '{name}' protocol mismatch: expected {expected:?}, found {actual:?}")]
    KindMismatch {
        name: EndpointName,
        expected: EndpointKind,
        actual: EndpointKind,
    },

    #[error("endpoint '{name}' requires a session-capable JSON-RPC2 endpoint")]
    SessionRequired { name: EndpointName },
}
