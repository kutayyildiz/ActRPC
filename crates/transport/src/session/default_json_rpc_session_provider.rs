use crate::{
    TransportError,
    session::{
        JsonRpcSession,
        json_rpc_session_provider::{JsonRpcSessionProvider, JsonRpcSessionProviderFuture},
        local_ipc::LocalIpcJsonRpcSession,
        stdio::StdioJsonRpcSession,
        tcp::TcpJsonRpcSession,
        web_socket::WebSocketJsonRpcSession,
    },
    target::TransportTarget,
};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

pub struct DefaultJsonRpcSessionProvider {
    cache: RwLock<HashMap<TransportTarget, Arc<dyn JsonRpcSession<Error = TransportError>>>>,
}

impl DefaultJsonRpcSessionProvider {
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
        }
    }

    pub fn clear_cache(&self) {
        self.cache
            .write()
            .expect("poisoned JSON-RPC session cache lock")
            .clear();
    }

    pub fn remove_cached_session(
        &self,
        target: &TransportTarget,
    ) -> Option<Arc<dyn JsonRpcSession<Error = TransportError>>> {
        self.cache
            .write()
            .expect("poisoned JSON-RPC session cache lock")
            .remove(target)
    }
}

impl Default for DefaultJsonRpcSessionProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonRpcSessionProvider for DefaultJsonRpcSessionProvider {
    type Error = TransportError;
    type Session = Arc<dyn JsonRpcSession<Error = TransportError>>;

    fn get_session<'a>(
        &'a self,
        target: &'a TransportTarget,
    ) -> JsonRpcSessionProviderFuture<'a, Result<Self::Session, Self::Error>> {
        Box::pin(async move {
            if let Some(session) = {
                let cache = self
                    .cache
                    .read()
                    .expect("poisoned JSON-RPC session cache lock");

                cache.get(target).cloned()
            } {
                return Ok(session);
            }

            let session: Arc<dyn JsonRpcSession<Error = TransportError>> = match target {
                TransportTarget::Stdio(target) => {
                    Arc::new(StdioJsonRpcSession::new(target.clone())?)
                }

                TransportTarget::Tcp(target) => {
                    Arc::new(TcpJsonRpcSession::new(target.clone()).await?)
                }

                TransportTarget::LocalIpc(target) => {
                    Arc::new(LocalIpcJsonRpcSession::new(target.clone()).await?)
                }

                TransportTarget::WebSocket(target) => {
                    Arc::new(WebSocketJsonRpcSession::new(target.clone()).await?)
                }

                TransportTarget::Http(_) => {
                    return Err(TransportError::UnsupportedTarget {
                        target: "http".to_owned(),
                        message: "HTTP does not support persistent JSON-RPC sessions".to_owned(),
                    });
                }
            };

            {
                let mut cache = self
                    .cache
                    .write()
                    .expect("poisoned JSON-RPC session cache lock");

                let entry = cache
                    .entry(target.clone())
                    .or_insert_with(|| session.clone());

                Ok(entry.clone())
            }
        })
    }
}
