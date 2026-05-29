# actrpc-core

`actrpc-core` is the protocol foundation of ActRPC.

It defines the shared data model used by the orchestrator, transports, interceptors, and action implementations. It does not execute calls, open connections, or run interceptor pipelines.

## What It Provides

- JSON-RPC 2.0 message types
- interception request and response types
- outbound/inbound interception phases
- action specifications, requested actions, and resolved action records
- action descriptors and value descriptors
- participant identity types
- shared protocol, codec, and action-codec errors
- descriptor derive macro re-exports from `actrpc-core-macros`

## Descriptor Derives

This crate re-exports:

```rust
use actrpc_core::{DescribeOk, DescribeParams, DescribeValue};
```

Use these derives to describe action params, action outputs, and nested value structures in a form that can be advertised to interceptors and validated by the orchestrator.
