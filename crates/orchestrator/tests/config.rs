use actrpc_orchestrator::{
    config::{ConfigFormat, OrchestratorConfig},
    error::ConfigError,
    method::{MethodName, ProviderName},
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn config_from_yaml_path_loads_orchestrator_config() {
    let dir = temp_test_dir("config_from_yaml_path_loads_orchestrator_config");
    let path = dir.join("actrpc.yaml");

    fs::write(
        &path,
        r#"
methods:
  - kind: native
    name: math
    description: Math provider
    target:
      http:
        url: "http://example.invalid/rpc"
        headers: []
        timeout_ms: 1000
    methods:
      - name: sum
        remote_method: sum_remote
        description: Adds numbers

interceptors:
  - name: firewall
    policy:
      outbound:
        - reject_call
        - request_review
      inbound: []
    target:
      http:
        url: "http://example.invalid/firewall"
        headers: []
        timeout_ms: 1000

pipelines:
  outbound:
    - firewall
  inbound: []
"#,
    )
    .unwrap();

    let config = OrchestratorConfig::from_path(&path).unwrap();

    assert_eq!(config.methods.len(), 1);
    assert_eq!(config.methods[0].name(), &ProviderName::from("math"));

    match &config.methods[0] {
        actrpc_orchestrator::method::MethodSourceConfig::Native(provider) => {
            assert_eq!(provider.name, ProviderName::from("math"));
            assert_eq!(provider.description.as_deref(), Some("Math provider"));
            assert_eq!(provider.methods.len(), 1);
            assert_eq!(provider.methods[0].name, MethodName::from("sum"));
            assert_eq!(provider.methods[0].remote_method, "sum_remote");
            assert_eq!(
                provider.methods[0].description.as_deref(),
                Some("Adds numbers")
            );
        }
        other => panic!("unexpected method source config: {other:?}"),
    }

    assert_eq!(config.interceptors.len(), 1);
    assert_eq!(config.interceptors[0].name, "firewall");

    assert_eq!(config.pipelines.outbound, vec!["firewall"]);
    assert!(config.pipelines.inbound.is_empty());
}

#[test]
fn config_from_toml_path_loads_orchestrator_config() {
    let dir = temp_test_dir("config_from_toml_path_loads_orchestrator_config");
    let path = dir.join("actrpc.toml");

    fs::write(
        &path,
        r#"
[[methods]]
kind = "native"
name = "math"
description = "Math provider"

[methods.target.http]
url = "http://example.invalid/rpc"
headers = []
timeout_ms = 1000

[[methods.methods]]
name = "sum"
remote_method = "sum_remote"
description = "Adds numbers"

[[interceptors]]
name = "firewall"

[interceptors.policy]
outbound = ["reject_call", "request_review"]
inbound = []

[interceptors.target.http]
url = "http://example.invalid/firewall"
headers = []
timeout_ms = 1000

[pipelines]
outbound = ["firewall"]
inbound = []
"#,
    )
    .unwrap();

    let config = OrchestratorConfig::from_path(&path).unwrap();

    assert_eq!(config.methods.len(), 1);
    assert_eq!(config.methods[0].name(), &ProviderName::from("math"));

    match &config.methods[0] {
        actrpc_orchestrator::method::MethodSourceConfig::Native(provider) => {
            assert_eq!(provider.name, ProviderName::from("math"));
            assert_eq!(provider.description.as_deref(), Some("Math provider"));
            assert_eq!(provider.methods.len(), 1);
            assert_eq!(provider.methods[0].name, MethodName::from("sum"));
            assert_eq!(provider.methods[0].remote_method, "sum_remote");
            assert_eq!(
                provider.methods[0].description.as_deref(),
                Some("Adds numbers")
            );
        }
        other => panic!("unexpected method source config: {other:?}"),
    }

    assert_eq!(config.interceptors.len(), 1);
    assert_eq!(config.interceptors[0].name, "firewall");

    assert_eq!(config.pipelines.outbound, vec!["firewall"]);
    assert!(config.pipelines.inbound.is_empty());
}

#[test]
fn config_from_paths_appends_files_in_order() {
    let dir = temp_test_dir("config_from_paths_appends_files_in_order");

    let first = dir.join("first.yaml");
    let second = dir.join("second.yaml");

    fs::write(
        &first,
        r#"
methods:
  - kind: native
    name: math
    target:
      http:
        url: "http://example.invalid/sum"
        headers: []
        timeout_ms: 1000
    methods:
      - name: sum
        remote_method: sum_remote

interceptors:
  - name: firewall
    policy:
      outbound:
        - reject_call
      inbound: []
    target:
      http:
        url: "http://example.invalid/firewall"
        headers: []
        timeout_ms: 1000

pipelines:
  outbound:
    - firewall
  inbound: []
"#,
    )
    .unwrap();

    fs::write(
        &second,
        r#"
methods:
  - kind: native
    name: filesystem
    target:
      http:
        url: "http://example.invalid/get"
        headers: []
        timeout_ms: 1000
    methods:
      - name: get
        remote_method: get_remote

interceptors:
  - name: audit_logger
    policy:
      outbound: []
      inbound: []
    target:
      http:
        url: "http://example.invalid/audit"
        headers: []
        timeout_ms: 1000

pipelines:
  outbound:
    - audit_logger
  inbound:
    - audit_logger
"#,
    )
    .unwrap();

    let config = OrchestratorConfig::from_paths([&first, &second]).unwrap();

    assert_eq!(config.methods.len(), 2);
    assert_eq!(config.methods[0].name(), &ProviderName::from("math"));
    assert_eq!(config.methods[1].name(), &ProviderName::from("filesystem"));

    match &config.methods[0] {
        actrpc_orchestrator::method::MethodSourceConfig::Native(provider) => {
            assert_eq!(provider.methods[0].name, MethodName::from("sum"));
            assert_eq!(provider.methods[0].remote_method, "sum_remote");
        }
        other => panic!("unexpected method source config: {other:?}"),
    }

    match &config.methods[1] {
        actrpc_orchestrator::method::MethodSourceConfig::Native(provider) => {
            assert_eq!(provider.methods[0].name, MethodName::from("get"));
            assert_eq!(provider.methods[0].remote_method, "get_remote");
        }
        other => panic!("unexpected method source config: {other:?}"),
    }

    assert_eq!(config.interceptors.len(), 2);
    assert_eq!(config.interceptors[0].name, "firewall");
    assert_eq!(config.interceptors[1].name, "audit_logger");

    assert_eq!(config.pipelines.outbound, vec!["firewall", "audit_logger"]);
    assert_eq!(config.pipelines.inbound, vec!["audit_logger"]);
}

#[test]
fn config_from_paths_rejects_duplicate_method_provider_names() {
    let dir = temp_test_dir("config_from_paths_rejects_duplicate_method_provider_names");

    let first = dir.join("first.yaml");
    let second = dir.join("second.yaml");

    fs::write(
        &first,
        r#"
methods:
  - kind: native
    name: math
    target:
      http:
        url: "http://example.invalid/one"
        headers: []
        timeout_ms: 1000
    methods:
      - name: sum
        remote_method: sum_one
"#,
    )
    .unwrap();

    fs::write(
        &second,
        r#"
methods:
  - kind: native
    name: math
    target:
      http:
        url: "http://example.invalid/two"
        headers: []
        timeout_ms: 1000
    methods:
      - name: sum
        remote_method: sum_two
"#,
    )
    .unwrap();

    let err = OrchestratorConfig::from_paths([&first, &second]).unwrap_err();

    match err {
        ConfigError::DuplicateMethodProvider { name } => {
            assert_eq!(name, ProviderName::from("math"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn config_from_paths_rejects_duplicate_interceptor_names() {
    let dir = temp_test_dir("config_from_paths_rejects_duplicate_interceptor_names");

    let first = dir.join("first.yaml");
    let second = dir.join("second.yaml");

    fs::write(
        &first,
        r#"
interceptors:
  - name: firewall
    policy:
      outbound: []
      inbound: []
    target:
      http:
        url: "http://example.invalid/one"
        headers: []
        timeout_ms: 1000
"#,
    )
    .unwrap();

    fs::write(
        &second,
        r#"
interceptors:
  - name: firewall
    policy:
      outbound: []
      inbound: []
    target:
      http:
        url: "http://example.invalid/two"
        headers: []
        timeout_ms: 1000
"#,
    )
    .unwrap();

    let err = OrchestratorConfig::from_paths([&first, &second]).unwrap_err();

    match err {
        ConfigError::DuplicateInterceptor { name } => {
            assert_eq!(name, "firewall");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn config_from_paths_rejects_empty_path_list() {
    let paths: [&Path; 0] = [];

    let err = OrchestratorConfig::from_paths(paths).unwrap_err();

    assert!(matches!(err, ConfigError::NoConfigPaths));
}

#[test]
fn config_from_path_rejects_unsupported_extension() {
    let dir = temp_test_dir("config_from_path_rejects_unsupported_extension");
    let path = dir.join("actrpc.json");

    fs::write(&path, "{}").unwrap();

    let err = OrchestratorConfig::from_path(&path).unwrap_err();

    match err {
        ConfigError::UnsupportedFormat { path: err_path } => {
            assert_eq!(err_path, path);
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn config_from_str_with_format_loads_yaml_without_path_file() {
    let config = OrchestratorConfig::from_str_with_format(
        r#"
interceptors:
  - name: firewall
    policy:
      outbound:
        - reject_call
      inbound: []
    target:
      http:
        url: "http://example.invalid/firewall"
        headers: []
        timeout_ms: 1000

pipelines:
  outbound:
    - firewall
  inbound: []
"#,
        ConfigFormat::Yaml,
        "<test>",
    )
    .unwrap();

    assert_eq!(config.interceptors.len(), 1);
    assert_eq!(config.interceptors[0].name, "firewall");
    assert_eq!(config.pipelines.outbound, vec!["firewall"]);
}

#[test]
fn config_from_str_with_format_loads_stdio_target() {
    let config = OrchestratorConfig::from_str_with_format(
        r#"
interceptors:
  - name: firewall
    policy:
      outbound:
        - reject_call
        - request_review
      inbound: []
    target:
      stdio:
        program: actrpc-firewall
        args:
          - --config
          - firewall.yaml
        env: []

pipelines:
  outbound:
    - firewall
  inbound: []
"#,
        ConfigFormat::Yaml,
        "<test>",
    )
    .unwrap();

    assert_eq!(config.interceptors.len(), 1);
    assert_eq!(config.interceptors[0].name, "firewall");
    assert_eq!(config.pipelines.outbound, vec!["firewall"]);
}

fn temp_test_dir(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    path.push(format!("actrpc_{name}_{unique}"));

    fs::create_dir_all(&path).unwrap();

    path
}
