use actrpc_core::{
    DescribeValue,
    descriptor::{
        Accepts,
        types::{OkDescriptor, ParamsDescriptor},
    },
};
use serde::{Deserialize, Serialize};
use std::{borrow::Borrow, fmt, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, DescribeValue)]
#[serde(transparent)]
pub struct MethodName {
    name: String,
}

impl MethodName {
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

impl fmt::Display for MethodName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.name.fmt(f)
    }
}

impl AsRef<str> for MethodName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for MethodName {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl From<String> for MethodName {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for MethodName {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl FromStr for MethodName {
    type Err = std::convert::Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(value))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MethodDescriptor {
    pub name: MethodName,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<ParamsDescriptor>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ok: Option<OkDescriptor>,
}

impl Accepts for MethodDescriptor {
    fn accepts(&self, actual: &Self) -> bool {
        self.name == actual.name
            && self.params.accepts(&actual.params)
            && self.ok.accepts(&actual.ok)
    }
}
