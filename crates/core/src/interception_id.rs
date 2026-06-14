use crate::descriptor::traits::DescribeValue;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InterceptionId(pub Uuid);

impl InterceptionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for InterceptionId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for InterceptionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for InterceptionId {
    type Err = uuid::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(value)?))
    }
}

impl DescribeValue for InterceptionId {
    fn describe_value() -> crate::descriptor::types::ValueDescriptor {
        use crate::descriptor::types::{PrimitiveDescriptor, ValueDescriptor};

        ValueDescriptor::Primitive(PrimitiveDescriptor::String)
    }
}
