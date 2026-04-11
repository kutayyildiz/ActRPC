# actrpc-core

`actrpc-core` defines the shared protocol model for ActRPC.

It contains the canonical types used by orchestrators, interceptors, transports, and action implementations.

## Purpose

This crate defines:

- JSON-RPC 2.0 message types
- interception request and response types
- action specifications and action records
- participant types
- shared errors
- protocol conversions

## What It Contains

- `JsonRpcMessage` and related JSON-RPC types
- `InterceptionRequest`
- `InterceptionResponse`
- `InterceptionPhase`
- `ActionSpec`
- `RequestedActionRecord`
- `ResolvedActionRecord`
- protocol error types

## Design

- Protocol-first
- Runtime-agnostic
- Shared by all other ActRPC crates
- Focused on structure, not execution

## Scope

This crate does not include:

- orchestrator runtime logic
- interceptor runtime logic
- transport implementations
- built-in actions

## Summary

`actrpc-core` is the protocol foundation of ActRPC.

It defines the types and rules that all other crates build on.
