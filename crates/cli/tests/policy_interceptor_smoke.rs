use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn cli_call_is_rejected_by_stdio_policy_interceptor() {
    let workspace = workspace_root();
    let temp = TestDir::new("actrpc-cli-policy-interceptor-smoke");

    let policy_path = temp.path().join("policy-deny-all.yaml");
    let actrpc_config_path = temp.path().join("actrpc.yaml");

    fs::write(&policy_path, deny_all_policy_config()).unwrap();
    fs::write(
        &actrpc_config_path,
        actrpc_config(&policy_path, &cargo_program()),
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_actrpc"))
        .current_dir(&workspace)
        .arg("--config")
        .arg(&actrpc_config_path)
        .arg("call")
        .arg("demo")
        .arg("anything")
        .arg("--params")
        .arg(r#"{"value":1}"#)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "actrpc exited with status {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    let response: Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["error"]["code"], -32001);
    assert_eq!(
        response["error"]["message"],
        "denied by policy interceptor smoke test"
    );
}

fn deny_all_policy_config() -> &'static str {
    r#"
rules:
  - name: deny_all_outbound
    match_expr:
      condition:
        fact: phase
        matcher:
          kind: exact
          value: outbound
    apply:
      immediate:
        - reject_call:
            error:
              code: -32001
              message: denied by policy interceptor smoke test
"#
}

fn actrpc_config(policy_path: &Path, cargo: &str) -> String {
    let policy_path = yaml_string(&policy_path.display().to_string());
    let cargo = yaml_string(cargo);

    format!(
        r#"
endpoints:
  - name: policy_interceptor
    target:
      stdio:
        program: {cargo}
        args:
          - run
          - -q
          - -p
          - actrpc-interceptor
          - --bin
          - actrpc_policy_interceptor
          - --
          - --config
          - {policy_path}
        env: []
        framing: newline_delimited

  - name: unreachable_demo_method
    target:
      http:
        url: http://127.0.0.1:9/rpc
        headers: []
        timeout_ms: 1000

methods:
  - json_rpc:
      provider: demo
      endpoint: unreachable_demo_method
      discovery:
        static:
          methods:
            - name: anything
              description: Dummy method; outbound policy rejects before this endpoint is called.
              info: {{}}

interceptors:
  - name: deny_all_policy
    endpoint: policy_interceptor
    policy:
      outbound:
        - reject_call
      inbound:
        - reject_call

pipelines:
  outbound:
    - deny_all_policy
  inbound:
    - deny_all_policy

runtime:
  max_call_depth: 8
  max_interception_reinvokes: 8
  interception_request_timeout_ms: 120000
  max_actions_per_interception: 64
"#
    )
}

fn cargo_program() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned())
}

fn yaml_string(value: &str) -> String {
    serde_json::to_string(value).unwrap()
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/cli should be two levels below workspace root")
        .to_path_buf()
}

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(prefix: &str) -> Self {
        let unique = format!(
            "{}-{}-{}",
            prefix,
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );

        let path = std::env::temp_dir().join(unique);
        fs::create_dir_all(&path).unwrap();

        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
