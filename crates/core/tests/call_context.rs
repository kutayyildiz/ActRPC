use actrpc_core::{CallContext, InterceptionContext};
use serde_json::json;
use std::collections::BTreeMap;

#[test]
fn call_context_serde_roundtrip() {
    let mut interceptors = BTreeMap::new();
    interceptors.insert(
        "dynamic_policy".to_owned(),
        json!({
            "mode": "detached",
            "allowed_method_targets": [
                { "provider": "filesystem", "method": "read_file" }
            ]
        }),
    );

    let ctx = CallContext {
        shared: Some(json!({ "trace_id": "abc" })),
        interceptors,
    };

    let encoded = serde_json::to_string(&ctx).unwrap();
    let decoded: CallContext = serde_json::from_str(&encoded).unwrap();
    assert_eq!(ctx, decoded);
}

#[test]
fn interception_context_serde_roundtrip() {
    let ctx = InterceptionContext {
        shared: Some(json!({ "trace_id": "abc" })),
        private: Some(json!({ "mode": "detached" })),
    };

    let encoded = serde_json::to_string(&ctx).unwrap();
    let decoded: InterceptionContext = serde_json::from_str(&encoded).unwrap();
    assert_eq!(ctx, decoded);
}

#[test]
fn interception_context_is_empty() {
    assert!(InterceptionContext::default().is_empty());
    assert!(!InterceptionContext {
        private: Some(json!(1)),
        ..Default::default()
    }
    .is_empty());
}

#[test]
fn call_context_filter_for_interceptor() {
    let mut interceptors = BTreeMap::new();
    interceptors.insert("dynamic_policy".to_owned(), json!({ "mode": "detached" }));
    interceptors.insert("other".to_owned(), json!({ "secret": true }));

    let ctx = CallContext {
        shared: Some(json!("shared")),
        interceptors,
    };

    let filtered = ctx.filter_for_interceptor("dynamic_policy");
    assert_eq!(filtered.shared, Some(json!("shared")));
    assert_eq!(filtered.private, Some(json!({ "mode": "detached" })));
    assert!(ctx.filter_for_interceptor("missing").private.is_none());
}

#[test]
fn call_context_rejects_unknown_fields() {
    let err = serde_json::from_str::<CallContext>(r#"{"unknown": true}"#).unwrap_err();
    assert!(err.to_string().contains("unknown field"));
}

#[test]
fn interception_context_rejects_unknown_fields() {
    let err = serde_json::from_str::<InterceptionContext>(r#"{"unknown": true}"#).unwrap_err();
    assert!(err.to_string().contains("unknown field"));
}