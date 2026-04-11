use actrpc_core::json_rpc::JsonRpcMessage;

pub trait JsonRpcClient: Send + Sync {
    type Error;

    fn send(&self, message: JsonRpcMessage) -> Result<JsonRpcMessage, Self::Error>;
}
