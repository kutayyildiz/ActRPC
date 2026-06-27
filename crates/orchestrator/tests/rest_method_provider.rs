use actrpc_core::json_rpc::{
    JsonRpcId, JsonRpcMessage, JsonRpcParams, JsonRpcRequest, JsonRpcResponse,
    JsonRpcSingleMessage, JsonRpcVersion,
};
use actrpc_orchestrator::{
    EndpointEntry, EndpointName, RestHttpEndpoint,
    endpoint::RestHttpEndpointImpl,
    method::{
        MethodName, MethodProvider, ProviderName, RestMethodDefinition, RestMethodProvider,
        RestMethodSourceConfig, RestRequestMapping, RestResponseMapping,
    },
    test_catalog,
};

use actrpc_transport::{
    HeaderPairs, HttpRestClient, RestHttpExecuteRequest, RestHttpExecuteResponse, TransportError,
    TransportTarget,
};
use serde_json::json;
use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

struct MockRestEndpoint {
    last_body: std::sync::Mutex<Option<Vec<u8>>>,
    response_status: u16,
    response_body: Vec<u8>,
}

impl RestHttpEndpoint for MockRestEndpoint {
    fn endpoint_name(&self) -> &EndpointName {
        static NAME: std::sync::OnceLock<EndpointName> = std::sync::OnceLock::new();
        NAME.get_or_init(|| EndpointName::from("openai_api"))
    }

    fn endpoint_kind(&self) -> actrpc_orchestrator::EndpointKind {
        actrpc_orchestrator::EndpointKind::RestHttp
    }

    fn endpoint_capabilities(&self) -> actrpc_orchestrator::EndpointCapabilities {
        actrpc_orchestrator::EndpointCapabilities::REST_HTTP
    }

    fn execute<'a>(
        &'a self,
        request: RestHttpExecuteRequest,
    ) -> Pin<Box<dyn Future<Output = Result<RestHttpExecuteResponse, TransportError>> + Send + 'a>>
    {
        *self.last_body.lock().unwrap() = request.body.clone();
        let status = self.response_status;
        let body = self.response_body.clone();
        Box::pin(async move { Ok(RestHttpExecuteResponse { status, body }) })
    }
}

fn provider_with_mock(mock: Arc<MockRestEndpoint>) -> RestMethodProvider {
    let endpoint_name = EndpointName::from("openai_api");
    let catalog = test_catalog(HashMap::from([(
        endpoint_name.clone(),
        EndpointEntry::RestHttp {
            endpoint: mock as Arc<dyn RestHttpEndpoint>,
        },
    )]));

    RestMethodProvider::from_config(
        RestMethodSourceConfig {
            provider: ProviderName::from("openai"),
            endpoint: endpoint_name,
            methods: vec![RestMethodDefinition {
                name: MethodName::from("completions.create"),
                description: None,
                params_schema: None,
                result_schema: None,
                request: RestRequestMapping {
                    method: "POST".to_owned(),
                    path: "/v1/completions".to_owned(),
                    headers: HeaderPairs::default(),
                    body: Some("$params".to_owned()),
                },
                response: RestResponseMapping {
                    success_status: 200,
                    result: Some("$body".to_owned()),
                },
            }],
        },
        &catalog,
    )
    .unwrap()
}

#[tokio::test]
async fn maps_params_to_body_and_body_to_result() {
    let mock = Arc::new(MockRestEndpoint {
        last_body: std::sync::Mutex::new(None),
        response_status: 200,
        response_body: br#"{"id":"cmpl-1","choices":[]}"#.to_vec(),
    });
    let provider = provider_with_mock(mock.clone());

    let internal_id = JsonRpcId::String("req-1".to_owned());
    let request = JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
        jsonrpc: JsonRpcVersion::V2_0,
        id: internal_id.clone(),
        method: "completions.create".to_owned(),
        params: Some(JsonRpcParams::Object(
            serde_json::from_value(json!({"model":"gpt","prompt":"hi"})).unwrap(),
        )),
    }));

    let response = provider
        .send_message(&MethodName::from("completions.create"), request)
        .await
        .unwrap();

    let JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Success(success))) =
        response
    else {
        panic!("expected success");
    };

    assert_eq!(success.id, internal_id);
    assert_eq!(success.result["id"], "cmpl-1");

    let body = mock.last_body.lock().unwrap().clone().expect("body sent");
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(parsed["model"], "gpt");
}

#[tokio::test]
async fn empty_body_maps_to_null_result() {
    let mock = Arc::new(MockRestEndpoint {
        last_body: std::sync::Mutex::new(None),
        response_status: 200,
        response_body: vec![],
    });
    let provider = provider_with_mock(mock);

    let internal_id = JsonRpcId::Number(1.into());
    let request = JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
        jsonrpc: JsonRpcVersion::V2_0,
        id: internal_id.clone(),
        method: "completions.create".to_owned(),
        params: None,
    }));

    let response = provider
        .send_message(&MethodName::from("completions.create"), request)
        .await
        .unwrap();

    let JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Success(success))) =
        response
    else {
        panic!("expected success");
    };

    assert_eq!(success.result, serde_json::Value::Null);
}

#[tokio::test]
async fn non_success_http_status_maps_to_logical_error() {
    let mock = Arc::new(MockRestEndpoint {
        last_body: std::sync::Mutex::new(None),
        response_status: 500,
        response_body: b"error".to_vec(),
    });
    let provider = provider_with_mock(mock);

    let internal_id = JsonRpcId::String("req-err".to_owned());
    let request = JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
        jsonrpc: JsonRpcVersion::V2_0,
        id: internal_id.clone(),
        method: "completions.create".to_owned(),
        params: None,
    }));

    let response = provider
        .send_message(&MethodName::from("completions.create"), request)
        .await
        .unwrap();

    let JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Error(error))) =
        response
    else {
        panic!("expected error response");
    };

    assert_eq!(error.id, internal_id);
    assert!(error.error.message.contains("500"));
}

#[test]
fn rest_request_mapping_debug_redacts_authorization() {
    let mapping = RestRequestMapping {
        method: "POST".to_owned(),
        path: "/v1/completions".to_owned(),
        headers: HeaderPairs::new(vec![(
            "Authorization".to_owned(),
            "secret-token".to_owned(),
        )]),
        body: Some("$params".to_owned()),
    };

    let debug = format!("{mapping:?}");
    assert!(!debug.contains("secret-token"));
    assert!(debug.contains("<redacted>"));
}

#[test]
fn rest_http_endpoint_impl_debug_redacts_target_headers() {
    let endpoint = RestHttpEndpointImpl::new(
        EndpointName::from("openai_api"),
        TransportTarget::Http(actrpc_transport::target::HttpTarget {
            url: "https://api.example.com".to_owned(),
            headers: HeaderPairs::new(vec![(
                "Authorization".to_owned(),
                "secret-token".to_owned(),
            )]),
            timeout_ms: 1000,
        }),
        HttpRestClient::new(actrpc_transport::target::HttpTarget {
            url: "https://api.example.com".to_owned(),
            headers: HeaderPairs::new(vec![(
                "Authorization".to_owned(),
                "secret-token".to_owned(),
            )]),
            timeout_ms: 1000,
        })
        .unwrap(),
    );

    let debug = format!("{endpoint:?}");
    assert!(!debug.contains("secret-token"));
    assert!(debug.contains("<redacted>"));
}

fn invalid_provider_config(
    request: RestRequestMapping,
    response: RestResponseMapping,
) -> actrpc_orchestrator::error::MethodProviderBuildError {
    let mock = Arc::new(MockRestEndpoint {
        last_body: std::sync::Mutex::new(None),
        response_status: 200,
        response_body: vec![],
    });
    let endpoint_name = EndpointName::from("openai_api");
    let catalog = test_catalog(HashMap::from([(
        endpoint_name.clone(),
        EndpointEntry::RestHttp {
            endpoint: mock as Arc<dyn RestHttpEndpoint>,
        },
    )]));

    match RestMethodProvider::from_config(
        RestMethodSourceConfig {
            provider: ProviderName::from("openai"),
            endpoint: endpoint_name,
            methods: vec![RestMethodDefinition {
                name: MethodName::from("completions.create"),
                description: None,
                params_schema: None,
                result_schema: None,
                request,
                response,
            }],
        },
        &catalog,
    ) {
        Err(error) => error,
        Ok(_) => panic!("expected REST provider build to fail"),
    }
}

fn valid_request_mapping() -> RestRequestMapping {
    RestRequestMapping {
        method: "POST".to_owned(),
        path: "/v1/completions".to_owned(),
        headers: HeaderPairs::default(),
        body: Some("$params".to_owned()),
    }
}

fn valid_response_mapping() -> RestResponseMapping {
    RestResponseMapping {
        success_status: 200,
        result: Some("$body".to_owned()),
    }
}

#[test]
fn rejects_invalid_body_template_at_build_time() {
    let mut request = valid_request_mapping();
    request.body = Some("$foo".to_owned());
    let err = invalid_provider_config(request, valid_response_mapping());
    assert!(err.to_string().contains("$foo"));
}

#[test]
fn rejects_invalid_result_template_at_build_time() {
    let mut response = valid_response_mapping();
    response.result = Some("$foo".to_owned());
    let err = invalid_provider_config(valid_request_mapping(), response);
    assert!(err.to_string().contains("$foo"));
}

#[test]
fn rejects_full_url_path_at_build_time() {
    let mut request = valid_request_mapping();
    request.path = "https://evil.test/x".to_owned();
    let err = invalid_provider_config(request, valid_response_mapping());
    assert!(err.to_string().contains("full URL"));
}

#[test]
fn rejects_relative_path_at_build_time() {
    let mut request = valid_request_mapping();
    request.path = "v1/completions".to_owned();
    let err = invalid_provider_config(request, valid_response_mapping());
    assert!(err.to_string().contains("start with '/'"));
}

#[test]
fn rejects_invalid_http_method_at_build_time() {
    let mut request = valid_request_mapping();
    request.method = "NOT A METHOD!!!".to_owned();
    let err = invalid_provider_config(request, valid_response_mapping());
    assert!(err.to_string().contains("invalid HTTP method"));
}

#[test]
fn rejects_invalid_success_status_at_build_time() {
    let mut response = valid_response_mapping();
    response.success_status = 99;
    let err = invalid_provider_config(valid_request_mapping(), response);
    assert!(err.to_string().contains("success_status"));
}
