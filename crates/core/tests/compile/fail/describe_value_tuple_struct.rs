use actrpc_core::DescribeValue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DescribeValue)]
struct Payload(String);

fn main() {}
