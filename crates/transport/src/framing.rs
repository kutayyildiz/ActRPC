mod content_length;
mod newline_delimited;

use crate::TransportError;
use actrpc_core::json_rpc::JsonRpcMessage;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufRead, AsyncWrite};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StreamFraming {
    #[default]
    NewlineDelimited,
    ContentLength,
}

pub async fn write_message<W>(
    writer: &mut W,
    framing: StreamFraming,
    message: &JsonRpcMessage,
) -> Result<(), TransportError>
where
    W: AsyncWrite + Unpin + Send,
{
    match framing {
        StreamFraming::NewlineDelimited => newline_delimited::write_message(writer, message).await,
        StreamFraming::ContentLength => content_length::write_message(writer, message).await,
    }
}

pub async fn read_message<R>(
    reader: &mut R,
    framing: StreamFraming,
) -> Result<JsonRpcMessage, TransportError>
where
    R: AsyncBufRead + Unpin + Send,
{
    match framing {
        StreamFraming::NewlineDelimited => newline_delimited::read_message(reader).await,
        StreamFraming::ContentLength => content_length::read_message(reader).await,
    }
}
