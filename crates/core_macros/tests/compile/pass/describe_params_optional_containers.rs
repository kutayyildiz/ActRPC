use actrpc_core::DescribeParams;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DescribeParams)]
struct Params {
    tags: Option<Vec<String>>,
    metadata: Option<HashMap<String, i32>>,
}

fn main() {}
