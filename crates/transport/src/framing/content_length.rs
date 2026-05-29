use crate::TransportError;
use actrpc_core::{error::CodecError, json_rpc::JsonRpcMessage};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const CONTENT_LENGTH_HEADER: &str = "content-length";

pub async fn write_message<W>(
    writer: &mut W,
    message: &JsonRpcMessage,
) -> Result<(), TransportError>
where
    W: AsyncWrite + Unpin + Send,
{
    let payload =
        serde_json::to_vec(message).map_err(|source| CodecError::Serialize(source.to_string()))?;

    let header = format!("Content-Length: {}\r\n\r\n", payload.len());

    writer
        .write_all(header.as_bytes())
        .await
        .map_err(|source| TransportError::Io {
            message: format!("failed to write content-length JSON-RPC header: {source}"),
        })?;

    writer
        .write_all(&payload)
        .await
        .map_err(|source| TransportError::Io {
            message: format!("failed to write content-length JSON-RPC payload: {source}"),
        })?;

    writer.flush().await.map_err(|source| TransportError::Io {
        message: format!("failed to flush content-length JSON-RPC frame: {source}"),
    })?;

    Ok(())
}

pub async fn read_message<R>(reader: &mut R) -> Result<JsonRpcMessage, TransportError>
where
    R: AsyncBufRead + Unpin + Send,
{
    let content_length = read_content_length(reader).await?;

    let mut payload = vec![0u8; content_length];

    reader
        .read_exact(&mut payload)
        .await
        .map_err(|source| TransportError::Io {
            message: format!("failed to read content-length JSON-RPC payload: {source}"),
        })?;

    serde_json::from_slice::<JsonRpcMessage>(&payload)
        .map_err(|source| CodecError::Deserialize(source.to_string()).into())
}

async fn read_content_length<R>(reader: &mut R) -> Result<usize, TransportError>
where
    R: AsyncBufRead + Unpin + Send,
{
    let mut content_length = None;

    loop {
        let mut line = String::new();

        let bytes_read =
            reader
                .read_line(&mut line)
                .await
                .map_err(|source| TransportError::Io {
                    message: format!("failed to read content-length JSON-RPC header: {source}"),
                })?;

        if bytes_read == 0 {
            return Err(TransportError::Connection {
                message: "peer closed connection while reading content-length JSON-RPC headers"
                    .to_owned(),
            });
        }

        let line = line.trim_end_matches(['\r', '\n']);

        if line.is_empty() {
            break;
        }

        let Some((name, value)) = line.split_once(':') else {
            return Err(TransportError::Codec(CodecError::Deserialize(format!(
                "invalid content-length JSON-RPC header line: {line}"
            ))));
        };

        if name.trim().eq_ignore_ascii_case(CONTENT_LENGTH_HEADER) {
            let parsed = value.trim().parse::<usize>().map_err(|source| {
                TransportError::Codec(CodecError::Deserialize(format!(
                    "invalid Content-Length value '{}': {source}",
                    value.trim()
                )))
            })?;

            content_length = Some(parsed);
        }
    }

    content_length.ok_or_else(|| {
        TransportError::Codec(CodecError::Deserialize(
            "missing Content-Length header in JSON-RPC frame".to_owned(),
        ))
    })
}
