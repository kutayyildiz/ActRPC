use actrpc_core_macros::DescribeValue;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, DescribeValue)]
#[serde(transparent)]
pub struct ActionKind {
    name: String,
}

impl ActionKind {
    pub fn new(value: impl Into<String>) -> Self {
        Self { name: value.into() }
    }

    pub fn as_str(&self) -> &str {
        &self.name
    }

    pub fn into_string(self) -> String {
        self.name
    }
}

impl fmt::Display for ActionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.name.fmt(f)
    }
}

impl AsRef<str> for ActionKind {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for ActionKind {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl From<String> for ActionKind {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for ActionKind {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<ActionKind> for String {
    fn from(value: ActionKind) -> Self {
        value.name
    }
}

impl From<&ActionKind> for String {
    fn from(value: &ActionKind) -> Self {
        value.as_str().to_owned()
    }
}

impl FromStr for ActionKind {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(s))
    }
}
