use actrpc_core::DescribeOk;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DescribeOk)]
struct OkPayload {
    maybe_name: Option<String>,
}

fn main() {}
