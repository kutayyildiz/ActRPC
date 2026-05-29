use actrpc_core::DescribeValue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DescribeValue)]
enum VariantValue {
    Text(String),
    Count(i32),
    Flags(Vec<bool>),
}

fn main() {}
