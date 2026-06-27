use crate::TransportError;
use reqwest::Method;
use std::str::FromStr;

pub fn validate_http_method(method: &str) -> Result<(), String> {
    Method::from_str(method)
        .map(|_| ())
        .map_err(|_| format!("invalid HTTP method '{method}'"))
}

pub fn validate_rest_path(path: &str) -> Result<(), String> {
    if path.starts_with("http://") || path.starts_with("https://") {
        return Err("REST path must not be a full URL".to_owned());
    }

    if !path.starts_with('/') {
        return Err("REST path must start with '/'".to_owned());
    }

    Ok(())
}

pub fn join_base_url_and_path(base_url: &str, path: &str) -> Result<String, TransportError> {
    if base_url.contains('?') {
        return Err(TransportError::Internal {
            message: "REST base_url must not contain a query string".to_owned(),
        });
    }

    validate_rest_path(path).map_err(|message| TransportError::Internal { message })?;

    let base = base_url.trim_end_matches('/');
    Ok(format!("{base}{path}"))
}

#[cfg(test)]
mod tests {
    use super::join_base_url_and_path;

    #[test]
    fn joins_base_and_path() {
        let url = join_base_url_and_path("https://api.openai.com", "/v1/completions").unwrap();
        assert_eq!(url, "https://api.openai.com/v1/completions");
    }

    #[test]
    fn normalizes_duplicate_slashes() {
        let url = join_base_url_and_path("https://api.openai.com/", "/v1/completions").unwrap();
        assert_eq!(url, "https://api.openai.com/v1/completions");
    }

    #[test]
    fn rejects_full_url_path() {
        join_base_url_and_path("https://api.openai.com", "https://evil.test/x").unwrap_err();
    }

    #[test]
    fn rejects_query_in_base_url() {
        join_base_url_and_path("https://api.openai.com?k=v", "/v1").unwrap_err();
    }
}
