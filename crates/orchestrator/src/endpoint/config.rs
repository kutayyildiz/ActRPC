use actrpc_core::DescribeValue;
use actrpc_transport::TransportTarget;
use serde::{Deserialize, Deserializer, Serialize};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum JsonRpc2Mode {
    #[default]
    Auto,
    RequestResponse,
    Session,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct JsonRpc2ProtocolConfig {
    #[serde(default)]
    pub mode: JsonRpc2Mode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct RestHttpProtocolConfig {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolConfig {
    JsonRpc2(JsonRpc2ProtocolConfig),
    RestHttp(RestHttpProtocolConfig),
}

impl Default for ProtocolConfig {
    fn default() -> Self {
        Self::JsonRpc2(JsonRpc2ProtocolConfig::default())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EndpointConfig {
    pub name: EndpointName,
    pub transport: TransportTarget,
    pub protocol: ProtocolConfig,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum EndpointConfigRaw {
    Legacy {
        name: EndpointName,
        target: TransportTarget,
    },
    New {
        name: EndpointName,
        transport: TransportTarget,
        protocol: ProtocolConfig,
    },
}

impl<'de> Deserialize<'de> for EndpointConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match EndpointConfigRaw::deserialize(deserializer)? {
            EndpointConfigRaw::Legacy { name, target } => Ok(Self {
                name,
                transport: target,
                protocol: ProtocolConfig::JsonRpc2(JsonRpc2ProtocolConfig {
                    mode: JsonRpc2Mode::Auto,
                }),
            }),
            EndpointConfigRaw::New {
                name,
                transport,
                protocol,
            } => Ok(Self {
                name,
                transport,
                protocol,
            }),
        }
    }
}

impl EndpointConfig {
    pub fn legacy(name: EndpointName, target: TransportTarget) -> Self {
        Self {
            name,
            transport: target,
            protocol: ProtocolConfig::JsonRpc2(JsonRpc2ProtocolConfig {
                mode: JsonRpc2Mode::Auto,
            }),
        }
    }
}
