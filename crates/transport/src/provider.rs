use crate::{client::JsonRpcClient, target::TransportTarget};

pub trait JsonRpcClientProvider: Send + Sync {
    type Error;
    type Client: JsonRpcClient<Error = Self::Error>;

    fn get_client(&self, target: &TransportTarget) -> Result<Self::Client, Self::Error>;
}
