use actrpc_core::DescribeValue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DescribeValue)]
enum Mixed {
    Valid(String),
    Invalid { value: String },
}

fn main() {}
