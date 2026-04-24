use actrpc_core::DescribeParams;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DescribeParams)]
struct Params {
    required_name: String,
    optional_count: Option<i32>,
    optional_flag: Option<bool>,
}

fn main() {}
