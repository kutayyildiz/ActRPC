use actrpc_core::json_rpc::JsonRpcParams;

pub(crate) fn parse_params(raw: Option<String>) -> Result<Option<JsonRpcParams>, String> {
    let Some(raw) = raw else {
        return Ok(None);
    };

    let value: serde_json::Value =
        serde_json::from_str(&raw).map_err(|source| format!("invalid JSON params: {source}"))?;

    match value {
        serde_json::Value::Array(items) => Ok(Some(JsonRpcParams::Array(items))),
        serde_json::Value::Object(map) => Ok(Some(JsonRpcParams::Object(map))),
        other => Err(format!(
            "params must be a JSON array or object, got {other}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actrpc_core::json_rpc::JsonRpcParams;
    use serde_json::json;

    #[test]
    fn parse_params_returns_none_for_missing_params() {
        let parsed = parse_params(None).unwrap();

        assert_eq!(parsed, None);
    }

    #[test]
    fn parse_params_accepts_json_array() {
        let parsed = parse_params(Some(r#"[1,2,"x"]"#.to_owned())).unwrap();

        assert_eq!(
            parsed,
            Some(JsonRpcParams::Array(vec![json!(1), json!(2), json!("x")]))
        );
    }

    #[test]
    fn parse_params_accepts_json_object() {
        let parsed = parse_params(Some(r#"{"path":"test.txt","force":true}"#.to_owned())).unwrap();

        match parsed {
            Some(JsonRpcParams::Object(map)) => {
                assert_eq!(map.get("path"), Some(&json!("test.txt")));
                assert_eq!(map.get("force"), Some(&json!(true)));
            }
            other => panic!("unexpected params: {other:?}"),
        }
    }

    #[test]
    fn parse_params_rejects_json_string() {
        let err = parse_params(Some(r#""hello""#.to_owned())).unwrap_err();

        assert!(err.contains("params must be a JSON array or object"));
    }

    #[test]
    fn parse_params_rejects_invalid_json() {
        let err = parse_params(Some(r#"{"#.to_owned())).unwrap_err();

        assert!(err.contains("invalid JSON params"));
    }
}
