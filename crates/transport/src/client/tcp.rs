use crate::{
    TransportError,
    client::{JsonRpcClient, JsonRpcClientFuture},
    framing,
    target::TcpTarget,
};
use actrpc_core::json_rpc::JsonRpcMessage;
use std::{
    net::{SocketAddr, ToSocketAddrs},
    time::Duration,
};
use tokio::{
    io::{BufReader, ReadHalf, WriteHalf},
    net::TcpStream,
    sync::Mutex,
    time,
};

/// JSON-RPC client over TCP using configurable stream framing.
///
/// Framing is configured by `TcpTarget::framing`.
#[derive(Debug)]
pub struct TcpJsonRpcClient {
    inner: Mutex<TcpConnection>,
}

#[derive(Debug)]
struct TcpConnection {
    reader: BufReader<ReadHalf<TcpStream>>,
    writer: WriteHalf<TcpStream>,
    framing: framing::StreamFraming,
    read_timeout_ms: u64,
    write_timeout_ms: u64,
}

impl TcpJsonRpcClient {
    pub async fn new(target: TcpTarget) -> Result<Self, TransportError> {
        let stream = connect_tcp(&target).await?;
        let (reader, writer) = tokio::io::split(stream);

        Ok(Self {
            inner: Mutex::new(TcpConnection {
                reader: BufReader::new(reader),
                writer,
                framing: target.framing,
                read_timeout_ms: target.read_timeout_ms,
                write_timeout_ms: target.write_timeout_ms,
            }),
        })
    }
}

impl JsonRpcClient for TcpJsonRpcClient {
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

impl TcpConnection {
    async fn write_message(&mut self, message: &JsonRpcMessage) -> Result<(), TransportError> {
        time::timeout(
            Duration::from_millis(self.write_timeout_ms),
            framing::write_message(&mut self.writer, self.framing, message),
        )
        .await
        .map_err(|_| TransportError::Timeout)?
    }

    async fn read_message(&mut self) -> Result<JsonRpcMessage, TransportError> {
        time::timeout(
            Duration::from_millis(self.read_timeout_ms),
            framing::read_message(&mut self.reader, self.framing),
        )
        .await
        .map_err(|_| TransportError::Timeout)?
    }
}

async fn connect_tcp(target: &TcpTarget) -> Result<TcpStream, TransportError> {
    let addrs = target
        .addr
        .to_socket_addrs()
        .map_err(|source| TransportError::Connection {
            message: format!("failed to resolve TCP target '{}': {source}", target.addr),
        })?;

    let stream = connect_to_any_resolved_addr(addrs, target).await?;

    configure_tcp_stream(&stream, target)?;

    Ok(stream)
}

async fn connect_to_any_resolved_addr<I>(
    addrs: I,
    target: &TcpTarget,
) -> Result<TcpStream, TransportError>
where
    I: IntoIterator<Item = SocketAddr>,
{
    let connect_timeout = Duration::from_millis(target.connect_timeout_ms);
    let mut last_error = None;

    for addr in addrs {
        match time::timeout(connect_timeout, TcpStream::connect(addr)).await {
            Ok(Ok(stream)) => return Ok(stream),
            Ok(Err(source)) => {
                last_error = Some(source);
            }
            Err(_) => {
                return Err(TransportError::Timeout);
            }
        }
    }

    Err(TransportError::Connection {
        message: match last_error {
            Some(source) => {
                format!(
                    "failed to connect to TCP target '{}': {source}",
                    target.addr
                )
            }
            None => {
                format!(
                    "TCP target '{}' resolved to no socket addresses",
                    target.addr
                )
            }
        },
    })
}

fn configure_tcp_stream(stream: &TcpStream, target: &TcpTarget) -> Result<(), TransportError> {
    stream
        .set_nodelay(target.nodelay)
        .map_err(|source| TransportError::Io {
            message: format!(
                "failed to configure TCP_NODELAY for '{}': {source}",
                target.addr
            ),
        })?;

    Ok(())
}
