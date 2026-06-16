use actrpc_core::json_rpc::{
    JsonRpcError, JsonRpcId, JsonRpcMessage, JsonRpcRequest, JsonRpcResponse, JsonRpcSingleMessage,
    JsonRpcSuccessResponse, JsonRpcVersion,
};
use actrpc_interceptor::interceptors::dynamic_policy::{
    DynamicPolicyInterceptor, method_snapshot, new_component, provider::DynamicPolicyMethodProvider,
};
use actrpc_orchestrator::interceptor::Interceptor;
use serde::Serialize;
use std::{
    error::Error,
    io::{self, BufRead, Write},
};
use tokio::runtime::Runtime;

const INITIALIZE_METHOD: &str = actrpc_core::ACTRPC_INTERCEPTOR_INITIALIZE_METHOD;
const INTERCEPT_METHOD: &str = actrpc_core::ACTRPC_INTERCEPTOR_INTERCEPT_METHOD;
const PROVIDER_INITIALIZE_METHOD: &str = actrpc_core::ACTRPC_METHOD_PROVIDER_INITIALIZE_METHOD;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let component = new_component();
    serve_stdio(component.interceptor, component.provider)
}

fn serve_stdio(
    interceptor: DynamicPolicyInterceptor,
    provider: DynamicPolicyMethodProvider,
) -> Result<(), Box<dyn Error>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;

        if line.trim().is_empty() {
            continue;
        }

        let response = match handle_line(&runtime, &interceptor, &provider, &line) {
            Ok(Some(response)) => response,
            Ok(None) => continue,
            Err(error_response) => error_response,
        };

        serde_json::to_writer(&mut stdout, &response)?;
        stdout.write_all(b"\n")?;
        stdout.flush()?;
    }

    Ok(())
}

fn handle_line(
    runtime: &Runtime,
    interceptor: &DynamicPolicyInterceptor,
    provider: &DynamicPolicyMethodProvider,
    line: &str,
) -> Result<Option<JsonRpcMessage>, JsonRpcMessage> {
    let message = serde_json::from_str::<JsonRpcMessage>(line).map_err(|source| {
        json_rpc_error_message(
            JsonRpcId::Null,
            -32700,
            format!("failed to parse JSON-RPC message: {source}"),
        )
    })?;

    let JsonRpcMessage::Single(JsonRpcSingleMessage::Request(request)) = message else {
        return Err(json_rpc_error_message(
            JsonRpcId::Null,
            -32600,
            "dynamic policy component only accepts single JSON-RPC requests".to_owned(),
        ));
    };

    let response = match request.method.as_str() {
        INITIALIZE_METHOD => handle_initialize(runtime, interceptor, request),
        INTERCEPT_METHOD => handle_intercept(runtime, interceptor, request),
        PROVIDER_INITIALIZE_METHOD => handle_provider_initialize(request),
        "create_scope" | "release_scope" | "get_scope" | "list_scopes" => {
            handle_provider_method(provider, request)
        }
        method => Err(json_rpc_error_message(
            request.id,
            -32601,
            format!("unknown method: {method}"),
        )),
    }?;

    Ok(Some(response))
}

fn handle_initialize(
    runtime: &Runtime,
    interceptor: &DynamicPolicyInterceptor,
    request: JsonRpcRequest,
) -> Result<JsonRpcMessage, JsonRpcMessage> {
    let result = runtime
        .block_on(interceptor.initialize())
        .map_err(|source| {
            json_rpc_error_message(
                request.id.clone(),
                -32000,
                format!("initialize failed: {source}"),
            )
        })?;

    Ok(success_message(request.id, result))
}

fn handle_intercept(
    runtime: &Runtime,
    interceptor: &DynamicPolicyInterceptor,
    request: JsonRpcRequest,
) -> Result<JsonRpcMessage, JsonRpcMessage> {
    let params = request.params.ok_or_else(|| {
        json_rpc_error_message(
            request.id.clone(),
            -32602,
            "intercept request missing params".to_owned(),
        )
    })?;

    let value = serde_json::to_value(params).map_err(|source| {
        json_rpc_error_message(
            request.id.clone(),
            -32602,
            format!("failed to encode intercept params: {source}"),
        )
    })?;

    let interception_request = serde_json::from_value(value).map_err(|source| {
        json_rpc_error_message(
            request.id.clone(),
            -32602,
            format!("invalid intercept params: {source}"),
        )
    })?;

    let result = runtime
        .block_on(interceptor.intercept(&interception_request))
        .map_err(|source| {
            json_rpc_error_message(
                request.id.clone(),
                -32000,
                format!("intercept failed: {source}"),
            )
        })?;

    Ok(success_message(request.id, result))
}

fn handle_provider_initialize(request: JsonRpcRequest) -> Result<JsonRpcMessage, JsonRpcMessage> {
    Ok(success_message(request.id, method_snapshot()))
}

fn handle_provider_method(
    provider: &DynamicPolicyMethodProvider,
    request: JsonRpcRequest,
) -> Result<JsonRpcMessage, JsonRpcMessage> {
    let params = request
        .params
        .map(serde_json::to_value)
        .transpose()
        .map_err(|source| {
            json_rpc_error_message(
                request.id.clone(),
                -32602,
                format!("failed to encode provider params: {source}"),
            )
        })?;

    let result = provider
        .handle_request(&request.method, params)
        .map_err(|error| {
            json_rpc_error_message(request.id.clone(), error.json_rpc_code(), error.to_string())
        })?;

    Ok(success_message(request.id, result))
}

fn success_message<T>(id: JsonRpcId, result: T) -> JsonRpcMessage
where
    T: Serialize,
{
    JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Success(
        JsonRpcSuccessResponse {
            jsonrpc: JsonRpcVersion::V2_0,
            id,
            result: serde_json::to_value(result).expect("failed to serialize JSON-RPC result"),
        },
    )))
}

fn json_rpc_error_message(id: JsonRpcId, code: i32, message: String) -> JsonRpcMessage {
    JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Error(
        actrpc_core::json_rpc::JsonRpcErrorResponse {
            jsonrpc: JsonRpcVersion::V2_0,
            id,
            error: JsonRpcError {
                code,
                message,
                data: None,
            },
        },
    )))
}
