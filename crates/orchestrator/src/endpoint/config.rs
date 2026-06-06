use actrpc_core::DescribeValue;
use actrpc_transport::TransportTarget;
use serde::{Deserialize, Serialize};
use std::{borrow::Borrow, fmt, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, DescribeValue)]
#[serde(transparent)]
pub struct EndpointName {
    name: String,
}

impl EndpointName {
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

impl fmt::Display for EndpointName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.name.fmt(f)
    }
}

impl AsRef<str> for EndpointName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for EndpointName {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl From<String> for EndpointName {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for EndpointName {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl FromStr for EndpointName {
    type Err = core::convert::Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(value))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EndpointConfig {
    pub name: EndpointName,
    pub target: TransportTarget,
}
