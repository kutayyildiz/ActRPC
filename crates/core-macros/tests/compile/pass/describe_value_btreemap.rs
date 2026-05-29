use std::collections::BTreeMap;

use actrpc_core::DescribeValue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DescribeValue)]
struct Payload {
    labels: BTreeMap<String, i32>,
}

fn main() {}
