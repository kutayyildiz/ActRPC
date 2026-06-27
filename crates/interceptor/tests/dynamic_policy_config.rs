use actrpc_core::MethodTarget;
use actrpc_interceptor::interceptors::dynamic_policy::{DynamicPolicyConfig, UnscopedBehavior};

#[test]
fn default_config_uses_allow_unscoped() {
    let config = DynamicPolicyConfig::default();
    assert_eq!(config.unscoped_policy.on_unscoped, UnscopedBehavior::Allow);
    assert!(config.unscoped_policy.allowed_method_targets.is_empty());
}

#[test]
fn scope_unscoped_requires_non_empty_allowlist() {
    let text = r#"
[unscoped_policy]
on_unscoped = "scope"
"#;
    let err = DynamicPolicyConfig::from_str_with_format(
        text,
        actrpc_interceptor::interceptors::dynamic_policy::config::DynamicPolicyConfigFormat::Toml,
        "test.toml",
    )
    .unwrap_err();
    assert!(err.to_string().contains("allowed_method_targets"));
}

#[test]
fn scope_unscoped_with_allowlist_loads() {
    let text = r#"
[unscoped_policy]
on_unscoped = "scope"

[[unscoped_policy.allowed_method_targets]]
provider = "agents"
method = "invoke"
"#;
    let config = DynamicPolicyConfig::from_str_with_format(
        text,
        actrpc_interceptor::interceptors::dynamic_policy::config::DynamicPolicyConfigFormat::Toml,
        "test.toml",
    )
    .unwrap();
    assert_eq!(
        config.unscoped_policy.on_unscoped,
        UnscopedBehavior::ScopeRoot
    );
    assert_eq!(
        config.unscoped_policy.allowed_method_targets,
        vec![MethodTarget {
            provider: "agents".to_owned(),
            method: "invoke".to_owned(),
        }]
    );
}

#[test]
fn invalid_on_unscoped_is_rejected() {
    let text = r#"
[unscoped_policy]
on_unscoped = "invalid"
"#;
    let err = toml::from_str::<DynamicPolicyConfig>(text).unwrap_err();
    assert!(!err.to_string().is_empty());
}
