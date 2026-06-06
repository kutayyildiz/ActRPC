use crate::TransportError;
use actrpc_core::json_rpc::{
    JsonRpcBatch, JsonRpcId, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
    JsonRpcSingleMessage,
};
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::sync::{Mutex, broadcast, oneshot};

#[derive(Clone, Debug)]
pub(crate) enum JsonRpcIdKey {
    String(String),
    Number(serde_json::Number),
    Null,
}

impl JsonRpcIdKey {
    pub(crate) fn from_id(id: &JsonRpcId) -> Self {
        match id {
            JsonRpcId::String(value) => Self::String(value.clone()),
            JsonRpcId::Number(value) => Self::Number(value.clone()),
            JsonRpcId::Null => Self::Null,
        }
    }
}

impl PartialEq for JsonRpcIdKey {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::String(left), Self::String(right)) => left == right,
            (Self::Number(left), Self::Number(right)) => left == right,
            (Self::Null, Self::Null) => true,
            _ => false,
        }
    }
}

impl Eq for JsonRpcIdKey {}

impl Hash for JsonRpcIdKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::String(value) => {
                0u8.hash(state);
                value.hash(state);
            }
            Self::Number(value) => {
                1u8.hash(state);
                value.to_string().hash(state);
            }
            Self::Null => {
                2u8.hash(state);
            }
        }
    }
}

pub(crate) struct SessionCore {
    pending: Mutex<HashMap<JsonRpcIdKey, oneshot::Sender<Result<JsonRpcResponse, TransportError>>>>,
    event_tx: broadcast::Sender<super::JsonRpcSessionEvent>,
    closed: AtomicBool,
}

impl SessionCore {
    pub(crate) fn new(event_tx: broadcast::Sender<super::JsonRpcSessionEvent>) -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
            event_tx,
            closed: AtomicBool::new(false),
        }
    }

    pub(crate) fn subscribe(&self) -> broadcast::Receiver<super::JsonRpcSessionEvent> {
        self.event_tx.subscribe()
    }

    pub(crate) async fn register_pending(
        &self,
        id: &JsonRpcId,
    ) -> Result<oneshot::Receiver<Result<JsonRpcResponse, TransportError>>, TransportError> {
        let key = JsonRpcIdKey::from_id(id);
        let (tx, rx) = oneshot::channel();
        let mut pending = self.pending.lock().await;
        if pending.contains_key(&key) {
            return Err(TransportError::Internal {
                message: format!("duplicate pending JSON-RPC request id: {id:?}"),
            });
        }
        pending.insert(key, tx);
        Ok(rx)
    }

    pub(crate) async fn cancel_pending(&self, id: &JsonRpcId) {
        let mut pending = self.pending.lock().await;
        pending.remove(&JsonRpcIdKey::from_id(id));
    }

    pub(crate) async fn dispatch_message(&self, message: JsonRpcMessage) {
        match message {
            JsonRpcMessage::Single(single) => self.dispatch_single(single).await,
            JsonRpcMessage::Batch(JsonRpcBatch(items)) => {
                for item in items {
                    self.dispatch_single(item).await;
                }
            }
        }
    }

    async fn dispatch_single(&self, message: JsonRpcSingleMessage) {
        match message {
            JsonRpcSingleMessage::Response(response) => {
                let id = response.id();
                let key = JsonRpcIdKey::from_id(id);
                let mut pending = self.pending.lock().await;
                if let Some(tx) = pending.remove(&key) {
                    let _ = tx.send(Ok(response));
                }
            }
            JsonRpcSingleMessage::Notification(notification) => {
                let _ = self
                    .event_tx
                    .send(super::JsonRpcSessionEvent::Notification(notification));
            }
            JsonRpcSingleMessage::Request(_) => {}
        }
    }

    pub(crate) async fn close_with_error(&self, message: String) {
        if self.closed.swap(true, Ordering::SeqCst) {
            return;
        }

        let _ = self.event_tx.send(super::JsonRpcSessionEvent::Closed);

        let mut pending = self.pending.lock().await;
        for (_, tx) in pending.drain() {
            let _ = tx.send(Err(TransportError::Connection {
                message: message.clone(),
            }));
        }
    }
}

trait ResponseId {
    fn id(&self) -> &JsonRpcId;
}

impl ResponseId for JsonRpcResponse {
    fn id(&self) -> &JsonRpcId {
        match self {
            JsonRpcResponse::Success(response) => &response.id,
            JsonRpcResponse::Error(response) => &response.id,
        }
    }
}

pub(crate) async fn session_request<W, F>(
    core: &Arc<SessionCore>,
    request: JsonRpcRequest,
    write: W,
) -> Result<JsonRpcResponse, TransportError>
where
    W: FnOnce(JsonRpcMessage) -> F,
    F: std::future::Future<Output = Result<(), TransportError>>,
{
    let id = request.id.clone();
    let response_rx = core.register_pending(&id).await?;

    if let Err(error) = write(JsonRpcMessage::Single(JsonRpcSingleMessage::Request(
        request,
    )))
    .await
    {
        core.cancel_pending(&id).await;
        return Err(error);
    }

    match response_rx.await {
        Ok(result) => result,
        Err(_) => Err(TransportError::Connection {
            message: "JSON-RPC session closed before response was received".to_owned(),
        }),
    }
}

pub(crate) async fn session_notify<W, F>(
    write: W,
    notification: JsonRpcNotification,
) -> Result<(), TransportError>
where
    W: FnOnce(JsonRpcMessage) -> F,
    F: std::future::Future<Output = Result<(), TransportError>>,
{
    write(JsonRpcMessage::Single(JsonRpcSingleMessage::Notification(
        notification,
    )))
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JsonRpcSessionEvent;
    use actrpc_core::json_rpc::{JsonRpcSuccessResponse, JsonRpcVersion};
    use serde_json::{Number, json};
    use tokio::sync::broadcast;

    #[test]
    fn json_rpc_id_key_matches_json_rpc_id_partial_eq() {
        let cases = sample_json_rpc_ids();

        for left in &cases {
            for right in &cases {
                let key_left = JsonRpcIdKey::from_id(left);
                let key_right = JsonRpcIdKey::from_id(right);
                assert_eq!(
                    left == right,
                    key_left == key_right,
                    "key equality must mirror JsonRpcId PartialEq: {left:?} vs {right:?}"
                );
            }
        }
    }

    #[test]
    fn json_rpc_id_key_hash_consistent_with_eq() {
        let cases = sample_json_rpc_ids();

        for left in &cases {
            for right in &cases {
                let key_left = JsonRpcIdKey::from_id(left);
                let key_right = JsonRpcIdKey::from_id(right);
                if key_left == key_right {
                    assert_eq!(
                        hash_key(&key_left),
                        hash_key(&key_right),
                        "equal keys must hash equally: {left:?} vs {right:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn json_rpc_id_key_integer_one_vs_float_one_matches_json_rpc_id() {
        let int_one = JsonRpcId::Number(1u64.into());
        let float_one = JsonRpcId::Number(Number::from_f64(1.0).expect("1.0"));

        let ids_equal = int_one == float_one;
        let key_int = JsonRpcIdKey::from_id(&int_one);
        let key_float = JsonRpcIdKey::from_id(&float_one);

        assert_eq!(ids_equal, key_int == key_float);

        if ids_equal {
            assert_eq!(hash_key(&key_int), hash_key(&key_float));
        } else {
            assert_ne!(hash_key(&key_int), hash_key(&key_float));
        }
    }

    #[test]
    fn json_rpc_id_key_string_equality() {
        let left = JsonRpcIdKey::from_id(&JsonRpcId::String("abc".to_owned()));
        let right = JsonRpcIdKey::from_id(&JsonRpcId::String("abc".to_owned()));
        assert_eq!(left, right);
        assert_eq!(hash_key(&left), hash_key(&right));
    }

    #[test]
    fn json_rpc_id_key_null_equality() {
        let left = JsonRpcIdKey::from_id(&JsonRpcId::Null);
        let right = JsonRpcIdKey::from_id(&JsonRpcId::Null);
        assert_eq!(left, right);
        assert_eq!(hash_key(&left), hash_key(&right));
    }

    #[test]
    fn json_rpc_id_key_integer_number_equality() {
        let left = JsonRpcIdKey::from_id(&JsonRpcId::Number(7u64.into()));
        let right = JsonRpcIdKey::from_id(&JsonRpcId::Number(7u64.into()));
        assert_eq!(left, right);
        assert_eq!(hash_key(&left), hash_key(&right));
    }

    #[test]
    fn json_rpc_id_key_decimal_number_equality() {
        let decimal = Number::from_f64(1.5).expect("decimal number");
        let left = JsonRpcIdKey::from_id(&JsonRpcId::Number(decimal.clone()));
        let right = JsonRpcIdKey::from_id(&JsonRpcId::Number(decimal));
        assert_eq!(left, right);
        assert_eq!(hash_key(&left), hash_key(&right));
    }

    fn sample_json_rpc_ids() -> Vec<JsonRpcId> {
        vec![
            JsonRpcId::String("x".to_owned()),
            JsonRpcId::String("y".to_owned()),
            JsonRpcId::Null,
            JsonRpcId::Number(7u64.into()),
            JsonRpcId::Number(Number::from_f64(1.5).expect("decimal")),
            JsonRpcId::Number(1u64.into()),
            JsonRpcId::Number(Number::from_f64(1.0).expect("1.0")),
        ]
    }

    fn hash_key(key: &JsonRpcIdKey) -> u64 {
        use std::hash::Hash;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    #[tokio::test]
    async fn close_with_error_drains_all_pending_with_connection_error() {
        let (event_tx, _rx) = broadcast::channel(4);
        let core = SessionCore::new(event_tx);

        let rx_a = core
            .register_pending(&JsonRpcId::Number(1u64.into()))
            .await
            .expect("register pending");
        let rx_b = core
            .register_pending(&JsonRpcId::Number(2u64.into()))
            .await
            .expect("register pending");

        core.close_with_error("session closed".to_owned()).await;

        for rx in [rx_a, rx_b] {
            match rx.await.unwrap() {
                Err(TransportError::Connection { message }) => {
                    assert_eq!(message, "session closed");
                }
                other => panic!("expected connection error, got {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn session_request_duplicate_pending_id_returns_error_before_write() {
        let (event_tx, _rx) = broadcast::channel(4);
        let core = Arc::new(SessionCore::new(event_tx));

        let request = JsonRpcRequest {
            jsonrpc: JsonRpcVersion::V2_0,
            id: JsonRpcId::Number(42u64.into()),
            method: "m".to_owned(),
            params: None,
        };

        let core_first = core.clone();
        let request_first = request.clone();
        let first = tokio::spawn(async move {
            session_request(&core_first, request_first, |_| async { Ok(()) }).await
        });

        tokio::task::yield_now().await;

        let second = session_request(&core, request, |_| async {
            panic!("write must not run for duplicate pending id");
        })
        .await;

        assert!(matches!(second, Err(TransportError::Internal { .. })));

        core.close_with_error("test cleanup".to_owned()).await;
        let _ = first.await;
    }

    #[tokio::test]
    async fn session_request_write_failure_removes_pending_entry() {
        let (event_tx, _rx) = broadcast::channel(4);
        let core = Arc::new(SessionCore::new(event_tx));

        let request = JsonRpcRequest {
            jsonrpc: JsonRpcVersion::V2_0,
            id: JsonRpcId::Number(99u64.into()),
            method: "m".to_owned(),
            params: None,
        };

        let result = session_request(&core, request, |_| async {
            Err(TransportError::Io {
                message: "write failed".to_owned(),
            })
        })
        .await;

        assert!(matches!(result, Err(TransportError::Io { .. })));

        let pending = core.pending.lock().await;
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn unknown_response_id_is_ignored() {
        let (event_tx, _rx) = broadcast::channel(4);
        let core = SessionCore::new(event_tx);

        let _response_rx = core
            .register_pending(&JsonRpcId::Number(1u64.into()))
            .await
            .expect("register pending");

        core.dispatch_message(JsonRpcMessage::Single(JsonRpcSingleMessage::Response(
            JsonRpcResponse::Success(JsonRpcSuccessResponse {
                jsonrpc: JsonRpcVersion::V2_0,
                id: JsonRpcId::Number(2u64.into()),
                result: json!({}),
            }),
        )))
        .await;

        assert!(
            core.pending
                .lock()
                .await
                .contains_key(&JsonRpcIdKey::from_id(&JsonRpcId::Number(1u64.into())))
        );
    }

    #[tokio::test]
    async fn incoming_request_is_ignored() {
        let (event_tx, mut rx) = broadcast::channel(4);
        let core = SessionCore::new(event_tx);

        core.dispatch_message(JsonRpcMessage::Single(JsonRpcSingleMessage::Request(
            JsonRpcRequest {
                jsonrpc: JsonRpcVersion::V2_0,
                id: JsonRpcId::Number(1u64.into()),
                method: "peer".to_owned(),
                params: None,
            },
        )))
        .await;

        assert!(core.pending.lock().await.is_empty());
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn batch_messages_are_fanned_out() {
        let (event_tx, mut rx) = broadcast::channel(8);
        let core = SessionCore::new(event_tx);

        let response_rx = core
            .register_pending(&JsonRpcId::Number(3u64.into()))
            .await
            .expect("register pending");

        core.dispatch_message(JsonRpcMessage::Batch(JsonRpcBatch(vec![
            JsonRpcSingleMessage::Notification(JsonRpcNotification {
                jsonrpc: JsonRpcVersion::V2_0,
                method: "evt".to_owned(),
                params: None,
            }),
            JsonRpcSingleMessage::Response(JsonRpcResponse::Success(JsonRpcSuccessResponse {
                jsonrpc: JsonRpcVersion::V2_0,
                id: JsonRpcId::Number(3u64.into()),
                result: json!({"ok": true}),
            })),
        ])))
        .await;

        match rx.recv().await.unwrap() {
            JsonRpcSessionEvent::Notification(notification) => {
                assert_eq!(notification.method, "evt");
            }
            other => panic!("expected notification, got {other:?}"),
        }

        match response_rx.await.unwrap().unwrap() {
            JsonRpcResponse::Success(success) => {
                assert_eq!(success.id, JsonRpcId::Number(3u64.into()));
            }
            other => panic!("expected success, got {other:?}"),
        }
    }
}
