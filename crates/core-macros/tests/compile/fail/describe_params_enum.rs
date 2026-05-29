use actrpc_core::DescribeParams;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DescribeParams)]
enum Params {
    A(String),
    B(i32),
}

fn main() {}
