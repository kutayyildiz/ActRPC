# Orchestrator runtime flow

1. Load orchestrator config (endpoints, methods, interceptors, pipelines).
2. Build `EndpointCatalog` from endpoint configs, method/interceptor configs, a JSON-RPC client provider, and a JSON-RPC session provider.
   - Per endpoint, connection mode is **Client** or **Session** (see README: Static/Initialize/Refreshable/MCP/interceptors → Client; Watchable → Session).
   - Session endpoints use one `JsonRpcSession` for both `request` and `subscribe`; HTTP targets are rejected for Session.
3. Build `MethodCatalog` and `InterceptorCatalog` from configs that reference `EndpointName` values, not raw transport targets.
4. Spawn watchable listener tasks via `spawn_watchable_listeners` after the method catalog is built (deduplicated by session endpoint name; handles stored on `OrchestratorResources`).
5. Build `OrchestratorResources` (interceptor catalog, method catalog, review provider, listener task handles) and a `CallExecutionFactory`.
6. On `call(provider, method, params)`:
   - Run the outbound interceptor pipeline and apply allowed actions.
   - Forward the call through the method provider (endpoint-backed JSON-RPC or MCP).
   - Run the inbound interceptor pipeline and apply response-side actions.
   - Return the final JSON-RPC message.

## Watchable method discovery

Watchable JSON-RPC providers:

- Require a **Session** endpoint (not HTTP).
- Initialize at build with `initialize_method` → `MethodProviderSnapshot`.
- Refresh on demand via `refresh_method` → full snapshot (same contract as Refreshable).
- Listen for `actrpc.method_provider.changed` on the endpoint session; valid notifications trigger `MethodCatalog::refresh_provider` after ownership and watchable checks. Malformed notifications are ignored; refresh failures keep the previous snapshot and the listener keeps running.

Notifications are triggers only—they do not carry trusted method metadata or apply diffs directly.

