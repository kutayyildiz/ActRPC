use std::collections::HashMap;

use actrpc_core::DescribeValue;
use serde::{Deserialize, Serialize};

type Labels = HashMap<String, String>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DescribeValue)]
struct Payload {
    // Intentionally rejected:
    // aliases hide the real container shape from the descriptor-facing API.
    labels: Labels,
}

fn main() {}
