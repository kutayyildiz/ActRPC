use crate::interceptors::call_request::{
    config::ExecutorConfig,
    error::CallRequestError,
    schema::{
        CallRequest, canonical_executed_call_request_key,
        executed_call_request_from_call_method_params, executed_call_request_from_call_request,
        params_to_json_rpc, parse_target,
    },
};
use actrpc_core::{
    InterceptorInitialization,
    action::{
        ActionDescriptor, ActionKind, ActionSpec, RequestedAction, RequestedActionRecord,
        ResolvedActionRecord,
    },
    interception::{
        InterceptionPhase, InterceptionRequest, InterceptionResponse, InterceptorContinuation,
    },
    json_rpc::{JsonRpcMessage, JsonRpcResponse, JsonRpcSingleMessage},
};
use actrpc_orchestrator::{
    action::actions::{
        call_method::{CallMethod, CallMethodParams},
        modify_result::{ModifyResult, ModifyResultParams},
    },
    interceptor::{Interceptor, InterceptorFuture},
};
use serde_json::{Value, json};
use std::collections::{HashMap, VecDeque};

pub struct CallRequestExecutor {
    config: ExecutorConfig,
}

impl CallRequestExecutor {
    pub fn new(config: ExecutorConfig) -> Self {
        Self { config }
    }

    fn intercept_inner(
        &self,
        request: &InterceptionRequest,
    ) -> Result<InterceptionResponse, CallRequestError> {
        if request.phase()? != InterceptionPhase::Inbound {
            return Ok(no_op());
        }

        let JsonRpcMessage::Single(JsonRpcSingleMessage::Response(response)) = &request.message
        else {
            return Ok(no_op());
        };

        let actrpc_core::json_rpc::JsonRpcResponse::Success(success) = response else {
            return Ok(no_op());
        };

        let result_value = &success.result;
        let field = &self.config.call_requests_field;

        let Some(field_value) = result_value.get(field) else {
            return Ok(no_op());
        };

        if field_value.is_null() {
            return Ok(no_op());
        }

        let call_requests = match parse_call_requests(field_value) {
            Ok(requests) => requests,
            Err(error) => {
                return Ok(error_modify_result(
                    result_value,
                    &self.config.results_field,
                    error,
                )?);
            }
        };

        if call_requests.is_empty() {
            return Ok(no_op());
        }

        let prior_call_method_records = collect_call_method_records(request);

        if prior_call_method_records.is_empty() {
            let actions = call_requests
                .iter()
                .map(build_call_method_action)
                .collect::<Result<Vec<_>, _>>()?;

            return Ok(InterceptionResponse {
                actions,
                continuation: InterceptorContinuation::Reinvoke,
            });
        }

        let results = build_call_results(&call_requests, &prior_call_method_records)?;
        let updated_result = merge_results(result_value, &self.config.results_field, results);

        Ok(InterceptionResponse {
            actions: vec![modify_result_action(updated_result)?],
            continuation: InterceptorContinuation::Stop,
        })
    }
}

impl Interceptor for CallRequestExecutor {
    fn initialize<'a>(
        &'a self,
    ) -> InterceptorFuture<
        'a,
        Result<InterceptorInitialization, actrpc_orchestrator::error::InterceptorRuntimeError>,
    >
    where
        Self: 'a,
    {
        Box::pin(async move {
            Ok(InterceptorInitialization {
                supports_outbound: false,
                supports_inbound: true,
                actions: action_descriptors(),
            })
        })
    }

    fn intercept<'a>(
        &'a self,
        request: &'a InterceptionRequest,
    ) -> InterceptorFuture<
        'a,
        Result<InterceptionResponse, actrpc_orchestrator::error::InterceptorRuntimeError>,
    >
    where
        Self: 'a,
    {
        Box::pin(async move {
            self.intercept_inner(request)
                .map_err(actrpc_orchestrator::error::InterceptorRuntimeError::from)
        })
    }
}

fn no_op() -> InterceptionResponse {
    InterceptionResponse {
        actions: vec![],
        continuation: InterceptorContinuation::Stop,
    }
}

fn parse_call_requests(value: &Value) -> Result<Vec<CallRequest>, String> {
    serde_json::from_value(value.clone()).map_err(|source| source.to_string())
}

fn collect_call_method_records<'a>(
    request: &'a InterceptionRequest,
) -> Vec<&'a ResolvedActionRecord> {
    let mut records = Vec::new();

    for round in request.iter_resolved_action_rounds() {
        for action in round {
            if action.kind == CallMethod::action_kind() {
                records.push(action);
            }
        }
    }

    records
}

fn build_call_results(
    call_requests: &[CallRequest],
    records: &[&ResolvedActionRecord],
) -> Result<Vec<Value>, CallRequestError> {
    let mut record_queues: HashMap<String, VecDeque<&ResolvedActionRecord>> = HashMap::new();

    for record in records {
        let Some(params) = &record.params else {
            continue;
        };
        let Ok(executed) = executed_call_request_from_call_method_params(params) else {
            continue;
        };
        let key = canonical_executed_call_request_key(&executed)?;
        record_queues.entry(key).or_default().push_back(record);
    }

    call_requests
        .iter()
        .map(|call_request| {
            let executed = executed_call_request_from_call_request(call_request);
            let key = canonical_executed_call_request_key(&executed)?;
            let request = serde_json::to_value(&executed).map_err(|source| {
                CallRequestError::InvalidCallRequest {
                    message: format!("failed to serialize executed call request: {source}"),
                }
            })?;

            match record_queues.get_mut(&key).and_then(VecDeque::pop_front) {
                Some(record) => record_to_result_entry(&request, record),
                None => Ok(json!({
                    "request": request,
                    "error": "missing CallMethod result",
                })),
            }
        })
        .collect()
}

fn record_to_result_entry(
    request: &Value,
    record: &ResolvedActionRecord,
) -> Result<Value, CallRequestError> {
    match &record.result {
        Ok(Some(value)) => {
            let response: JsonRpcResponse =
                serde_json::from_value(value.clone()).map_err(|source| {
                    CallRequestError::InvalidCallRequest {
                        message: format!("CallMethod result is not a JSON-RPC response: {source}"),
                    }
                })?;

            let response_value = serde_json::to_value(response).map_err(|source| {
                CallRequestError::InvalidCallRequest {
                    message: format!("failed to serialize JSON-RPC response: {source}"),
                }
            })?;

            Ok(json!({
                "request": request,
                "response": response_value,
            }))
        }
        Ok(None) => Ok(json!({
            "request": request,
            "error": "missing CallMethod result",
        })),
        Err(protocol_error) => Ok(json!({
            "request": request,
            "error": protocol_error.to_string(),
        })),
    }
}

fn merge_results(original: &Value, results_field: &str, results: Vec<Value>) -> Value {
    if let Value::Object(map) = original {
        let mut updated = map.clone();
        updated.insert(results_field.to_owned(), Value::Array(results));
        Value::Object(updated)
    } else {
        let mut map = serde_json::Map::new();
        map.insert("value".to_owned(), original.clone());
        map.insert(results_field.to_owned(), Value::Array(results));
        Value::Object(map)
    }
}

fn error_modify_result(
    original: &Value,
    results_field: &str,
    message: String,
) -> Result<InterceptionResponse, CallRequestError> {
    let updated = merge_results(original, results_field, vec![json!({ "error": message })]);

    Ok(InterceptionResponse {
        actions: vec![modify_result_action(updated)?],
        continuation: InterceptorContinuation::Stop,
    })
}

fn build_call_method_action(
    call_request: &CallRequest,
) -> Result<RequestedActionRecord, CallRequestError> {
    let (provider, method) = parse_target(&call_request.target)?;
    let params = params_to_json_rpc(call_request.params.clone())?;

    RequestedAction::<CallMethod> {
        params: CallMethodParams {
            provider,
            method,
            params,
            ctx: call_request.ctx.clone(),
        },
    }
    .try_into()
    .map_err(|source| CallRequestError::ActionEncoding { source })
}

fn modify_result_action(result: Value) -> Result<RequestedActionRecord, CallRequestError> {
    RequestedAction::<ModifyResult> {
        params: ModifyResultParams { result },
    }
    .try_into()
    .map_err(|source| CallRequestError::ActionEncoding { source })
}

fn action_descriptors() -> HashMap<ActionKind, ActionDescriptor> {
    actrpc_core::action::action_descriptor_map!(CallMethod, ModifyResult)
}
