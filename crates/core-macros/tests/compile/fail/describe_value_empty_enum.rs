// crates/core_macros/tests/ui/fail/describe_value_empty_enum.rs
use actrpc_core::DescribeValue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DescribeValue)]
enum Empty {}

fn main() {}
