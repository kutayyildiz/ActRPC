use crate::{
    TransportError,
    client::{JsonRpcClient, JsonRpcClientFuture},
    framing,
    target::StdioTarget,
};
use actrpc_core::json_rpc::JsonRpcMessage;
use std::process::Stdio;
use tokio::{
    io::BufReader,
    process::{Child, ChildStdin, ChildStdout, Command},
    sync::Mutex,
};

#[derive(Debug)]
pub struct StdioJsonRpcClient {
    inner: Mutex<StdioConnection>,
}

#[derive(Debug)]
struct StdioConnection {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    framing: framing::StreamFraming,
}

impl StdioJsonRpcClient {
    pub fn new(target: StdioTarget) -> Result<Self, TransportError> {
        let mut command = Command::new(&target.program);

        command
            .args(&target.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

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

        Ok(Self {
            inner: Mutex::new(StdioConnection {
                child,
                stdin,
                stdout: BufReader::new(stdout),
                framing: target.framing,
            }),
        })
    }

    pub fn from_parts(child: Child, stdin: ChildStdin, stdout: ChildStdout) -> Self {
        Self::from_parts_with_framing(child, stdin, stdout, framing::StreamFraming::default())
    }

    pub fn from_parts_with_framing(
        child: Child,
        stdin: ChildStdin,
        stdout: ChildStdout,
        framing: framing::StreamFraming,
    ) -> Self {
        Self {
            inner: Mutex::new(StdioConnection {
                child,
                stdin,
                stdout: BufReader::new(stdout),
                framing,
            }),
        }
    }
}

impl JsonRpcClient for StdioJsonRpcClient {
    type Error = TransportError;

    fn send<'a>(
        &'a self,
        message: JsonRpcMessage,
    ) -> JsonRpcClientFuture<'a, Result<JsonRpcMessage, Self::Error>> {
        Box::pin(async move {
            let mut inner = self.inner.lock().await;

            inner.ensure_child_running()?;
            inner.write_message(&message).await?;
            inner.read_message().await
        })
    }
}

impl StdioConnection {
    fn ensure_child_running(&mut self) -> Result<(), TransportError> {
        match self.child.try_wait() {
            Ok(Some(status)) => Err(TransportError::Connection {
                message: format!("stdio child exited before request completed: {status}"),
            }),

            Ok(None) => Ok(()),

            Err(source) => Err(TransportError::Io {
                message: format!("failed to inspect stdio child status: {source}"),
            }),
        }
    }

    async fn write_message(&mut self, message: &JsonRpcMessage) -> Result<(), TransportError> {
        framing::write_message(&mut self.stdin, self.framing, message).await
    }

    async fn read_message(&mut self) -> Result<JsonRpcMessage, TransportError> {
        framing::read_message(&mut self.stdout, self.framing).await
    }
}
