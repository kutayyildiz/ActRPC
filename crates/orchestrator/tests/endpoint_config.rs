use actrpc_orchestrator::{
    EndpointConfig, EndpointName, JsonRpc2Mode, ProtocolConfig,
    error::OrchestratorError,
    method::{
        JsonRpcMethodDiscoveryConfig, JsonRpcMethodSourceConfig, MethodSourceConfig, ProviderName,
    },
};
use actrpc_transport::{
    JsonRpcClient, JsonRpcClientProvider, JsonRpcClientProviderFuture, JsonRpcSession,
    JsonRpcSessionProvider, JsonRpcSessionProviderFuture, TransportError, TransportTarget,
};
use serde_json::json;
use std::sync::Arc;

#[test]
fn legacy_target_deserializes_as_auto_json_rpc2() {
    let config: EndpointConfig = serde_yaml::from_str(
        r#"
name: math_ep
target:
  http:
    url: "http://example.invalid/rpc"
    headers: []
    timeout_ms: 1000
"#,
    )
    .unwrap();

    assert_eq!(config.name, EndpointName::from("math_ep"));
    assert!(matches!(config.protocol, ProtocolConfig::JsonRpc2(_)));
    if let ProtocolConfig::JsonRpc2(protocol) = config.protocol {
        assert_eq!(protocol.mode, JsonRpc2Mode::Auto);
    }
}

#[test]
fn new_json_rpc2_without_mode_defaults_to_auto() {
    let config: EndpointConfig = serde_yaml::from_str(
        r#"
name: worker
transport:
  tcp:
    addr: "127.0.0.1:9000"
protocol:
  json_rpc2: {}
"#,
    )
    .unwrap();

    if let ProtocolConfig::JsonRpc2(protocol) = config.protocol {
        assert_eq!(protocol.mode, JsonRpc2Mode::Auto);
    } else {
        panic!("expected json_rpc2 protocol");
    }
}

struct NoopClientProvider;
struct NoopSessionProvider;

impl JsonRpcClientProvider for NoopClientProvider {
    type Error = TransportError;
    type Client = Arc<dyn JsonRpcClient<Error = TransportError>>;

    fn get_client<'a>(
        &'a self,
        _target: &'a TransportTarget,
    ) -> JsonRpcClientProviderFuture<'a, Result<Self::Client, Self::Error>> {
        Box::pin(async move {
            Err(TransportError::Internal {
                message: "not implemented".to_owned(),
            })
        })
    }
}

impl JsonRpcSessionProvider for NoopSessionProvider {
    type Error = TransportError;
    type Session = Arc<dyn JsonRpcSession<Error = TransportError>>;

    fn get_session<'a>(
        &'a self,
        _target: &'a TransportTarget,
    ) -> JsonRpcSessionProviderFuture<'a, Result<Self::Session, Self::Error>> {
        Box::pin(async move {
            Err(TransportError::Internal {
                message: "not implemented".to_owned(),
            })
        })
    }
}

#[tokio::test]
async fn rest_http_over_stdio_errors() {
    let config: EndpointConfig = serde_json::from_value(json!({
        "name": "openai",
        "transport": {
            "stdio": {
                "program": "noop",
                "args": []
            }
        },
        "protocol": { "rest_http": {} }
    }))
    .unwrap();

    let result = actrpc_orchestrator::EndpointCatalog::from_configs(
        vec![config],
        &[],
        &[],
        &NoopClientProvider,
        &NoopSessionProvider,
    )
    .await;

    let Err(OrchestratorError::Config(config_err)) = result else {
        panic!("expected config error");
    };
    assert!(config_err.to_string().contains("rest_http"));
}

#[tokio::test]
async fn watchable_provider_with_request_response_mode_errors() {
    let endpoint = EndpointName::from("worker_ep");
    let methods = vec![MethodSourceConfig::JsonRpc(JsonRpcMethodSourceConfig {
        provider: ProviderName::from("worker"),
        endpoint: endpoint.clone(),
        discovery: JsonRpcMethodDiscoveryConfig::Watchable {
            initialize_method: "actrpc.method_provider.initialize".to_owned(),
            refresh_method: "actrpc.method_provider.refresh".to_owned(),
        },
    })];

    let config: EndpointConfig = serde_json::from_value(json!({
        "name": "worker_ep",
        "transport": {
            "tcp": { "addr": "127.0.0.1:9000", "framing": "newline_delimited" }
        },
        "protocol": {
            "json_rpc2": { "mode": "request_response" }
        }
    }))
    .unwrap();

    let result = actrpc_orchestrator::EndpointCatalog::from_configs(
        vec![config],
        &methods,
        &[],
        &NoopClientProvider,
        &NoopSessionProvider,
    )
    .await;

    let Err(OrchestratorError::Config(config_err)) = result else {
        panic!("expected config error");
    };
    assert!(config_err.to_string().contains("worker"));
    assert!(config_err.to_string().contains("session"));
}
