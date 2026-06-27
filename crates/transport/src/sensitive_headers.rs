use std::fmt;

const SENSITIVE_HEADER_NAMES: &[&str] = &[
    "authorization",
    "proxy-authorization",
    "cookie",
    "set-cookie",
    "x-api-key",
    "openai-api-key",
];

pub fn is_sensitive_header(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    SENSITIVE_HEADER_NAMES.contains(&lower.as_str())
}

/// Header pairs for HTTP/REST requests. Debug output redacts sensitive values.
#[derive(Clone, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct HeaderPairs(pub Vec<(String, String)>);

impl HeaderPairs {
    pub fn new(headers: Vec<(String, String)>) -> Self {
        Self(headers)
    }

    pub fn as_pairs(&self) -> &[(String, String)] {
        &self.0
    }

    pub fn iter(&self) -> impl Iterator<Item = &(String, String)> {
        self.0.iter()
    }
}

impl From<Vec<(String, String)>> for HeaderPairs {
    fn from(value: Vec<(String, String)>) -> Self {
        Self(value)
    }
}

impl fmt::Debug for HeaderPairs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.0.iter().map(|(name, value)| {
                if is_sensitive_header(name) {
                    format!("{name}: <redacted>")
                } else {
                    format!("{name}: {value}")
                }
            }))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::HeaderPairs;
    use crate::{HttpRestClient, target::HttpTarget};

    #[test]
    fn header_pairs_debug_redacts_sensitive_values() {
        let headers = HeaderPairs::new(vec![
            ("Authorization".to_owned(), "secret-token".to_owned()),
            ("X-Custom".to_owned(), "visible".to_owned()),
        ]);

        let debug = format!("{headers:?}");
        assert!(!debug.contains("secret-token"));
        assert!(debug.contains("<redacted>"));
        assert!(debug.contains("visible"));
    }

    #[test]
    fn http_rest_client_debug_redacts_target_headers() {
        let client = HttpRestClient::new(HttpTarget {
            url: "https://api.example.com".to_owned(),
            headers: HeaderPairs::new(vec![(
                "Authorization".to_owned(),
                "secret-token".to_owned(),
            )]),
            timeout_ms: 1000,
        })
        .unwrap();

        let debug = format!("{client:?}");
        assert!(!debug.contains("secret-token"));
        assert!(debug.contains("<redacted>"));
    }
}
