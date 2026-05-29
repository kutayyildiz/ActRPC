use actrpc_core::json_rpc::{
    JsonRpcId, JsonRpcMessage, JsonRpcParams, JsonRpcRequest, JsonRpcSingleMessage, JsonRpcVersion,
};
use actrpc_transport::{
    DefaultJsonRpcClientProvider, JsonRpcClient, JsonRpcClientFactory, JsonRpcClientFactoryFuture,
    JsonRpcClientFuture, JsonRpcClientProvider, TransportError, TransportTarget,
    target::HttpTarget,
};
use serde_json::json;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Debug)]
struct EchoClient;

impl JsonRpcClient for EchoClient {
    type Error = TransportError;

    fn send<'a>(
        &'a self,
        message: JsonRpcMessage,
    ) -> JsonRpcClientFuture<'a, Result<JsonRpcMessage, Self::Error>> {
        Box::pin(async move { Ok(message) })
    }
}

#[derive(Debug, Clone)]
struct CountingFactory {
    calls: Arc<AtomicUsize>,
}

impl JsonRpcClientFactory for CountingFactory {
    fn create_client<'a>(
        &'a self,
        _target: &'a TransportTarget,
    ) -> JsonRpcClientFactoryFuture<
        'a,
        Result<Arc<dyn JsonRpcClient<Error = TransportError>>, TransportError>,
    > {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);

            let client: Arc<dyn JsonRpcClient<Error = TransportError>> = Arc::new(EchoClient);

            Ok(client)
        })
    }
}

#[tokio::test]
async fn test_default_provider_caches_client_per_target() {
    let calls = Arc::new(AtomicUsize::new(0));
    let provider = DefaultJsonRpcClientProvider::with_factory(CountingFactory {
        calls: calls.clone(),
    });

    let target = http_target();

    let first = provider.get_client(&target).await.unwrap();
    let second = provider.get_client(&target).await.unwrap();

    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    let request = request(1, "echo");
    let echoed = first.send(request.clone()).await.unwrap();

    assert_eq!(echoed, request);
}

#[tokio::test]
async fn test_default_provider_remove_cached_client_forces_recreate() {
    let calls = Arc::new(AtomicUsize::new(0));
    let provider = DefaultJsonRpcClientProvider::with_factory(CountingFactory {
        calls: calls.clone(),
    });

    let target = http_target();

    let first = provider.get_client(&target).await.unwrap();

    let removed = provider.remove_cached_client(&target);
    assert!(removed.is_some());

    let second = provider.get_client(&target).await.unwrap();

    assert!(!Arc::ptr_eq(&first, &second));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_default_provider_clear_cache_forces_recreate() {
    let calls = Arc::new(AtomicUsize::new(0));
    let provider = DefaultJsonRpcClientProvider::with_factory(CountingFactory {
        calls: calls.clone(),
    });

    let target = http_target();

    let first = provider.get_client(&target).await.unwrap();

    provider.clear_cache();

    let second = provider.get_client(&target).await.unwrap();

    assert!(!Arc::ptr_eq(&first, &second));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

fn http_target() -> TransportTarget {
    TransportTarget::Http(HttpTarget {
        url: "http://127.0.0.1:8080/rpc".to_owned(),
        headers: vec![],
        timeout_ms: 1_000,
    })
}

fn request(id: u64, method: impl Into<String>) -> JsonRpcMessage {
    JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
        jsonrpc: JsonRpcVersion::V2_0,
        id: JsonRpcId::Number(id.into()),
        method: method.into(),
        params: Some(JsonRpcParams::Array(vec![json!(1), json!(2)])),
    }))
}
