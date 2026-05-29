use std::collections::HashMap;

use actrpc_core::DescribeValue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DescribeValue)]
struct Payload {
    bad: HashMap<i32, String>,
}

fn main() {}
