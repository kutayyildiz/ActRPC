# actrpc-orchestrator

`actrpc-orchestrator` is the ActRPC runtime engine.

It coordinates interceptor execution, action handling, method calls, transport-backed destinations, working pipeline state, and transcript state.

## What It Provides

- the `Orchestrator` trait
- `DefaultOrchestrator`
- call execution and nested call execution factories
- outbound and inbound phase runtimes
- interceptor catalog and working pipeline models
- action registry and typed action-handler traits
- built-in action registry construction
- method catalog support
- transcript capture
- orchestrator-specific errors

## Runtime Flow

1. Build or provide a `MethodCatalog` for callable downstream methods.
2. Build an `InterceptorCatalog` from configured interceptor targets and policies.
3. Build an `ActionRegistry`, usually with built-in action handlers.
4. Create `OrchestratorResources` and a `CallExecutionFactory`.
5. Use `DefaultOrchestrator` to call a method with optional JSON-RPC params.
6. The runtime runs outbound interceptors, applies actions, forwards the call, runs inbound interceptors, applies response-side actions, and returns the resulting JSON-RPC message.

## Built-in Actions

The `action::actions` module contains built-in action specs and handlers for:

- `call_method`
- `exclude_interceptors`
- `get_interceptor_catalog`
- `get_working_interceptor_catalog`
- `get_working_pipeline`
- `get_transcript`
- `modify_params`
- `modify_result`
- `modify_error`
- `reject_call`
- `request_review`

The `action::build_builtin_action_registry` helper builds a registry from the runtime resources used by those handlers.

## Interceptors

The orchestrator invokes interceptors through the `interceptor::Interceptor` trait. It also includes `JsonRpcBackedInterceptor`, which lets an interceptor be backed by a JSON-RPC client from `actrpc-transport`.

Interceptor catalog entries include names, transport targets, phase policy, and advertised capabilities.
