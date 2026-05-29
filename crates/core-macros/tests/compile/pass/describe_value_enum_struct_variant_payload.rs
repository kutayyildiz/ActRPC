use actrpc_core::DescribeValue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DescribeValue)]
struct ObjPayload {
    enabled: bool,
    label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DescribeValue)]
enum Wrapper {
    Object(ObjPayload),
    Count(i64),
}

fn main() {}
