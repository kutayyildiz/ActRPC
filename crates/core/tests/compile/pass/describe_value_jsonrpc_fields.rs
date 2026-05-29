use actrpc_core::{
    DescribeValue,
    json_rpc::{JsonRpcId, JsonRpcParams},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DescribeValue)]
struct Payload {
    id: JsonRpcId,
    params: JsonRpcParams,
}

fn main() {}
