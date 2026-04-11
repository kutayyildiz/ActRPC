# actrpc-transport

`actrpc-transport` provides the transport-side abstractions used to deliver ActRPC and JSON-RPC messages.

It defines how orchestrators and interceptors communicate over concrete transport mechanisms.

## Purpose

This crate exists to separate message delivery from protocol structure and runtime behavior.

It provides the transport-side contracts needed for:

- interceptor invocation
- downstream endpoint forwarding
- transport-specific client reuse
- lazy initialization and connection management

## What It Contains

- transport target definitions
- JSON-RPC client traits
- client provider traits
- concrete transport implementations
- transport error types

## Design

- Built on `actrpc-core`
- Focused on delivery, not protocol
- Supports lazy initialization and caching per target
- Works with heterogeneous transport definitions

## Transport Model

A transport implementation is responsible for turning a transport target into a usable JSON-RPC client.

A provider may:

- lazily initialize a client
- cache clients per target
- reconnect failed clients
- create fresh clients if needed

The orchestrator should not care which strategy is used.

## Scope

This crate does not include:

- protocol definitions
- orchestrator pipeline logic
- interceptor logic
- built-in actions

## Summary

`actrpc-transport` defines how ActRPC participants are contacted.

It handles delivery and client resolution while leaving protocol and orchestration logic to other crates.
