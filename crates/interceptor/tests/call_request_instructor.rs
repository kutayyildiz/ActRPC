mod common;

use actrpc_core::{
    interception::{InterceptionRequest, InterceptorContinuation},
    json_rpc::{
        JsonRpcMessage, JsonRpcParams, JsonRpcRequest, JsonRpcSingleMessage, JsonRpcVersion,
    },
};
use actrpc_interceptor::interceptors::call_request::{
    CallRequestInstructor, InstructorConfig, PromptInjection, PromptInjectionRule,
};
use actrpc_orchestrator::interceptor::Interceptor;
use common::support::{default_target, external_origin};
use serde_json::json;

fn outbound_invoke(prompt: &str) -> InterceptionRequest {
    let mut params = serde_json::Map::new();
    params.insert("prompt".to_owned(), json!(prompt));

    InterceptionRequest {
        origin: external_origin("caller"),
        target: actrpc_core::MethodTarget {
            provider: "agents".to_owned(),
            method: "invoke".to_owned(),
        },
        message: JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
            jsonrpc: JsonRpcVersion::V2_0,
            id: actrpc_core::json_rpc::JsonRpcId::Number(1.into()),
            method: "invoke".to_owned(),
            params: Some(JsonRpcParams::Object(params)),
        })),
        call_id: actrpc_core::CallId::new(),
        interception_id: actrpc_core::InterceptionId::new(),
        resolved_action_history: vec![],
        ctx: Default::default(),
    }
}

fn instructor_config(injection: PromptInjection) -> InstructorConfig {
    InstructorConfig {
        version: 1,
        rules: vec![PromptInjectionRule {
            name: "agent_invoke".to_owned(),
            provider: "agents".to_owned(),
            method: "invoke".to_owned(),
            prompt_field: "prompt".to_owned(),
            injection,
        }],
    }
}

#[tokio::test]
async fn instructor_appends_configured_text() {
    let instructor = CallRequestInstructor::new(instructor_config(PromptInjection {
        append: Some("CUSTOM_INSTRUCTION".to_owned()),
        ..Default::default()
    }));
    let response = instructor
        .intercept(&outbound_invoke("hello"))
        .await
        .unwrap();

    assert_eq!(response.continuation, InterceptorContinuation::Stop);
    let prompt = response.actions[0].params.as_ref().unwrap()["params"]["prompt"]
        .as_str()
        .unwrap();
    assert_eq!(prompt, "hello\n\nCUSTOM_INSTRUCTION");
    assert!(!prompt.contains("_actrpc_call_requests"));
}

#[tokio::test]
async fn instructor_prepends_configured_text() {
    let instructor = CallRequestInstructor::new(instructor_config(PromptInjection {
        prepend: Some("PREFIX".to_owned()),
        ..Default::default()
    }));
    let response = instructor
        .intercept(&outbound_invoke("hello"))
        .await
        .unwrap();
    let prompt = response.actions[0].params.as_ref().unwrap()["params"]["prompt"]
        .as_str()
        .unwrap();
    assert_eq!(prompt, "PREFIX\n\nhello");
}

#[tokio::test]
async fn instructor_prepends_and_appends_in_order() {
    let instructor = CallRequestInstructor::new(instructor_config(PromptInjection {
        prepend: Some("PREFIX".to_owned()),
        append: Some("SUFFIX".to_owned()),
    }));
    let response = instructor
        .intercept(&outbound_invoke("middle"))
        .await
        .unwrap();
    let prompt = response.actions[0].params.as_ref().unwrap()["params"]["prompt"]
        .as_str()
        .unwrap();
    assert_eq!(prompt, "PREFIX\n\nmiddle\n\nSUFFIX");
}

#[tokio::test]
async fn instructor_noop_for_nonmatching_target() {
    let instructor = CallRequestInstructor::new(instructor_config(PromptInjection {
        append: Some("CUSTOM".to_owned()),
        ..Default::default()
    }));
    let mut request = outbound_invoke("hello");
    request.target = default_target("other");

    let response = instructor.intercept(&request).await.unwrap();
    assert!(response.actions.is_empty());
}

#[tokio::test]
async fn instructor_noop_for_missing_prompt_field() {
    let instructor = CallRequestInstructor::new(instructor_config(PromptInjection {
        append: Some("CUSTOM".to_owned()),
        ..Default::default()
    }));
    let mut request = outbound_invoke("hello");
    request.message = JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
        jsonrpc: JsonRpcVersion::V2_0,
        id: actrpc_core::json_rpc::JsonRpcId::Number(1.into()),
        method: "invoke".to_owned(),
        params: Some(JsonRpcParams::Object(serde_json::Map::new())),
    }));

    let response = instructor.intercept(&request).await.unwrap();
    assert!(response.actions.is_empty());
}

#[tokio::test]
async fn instructor_noop_for_non_string_prompt_field() {
    let instructor = CallRequestInstructor::new(instructor_config(PromptInjection {
        append: Some("CUSTOM".to_owned()),
        ..Default::default()
    }));
    let mut request = outbound_invoke("hello");
    if let JsonRpcMessage::Single(JsonRpcSingleMessage::Request(req)) = &mut request.message {
        if let Some(JsonRpcParams::Object(map)) = &mut req.params {
            map.insert("prompt".to_owned(), json!(123));
        }
    }

    let response = instructor.intercept(&request).await.unwrap();
    assert!(response.actions.is_empty());
}

#[test]
fn instructor_config_rejects_unknown_fields() {
    let text = r#"
version = 1

[[rules]]
name = "agent_invoke"
provider = "agents"
method = "invoke"
prompt_field = "prompt"
unknown = true

[rules.injection]
append = "x"
"#;
    let err = InstructorConfig::from_str_with_format(
        text,
        actrpc_interceptor::interceptors::call_request::config::CallRequestConfigFormat::Toml,
        "test.toml",
    )
    .unwrap_err();
    assert!(err.to_string().contains("unknown field"));
}

#[test]
fn instructor_config_rejects_unsupported_version() {
    let text = r#"
version = 2

[[rules]]
name = "agent_invoke"
provider = "agents"
method = "invoke"
prompt_field = "prompt"

[rules.injection]
append = "x"
"#;
    let err = InstructorConfig::from_str_with_format(
        text,
        actrpc_interceptor::interceptors::call_request::config::CallRequestConfigFormat::Toml,
        "test.toml",
    )
    .unwrap_err();
    assert!(
        err.to_string()
            .contains("unsupported instructor config version")
    );
}

#[test]
fn instructor_config_rejects_empty_injection() {
    let config = InstructorConfig {
        version: 1,
        rules: vec![PromptInjectionRule {
            name: "agent_invoke".to_owned(),
            provider: "agents".to_owned(),
            method: "invoke".to_owned(),
            prompt_field: "prompt".to_owned(),
            injection: PromptInjection::default(),
        }],
    };
    let err = config.validate().unwrap_err();
    assert!(err.to_string().contains("prepend or append"));
}

#[test]
fn instructor_config_rejects_duplicate_rule_names() {
    let config = InstructorConfig {
        version: 1,
        rules: vec![
            PromptInjectionRule {
                name: "dup".to_owned(),
                provider: "agents".to_owned(),
                method: "invoke".to_owned(),
                prompt_field: "prompt".to_owned(),
                injection: PromptInjection {
                    append: Some("a".to_owned()),
                    ..Default::default()
                },
            },
            PromptInjectionRule {
                name: "dup".to_owned(),
                provider: "agents".to_owned(),
                method: "other".to_owned(),
                prompt_field: "prompt".to_owned(),
                injection: PromptInjection {
                    append: Some("b".to_owned()),
                    ..Default::default()
                },
            },
        ],
    };
    let err = config.validate().unwrap_err();
    assert!(err.to_string().contains("duplicate rule name"));
}

#[tokio::test]
async fn instructor_does_not_inject_hardcoded_call_request_text() {
    let instructor = CallRequestInstructor::new(instructor_config(PromptInjection {
        append: Some("ONLY_CONFIG".to_owned()),
        ..Default::default()
    }));
    let response = instructor
        .intercept(&outbound_invoke("hello"))
        .await
        .unwrap();
    let prompt = response.actions[0].params.as_ref().unwrap()["params"]["prompt"]
        .as_str()
        .unwrap();
    assert!(!prompt.contains("_actrpc_call_requests"));
    assert!(!prompt.contains("dynamic_policy"));
}

