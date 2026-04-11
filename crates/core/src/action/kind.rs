use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActionKind(String);

impl ActionKind {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for ActionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
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
        Self(value)
    }
}

impl From<&str> for ActionKind {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<ActionKind> for String {
    fn from(value: ActionKind) -> Self {
        value.0
    }
}

impl From<&ActionKind> for String {
    fn from(value: &ActionKind) -> Self {
        value.0.clone()
    }
}

impl FromStr for ActionKind {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(s))
    }
}
