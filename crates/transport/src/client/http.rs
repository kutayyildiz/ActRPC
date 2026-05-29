use crate::{
    TransportError,
    client::{JsonRpcClient, JsonRpcClientFuture},
    target::HttpTarget,
};
use actrpc_core::{error::CodecError, json_rpc::JsonRpcMessage};
use reqwest::{
    Client,
    header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue},
};
use std::{str::FromStr, time::Duration};

/// JSON-RPC client over HTTP.
///
/// HTTP is a message transport:
/// - outbound: one HTTP POST body containing one JSON-RPC message
/// - inbound: one HTTP response body containing one JSON-RPC message
///
/// Stream framing is intentionally not used here.
#[derive(Debug, Clone)]
pub struct HttpJsonRpcClient {
    client: Client,
    target: HttpTarget,
}

impl HttpJsonRpcClient {
    pub fn new(target: HttpTarget) -> Result<Self, TransportError> {
        let client = Client::builder()
            .timeout(Duration::from_millis(target.timeout_ms))
            .build()
            .map_err(|source| TransportError::ClientInit {
                message: format!("failed to initialize HTTP client: {source}"),
            })?;

        Ok(Self { client, target })
    }
}

impl JsonRpcClient for HttpJsonRpcClient {
    type Error = TransportError;

    fn send<'a>(
        &'a self,
        message: JsonRpcMessage,
    ) -> JsonRpcClientFuture<'a, Result<JsonRpcMessage, Self::Error>> {
        Box::pin(async move {
            let headers = build_headers(&self.target)?;

            let body = serde_json::to_vec(&message)
                .map_err(|source| CodecError::Serialize(source.to_string()))?;

            let response = self
                .client
                .post(&self.target.url)
                .headers(headers)
                .body(body)
                .send()
                .await
                .map_err(map_reqwest_error)?;

            let status = response.status();

            if !status.is_success() {
                let body = response.text().await.map_err(map_reqwest_error)?;

                return Err(TransportError::HttpStatus {
                    status: status.as_u16(),
                    body,
                });
            }

            let bytes = response.bytes().await.map_err(map_reqwest_error)?;

            serde_json::from_slice::<JsonRpcMessage>(&bytes)
                .map_err(|source| CodecError::Deserialize(source.to_string()).into())
        })
    }
}

fn build_headers(target: &HttpTarget) -> Result<HeaderMap, TransportError> {
    let mut headers = HeaderMap::new();

    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

    for (name, value) in &target.headers {
        let header_name =
            HeaderName::from_str(name).map_err(|source| TransportError::ClientInit {
                message: format!("invalid HTTP header name '{name}': {source}"),
            })?;

        let header_value =
            HeaderValue::from_str(value).map_err(|source| TransportError::ClientInit {
                message: format!("invalid HTTP header value for '{name}': {source}"),
            })?;

        headers.insert(header_name, header_value);
    }

    Ok(headers)
}

fn map_reqwest_error(source: reqwest::Error) -> TransportError {
    if source.is_timeout() {
        return TransportError::Timeout;
    }

    if source.is_connect() {
        return TransportError::Connection {
            message: format!("HTTP connection failed: {source}"),
        };
    }

    if source.is_decode() {
        return TransportError::Codec(CodecError::Deserialize(source.to_string()));
    }

    TransportError::Io {
        message: format!("HTTP transport error: {source}"),
    }
}
