use crate::TransportError;
use actrpc_core::{error::CodecError, json_rpc::JsonRpcMessage};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

pub async fn write_message<W>(
    writer: &mut W,
    message: &JsonRpcMessage,
) -> Result<(), TransportError>
where
    W: AsyncWrite + Unpin + Send,
{
    let payload = serde_json::to_string(message)
        .map_err(|source| CodecError::Serialize(source.to_string()))?;

    writer
        .write_all(payload.as_bytes())
        .await
        .map_err(|source| TransportError::Io {
            message: format!("failed to write newline-delimited JSON-RPC payload: {source}"),
        })?;

    writer
        .write_all(b"\n")
        .await
        .map_err(|source| TransportError::Io {
            message: format!(
                "failed to write newline-delimited JSON-RPC frame delimiter: {source}"
            ),
        })?;

    writer.flush().await.map_err(|source| TransportError::Io {
        message: format!("failed to flush newline-delimited JSON-RPC frame: {source}"),
    })?;

    Ok(())
}

pub async fn read_message<R>(reader: &mut R) -> Result<JsonRpcMessage, TransportError>
where
    R: AsyncBufRead + Unpin + Send,
{
    let mut line = String::new();

    let bytes_read = reader
        .read_line(&mut line)
        .await
        .map_err(|source| TransportError::Io {
            message: format!("failed to read newline-delimited JSON-RPC frame: {source}"),
        })?;

    if bytes_read == 0 {
        return Err(TransportError::Connection {
            message: "peer closed connection while reading newline-delimited JSON-RPC frame"
                .to_owned(),
        });
    }

    let trimmed = line.trim();

    if trimmed.is_empty() {
        return Err(TransportError::Codec(CodecError::Deserialize(
            "received empty newline-delimited JSON-RPC frame".to_owned(),
        )));
    }

    serde_json::from_str::<JsonRpcMessage>(trimmed)
        .map_err(|source| CodecError::Deserialize(source.to_string()).into())
}
