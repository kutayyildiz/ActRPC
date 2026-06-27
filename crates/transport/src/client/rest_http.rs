use crate::{
    TransportError, rest_url::join_base_url_and_path, sensitive_headers::HeaderPairs,
    target::HttpTarget,
};
use reqwest::{
    Client, Method,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use std::{future::Future, pin::Pin};
use std::{str::FromStr, time::Duration};

type RestHttpClientFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Clone)]
pub struct RestHttpExecuteRequest {
    pub method: String,
    pub path: String,
    pub headers: HeaderPairs,
    pub body: Option<Vec<u8>>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct RestHttpExecuteResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

impl std::fmt::Debug for RestHttpExecuteResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RestHttpExecuteResponse")
            .field("status", &self.status)
            .field("body_len", &self.body.len())
            .finish()
    }
}

/// HTTP client for REST (non-JSON-RPC) requests.
#[derive(Clone)]
pub struct HttpRestClient {
    client: Client,
    target: HttpTarget,
}

impl std::fmt::Debug for HttpRestClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpRestClient")
            .field("target", &self.target)
            .finish_non_exhaustive()
    }
}

impl HttpRestClient {
    pub fn new(target: HttpTarget) -> Result<Self, TransportError> {
        let client = Client::builder()
            .timeout(Duration::from_millis(target.timeout_ms))
            .build()
            .map_err(|source| TransportError::ClientInit {
                message: format!("failed to initialize HTTP REST client: {source}"),
            })?;

        Ok(Self { client, target })
    }

    pub fn execute<'a>(
        &'a self,
        request: RestHttpExecuteRequest,
    ) -> RestHttpClientFuture<'a, Result<RestHttpExecuteResponse, TransportError>> {
        Box::pin(async move {
            let url = join_base_url_and_path(&self.target.url, &request.path)?;
            let method =
                Method::from_str(&request.method).map_err(|_| TransportError::Internal {
                    message: format!("invalid HTTP method '{}'", request.method),
                })?;

            let headers = build_headers(&self.target, &request.headers)?;

            let mut builder = self.client.request(method, url).headers(headers);
            if let Some(body) = request.body {
                builder = builder.body(body);
            }

            let response = builder.send().await.map_err(map_reqwest_error)?;
            let status = response.status().as_u16();
            let body = response.bytes().await.map_err(map_reqwest_error)?.to_vec();

            Ok(RestHttpExecuteResponse { status, body })
        })
    }
}

fn build_headers(target: &HttpTarget, extra: &HeaderPairs) -> Result<HeaderMap, TransportError> {
    let mut headers = HeaderMap::new();

    for (name, value) in target.headers.iter().chain(extra.iter()) {
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
        return TransportError::Codec(actrpc_core::error::CodecError::Deserialize(
            source.to_string(),
        ));
    }

    TransportError::Io {
        message: format!("HTTP transport error: {source}"),
    }
}
