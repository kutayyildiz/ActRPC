use crate::{
    TransportError,
    client::{JsonRpcClient, JsonRpcClientFuture},
    framing,
    target::{LocalIpcNamespace, LocalIpcTarget},
};
use actrpc_core::json_rpc::JsonRpcMessage;
use interprocess::local_socket::{
    GenericFilePath, GenericNamespaced, Name, prelude::*, tokio::Stream as LocalSocketStream,
    traits::tokio::Stream as LocalSocketStreamTrait,
};
use tokio::{
    io::BufReader,
    sync::Mutex,
    time::{self, Duration},
};

/// JSON-RPC client over cross-platform local IPC.
///
/// Backend behavior is delegated to `interprocess`:
/// - Unix-like systems: local socket / Unix-domain-socket-style backend
/// - Windows: named-pipe-style backend
///
/// Framing is configured by `LocalIpcTarget::framing`.
#[derive(Debug)]
pub struct LocalIpcJsonRpcClient {
    inner: Mutex<LocalIpcConnection>,
}

#[derive(Debug)]
struct LocalIpcConnection {
    stream: BufReader<LocalSocketStream>,
    framing: framing::StreamFraming,
    read_timeout_ms: u64,
    write_timeout_ms: u64,
}

impl LocalIpcJsonRpcClient {
    pub async fn new(target: LocalIpcTarget) -> Result<Self, TransportError> {
        let name = build_local_socket_name(&target)?;

        let stream = time::timeout(
            Duration::from_millis(target.connect_timeout_ms),
            LocalSocketStream::connect(name),
        )
        .await
        .map_err(|_| TransportError::Timeout)?
        .map_err(|source| TransportError::Connection {
            message: format!(
                "failed to connect to local IPC target '{}': {source}",
                target.name
            ),
        })?;

        Ok(Self {
            inner: Mutex::new(LocalIpcConnection {
                stream: BufReader::new(stream),
                framing: target.framing,
                read_timeout_ms: target.read_timeout_ms,
                write_timeout_ms: target.write_timeout_ms,
            }),
        })
    }
}

impl JsonRpcClient for LocalIpcJsonRpcClient {
    type Error = TransportError;

    fn send<'a>(
        &'a self,
        message: JsonRpcMessage,
    ) -> JsonRpcClientFuture<'a, Result<JsonRpcMessage, Self::Error>> {
        Box::pin(async move {
            let mut inner = self.inner.lock().await;

            inner.write_message(&message).await?;
            inner.read_message().await
        })
    }
}

impl LocalIpcConnection {
    async fn write_message(&mut self, message: &JsonRpcMessage) -> Result<(), TransportError> {
        time::timeout(
            Duration::from_millis(self.write_timeout_ms),
            framing::write_message(self.stream.get_mut(), self.framing, message),
        )
        .await
        .map_err(|_| TransportError::Timeout)?
    }

    async fn read_message(&mut self) -> Result<JsonRpcMessage, TransportError> {
        time::timeout(
            Duration::from_millis(self.read_timeout_ms),
            framing::read_message(&mut self.stream, self.framing),
        )
        .await
        .map_err(|_| TransportError::Timeout)?
    }
}

fn build_local_socket_name(target: &LocalIpcTarget) -> Result<Name<'_>, TransportError> {
    match target.namespace {
        LocalIpcNamespace::Generic => {
            if GenericNamespaced::is_supported() {
                target
                    .name
                    .as_str()
                    .to_ns_name::<GenericNamespaced>()
                    .map_err(|source| TransportError::ClientInit {
                        message: format!(
                            "invalid namespaced local IPC name '{}': {source}",
                            target.name
                        ),
                    })
            } else {
                target
                    .name
                    .as_str()
                    .to_fs_name::<GenericFilePath>()
                    .map_err(|source| TransportError::ClientInit {
                        message: format!(
                            "invalid filesystem local IPC path '{}': {source}",
                            target.name
                        ),
                    })
            }
        }

        LocalIpcNamespace::Namespaced => target
            .name
            .as_str()
            .to_ns_name::<GenericNamespaced>()
            .map_err(|source| TransportError::ClientInit {
                message: format!(
                    "invalid namespaced local IPC name '{}': {source}",
                    target.name
                ),
            }),

        LocalIpcNamespace::FilePath => target
            .name
            .as_str()
            .to_fs_name::<GenericFilePath>()
            .map_err(|source| TransportError::ClientInit {
                message: format!(
                    "invalid filesystem local IPC path '{}': {source}",
                    target.name
                ),
            }),
    }
}
