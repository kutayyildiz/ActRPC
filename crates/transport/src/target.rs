use crate::framing::StreamFraming;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportTarget {
    Stdio(StdioTarget),
    Tcp(TcpTarget),
    LocalIpc(LocalIpcTarget),
    Http(HttpTarget),
    WebSocket(WebSocketTarget),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StdioTarget {
    pub program: String,

    #[serde(default)]
    pub args: Vec<String>,

    #[serde(default)]
    pub env: Vec<(String, String)>,

    #[serde(default)]
    pub framing: StreamFraming,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TcpTarget {
    pub addr: String,

    #[serde(default)]
    pub framing: StreamFraming,

    #[serde(default = "default_connect_timeout_ms")]
    pub connect_timeout_ms: u64,

    #[serde(default = "default_read_timeout_ms")]
    pub read_timeout_ms: u64,

    #[serde(default = "default_write_timeout_ms")]
    pub write_timeout_ms: u64,

    #[serde(default = "default_tcp_nodelay")]
    pub nodelay: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalIpcTarget {
    pub name: String,

    #[serde(default)]
    pub namespace: LocalIpcNamespace,

    #[serde(default)]
    pub framing: StreamFraming,

    #[serde(default = "default_connect_timeout_ms")]
    pub connect_timeout_ms: u64,

    #[serde(default = "default_read_timeout_ms")]
    pub read_timeout_ms: u64,

    #[serde(default = "default_write_timeout_ms")]
    pub write_timeout_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LocalIpcNamespace {
    /// Prefer OS namespaced local socket if supported.
    ///
    /// Falls back to filesystem path if namespaced sockets are unsupported.
    #[default]
    Generic,

    /// Force namespaced local socket.
    Namespaced,

    /// Force filesystem-path local socket.
    FilePath,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HttpTarget {
    pub url: String,

    #[serde(default)]
    pub headers: Vec<(String, String)>,

    #[serde(default = "default_http_timeout_ms")]
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebSocketTarget {
    pub url: String,

    #[serde(default)]
    pub headers: Vec<(String, String)>,

    #[serde(default = "default_connect_timeout_ms")]
    pub connect_timeout_ms: u64,

    #[serde(default = "default_read_timeout_ms")]
    pub read_timeout_ms: u64,

    #[serde(default = "default_write_timeout_ms")]
    pub write_timeout_ms: u64,
}

fn default_connect_timeout_ms() -> u64 {
    10_000
}
fn default_read_timeout_ms() -> u64 {
    30_000
}
fn default_write_timeout_ms() -> u64 {
    30_000
}

fn default_tcp_nodelay() -> bool {
    true
}
fn default_http_timeout_ms() -> u64 {
    30_000
}
