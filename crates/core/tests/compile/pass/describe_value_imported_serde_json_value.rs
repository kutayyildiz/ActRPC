use actrpc_core::DescribeValue;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DescribeValue)]
struct Payload {
    data: Value,
}

fn main() {}
