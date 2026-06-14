use crate::DescribeValue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, DescribeValue)]
pub struct MethodTarget {
    pub provider: String,
    pub method: String,
}
