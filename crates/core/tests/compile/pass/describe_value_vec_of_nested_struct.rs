use actrpc_core::DescribeValue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DescribeValue)]
struct Nested {
    enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DescribeValue)]
struct Holder {
    nested: Vec<Nested>,
}

fn main() {}
