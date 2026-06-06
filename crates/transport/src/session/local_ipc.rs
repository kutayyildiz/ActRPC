use crate::{
    TransportError, framing,
    session::{
        JsonRpcSession, JsonRpcSessionEvent, JsonRpcSessionFuture,
        stream_loop::{SessionCore, session_notify, session_request},
    },
    target::{LocalIpcNamespace, LocalIpcTarget},
};
use actrpc_core::json_rpc::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
use interprocess::local_socket::{
    GenericFilePath, GenericNamespaced, Name, prelude::*, tokio::Stream as LocalSocketStream,
    traits::tokio::Stream as LocalSocketStreamTrait,
};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{BufReader, ReadHalf, WriteHalf},
    sync::Mutex,
    task::JoinHandle,
    time,
};

pub struct LocalIpcJsonRpcSession {
    core: Arc<SessionCore>,
    writer: Arc<Mutex<WriteHalf<LocalSocketStream>>>,
    framing: framing::StreamFraming,
    write_timeout_ms: u64,
    _reader: JoinHandle<()>,
}

impl LocalIpcJsonRpcSession {
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

        let (reader, writer) = tokio::io::split(stream);
        let framing = target.framing;
        let read_timeout_ms = target.read_timeout_ms;
        let write_timeout_ms = target.write_timeout_ms;

        let (event_tx, _) = tokio::sync::broadcast::channel(64);
        let core = Arc::new(SessionCore::new(event_tx));
        let writer = Arc::new(Mutex::new(writer));

        let reader_core = core.clone();
        let reader = tokio::spawn(async move {
            run_reader(reader, framing, read_timeout_ms, reader_core).await;
        });

        Ok(Self {
            core,
            writer,
            framing,
            write_timeout_ms,
            _reader: reader,
        })
    }
}

impl JsonRpcSession for LocalIpcJsonRpcSession {
    type Error = TransportError;

    fn request<'a>(
        &'a self,
        request: JsonRpcRequest,
    ) -> JsonRpcSessionFuture<'a, Result<JsonRpcResponse, Self::Error>> {
        let core = self.core.clone();
        let writer = self.writer.clone();
        let framing = self.framing;
        let write_timeout_ms = self.write_timeout_ms;

        Box::pin(async move {
            session_request(&core, request, |message| {
                let writer = writer.clone();
                async move { write_message(writer, framing, write_timeout_ms, &message).await }
            })
            .await
        })
    }

    fn notify<'a>(
        &'a self,
        notification: JsonRpcNotification,
    ) -> JsonRpcSessionFuture<'a, Result<(), Self::Error>> {
        let writer = self.writer.clone();
        let framing = self.framing;
        let write_timeout_ms = self.write_timeout_ms;

        Box::pin(async move {
            session_notify(
                |message| {
                    let writer = writer.clone();
                    async move { write_message(writer, framing, write_timeout_ms, &message).await }
                },
                notification,
            )
            .await
        })
    }

    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<JsonRpcSessionEvent> {
        self.core.subscribe()
    }
}

async fn run_reader(
    reader: ReadHalf<LocalSocketStream>,
    framing: framing::StreamFraming,
    read_timeout_ms: u64,
    core: Arc<SessionCore>,
) {
    let mut reader = BufReader::new(reader);
    let close_message = loop {
        let message = match time::timeout(
            Duration::from_millis(read_timeout_ms),
            framing::read_message(&mut reader, framing),
        )
        .await
        {
            Ok(Ok(message)) => message,
            Ok(Err(error)) => break error.to_string(),
            Err(_) => break "local IPC session read timed out".to_owned(),
        };

        core.dispatch_message(message).await;
    };

    core.close_with_error(close_message).await;
}

async fn write_message(
    writer: Arc<Mutex<WriteHalf<LocalSocketStream>>>,
    framing: framing::StreamFraming,
    write_timeout_ms: u64,
    message: &actrpc_core::json_rpc::JsonRpcMessage,
) -> Result<(), TransportError> {
    let mut writer = writer.lock().await;

    time::timeout(
        Duration::from_millis(write_timeout_ms),
        framing::write_message(&mut *writer, framing, message),
    )
    .await
    .map_err(|_| TransportError::Timeout)?
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
