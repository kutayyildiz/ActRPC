# actrpc-orchestrator

`actrpc-orchestrator` is the ActRPC runtime engine.

It coordinates interceptor execution, action handling, method calls, endpoint-backed destinations, working pipeline state, and transcript state.

## What It Provides

- the `Orchestrator` trait
- `DefaultOrchestrator`
- call execution and nested call execution factories
- outbound and inbound phase runtimes
- interceptor catalog and working pipeline models
- action registry and typed action-handler traits
- built-in action registry construction
- method catalog support (endpoint-backed JSON-RPC and MCP providers)
- `EndpointCatalog` with endpoint-backed interceptors and methods
- `EndpointConnection::Client` and `EndpointConnection::Session` (persistent `JsonRpcSession` for notifications and request/response on the same connection)
- watchable JSON-RPC method discovery (`actrpc.method_provider.changed` notifications trigger full snapshot refresh)
- transcript capture
- orchestrator-specific errors

## Endpoint connections

`EndpointCatalog` inspects method and interceptor configs only to decide **Client** vs **Session** per endpoint:

| Role | Connection |
|------|------------|
| JSON-RPC Static / Initialize / Refreshable | Client |
| JSON-RPC Watchable | Session |
| MCP | Client |
| Interceptor | Client |

If any role on an endpoint requires Session, that endpoint is built as Session. Request/response calls use `JsonRpcEndpoint::request` on that same session (no separate client for the same endpoint).

HTTP targets cannot be used for Session endpoints (including Watchable providers); build fails with a clear error.

## Runtime Flow

1. Build `EndpointCatalog` from endpoint configs, method/interceptor configs, a JSON-RPC client provider, and a JSON-RPC session provider.
2. Build `MethodCatalog` and `InterceptorCatalog` from configs that reference `EndpointName` values.
3. Spawn watchable listener tasks (one per session endpoint used by Watchable providers); retain their `JoinHandle`s on `OrchestratorResources`.
4. Build an `ActionRegistry`, usually with built-in action handlers.
5. Create `OrchestratorResources` and a `CallExecutionFactory`.
6. Use `DefaultOrchestrator` to call a method with optional JSON-RPC params.
7. The runtime runs outbound interceptors, applies actions, forwards the call, runs inbound interceptors, applies response-side actions, and returns the resulting JSON-RPC message.

### Watchable method providers

Watchable discovery initializes via `initialize_method`, stores `refresh_method`, and keeps the method list in a provider snapshot. When the remote sends `actrpc.method_provider.changed` over the endpoint session, a listener parses the notification, validates provider/endpoint ownership, and calls `MethodCatalog::refresh_provider` to pull a full replacement snapshot (no patch/diff updates from the notification itself).

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

The orchestrator invokes interceptors through the `interceptor::Interceptor` trait. `JsonRpcBackedInterceptor` is endpoint-backed: it sends JSON-RPC requests through `JsonRpcEndpoint::request`.

Interceptor catalog entries include names, endpoint references, phase policy, and advertised capabilities.