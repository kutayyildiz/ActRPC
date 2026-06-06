use crate::{
    TransportError, framing,
    session::{
        JsonRpcSession, JsonRpcSessionEvent, JsonRpcSessionFuture,
        stream_loop::{SessionCore, session_notify, session_request},
    },
    target::StdioTarget,
};
use actrpc_core::json_rpc::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
use std::{process::Stdio, sync::Arc};
use tokio::{
    io::BufReader,
    process::{Child, ChildStdin, ChildStdout, Command},
    sync::Mutex,
    task::JoinHandle,
};

pub struct StdioJsonRpcSession {
    core: Arc<SessionCore>,
    writer: Arc<Mutex<ChildStdin>>,
    framing: framing::StreamFraming,
    _child: Child,
    _reader: JoinHandle<()>,
}

impl StdioJsonRpcSession {
    pub fn new(target: StdioTarget) -> Result<Self, TransportError> {
        let mut command = Command::new(&target.program);
        command
            .args(&target.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);

        for (key, value) in target.env {
            command.env(key, value);
        }

        let mut child = command
            .spawn()
            .map_err(|source| TransportError::Connection {
                message: format!(
                    "failed to spawn stdio target '{}': {source}",
                    target.program
                ),
            })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| TransportError::ClientInit {
                message: "stdio child did not expose stdin".to_owned(),
            })?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| TransportError::ClientInit {
                message: "stdio child did not expose stdout".to_owned(),
            })?;

        let framing = target.framing;
        let (event_tx, _) = tokio::sync::broadcast::channel(64);
        let core = Arc::new(SessionCore::new(event_tx));
        let writer = Arc::new(Mutex::new(stdin));

        let reader_core = core.clone();
        let reader = tokio::spawn(async move {
            run_reader(stdout, framing, reader_core).await;
        });

        Ok(Self {
            core,
            writer,
            framing,
            _child: child,
            _reader: reader,
        })
    }
}

impl JsonRpcSession for StdioJsonRpcSession {
    type Error = TransportError;

    fn request<'a>(
        &'a self,
        request: JsonRpcRequest,
    ) -> JsonRpcSessionFuture<'a, Result<JsonRpcResponse, Self::Error>> {
        let core = self.core.clone();
        let writer = self.writer.clone();
        let framing = self.framing;

        Box::pin(async move {
            session_request(&core, request, |message| {
                let writer = writer.clone();
                async move { write_message(writer, framing, &message).await }
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

        Box::pin(async move {
            session_notify(
                |message| {
                    let writer = writer.clone();
                    async move { write_message(writer, framing, &message).await }
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

async fn run_reader(stdout: ChildStdout, framing: framing::StreamFraming, core: Arc<SessionCore>) {
    let mut reader = BufReader::new(stdout);
    let close_message = loop {
        let message = match framing::read_message(&mut reader, framing).await {
            Ok(message) => message,
            Err(error) => break error.to_string(),
        };

        core.dispatch_message(message).await;
    };

    core.close_with_error(close_message).await;
}

async fn write_message(
    writer: Arc<Mutex<ChildStdin>>,
    framing: framing::StreamFraming,
    message: &actrpc_core::json_rpc::JsonRpcMessage,
) -> Result<(), TransportError> {
    let mut writer = writer.lock().await;
    framing::write_message(&mut *writer, framing, message).await
}
