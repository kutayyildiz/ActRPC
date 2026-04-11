#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TransportTarget {
    Stdio(StdioTarget),
    Tcp(TcpTarget),
    Unix(UnixTarget),
    Http(HttpTarget),
    WebSocket(WebSocketTarget),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StdioTarget {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TcpTarget {
    pub addr: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnixTarget {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HttpTarget {
    pub url: String,
    pub headers: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WebSocketTarget {
    pub url: String,
    pub headers: Vec<(String, String)>,
}
