# actrpc-transport

`actrpc-transport` provides JSON-RPC client abstractions and concrete client implementations for ActRPC.

## What It Provides

- `JsonRpcClient` trait
- `JsonRpcClientFactory` trait
- `JsonRpcClientProvider` trait
- default client factory and provider
- transport target definitions
- concrete JSON-RPC clients
- stream framing helpers
- transport-specific error handling
- `JsonRpcSession` trait and `JsonRpcSessionProvider`
- concrete JSON-RPC sessions for stream-capable transports

## Supported Targets

`TransportTarget` supports:

- `StdioTarget`
- `TcpTarget`
- `LocalIpcTarget`
- `HttpTarget`
- `WebSocketTarget`

Concrete client types include:

- `StdioJsonRpcClient`
- `TcpJsonRpcClient`
- `LocalIpcJsonRpcClient`
- `HttpJsonRpcClient`
- `WebSocketJsonRpcClient`

## Sessions

`JsonRpcSession` is implemented for stdio, TCP, local IPC, and WebSocket.

HTTP remains request-response only and does not support sessions.

Concrete session types include:

- `StdioJsonRpcSession`
- `TcpJsonRpcSession`
- `LocalIpcJsonRpcSession`
- `WebSocketJsonRpcSession`

`DefaultJsonRpcSessionProvider` lazily creates and caches sessions per target. It also exposes `clear_cache` and `remove_cached_session`.

## Framing

For stream-oriented transports, the crate supports:

- content-length framing
- newline-delimited framing

The public `StreamFraming` type selects the framing mode, while the framing module handles serialization and deserialization of `JsonRpcMessage` values.

## Client Reuse

`DefaultJsonRpcClientProvider` can lazily create and cache clients per target through `DefaultJsonRpcClientFactory`. It also exposes cache-clearing and per-target removal helpers.
