use actrpc_interceptor::interceptors::policy::{
    PolicyError, PolicyInterceptor, config::PolicyConfig,
};
use std::path::{Path, PathBuf};

#[test]
fn loads_simple_review_example() {
    let config = PolicyConfig::from_path(example_path("simple_review.yaml")).unwrap();

    assert_eq!(config.rules.len(), 1);
    assert_eq!(config.rules[0].name, "review_sensitive_write");

    PolicyInterceptor::new(config).unwrap();
}

#[test]
fn loads_block_sensitive_write_example() {
    let config = PolicyConfig::from_path(example_path("block_sensitive_write.yaml")).unwrap();

    assert_eq!(config.rules.len(), 1);
    assert_eq!(config.rules[0].name, "block_sensitive_system_writes");

    PolicyInterceptor::new(config).unwrap();
}

#[test]
fn loads_exclude_loggers_example() {
    let config = PolicyConfig::from_path(example_path("exclude_loggers.yaml")).unwrap();

    assert_eq!(config.rules.len(), 1);
    assert_eq!(config.rules[0].name, "hide_loggers_for_secret_reads");

    PolicyInterceptor::new(config).unwrap();
}

#[test]
fn rejects_unsupported_config_extension() {
    let err = PolicyConfig::from_path(Path::new("policy.json")).unwrap_err();

    assert!(matches!(err, PolicyError::UnsupportedConfigFormat { .. }));
}

fn example_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("interceptors")
        .join("policy")
        .join("config")
        .join("examples")
        .join(name)
}
