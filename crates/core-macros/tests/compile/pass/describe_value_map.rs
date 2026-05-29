use std::collections::{BTreeMap, HashMap};

use actrpc_core::DescribeValue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DescribeValue)]
struct Payload {
    tags: HashMap<String, String>,
    metadata: BTreeMap<String, i32>,
}

fn main() {}
