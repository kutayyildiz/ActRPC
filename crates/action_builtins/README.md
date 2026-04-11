# actrpc-action-builtins

`actrpc-action-builtins` provides ready-made ActRPC actions implemented for the canonical orchestrator.

These actions implement the action execution trait defined by `actrpc-orchestrator`.

## Purpose

This crate provides a default set of reusable actions without coupling them to either the protocol crate or the orchestrator runtime crate.

It exists to keep:

- protocol definitions in `actrpc-core`
- orchestration logic in `actrpc-orchestrator`
- concrete action implementations in a separate crate

## What It Contains

- built-in action specs
- built-in action executors
- registration helpers

Examples may include:

- message rewrite
- metadata injection
- target rerouting
- remote method calls
- short-circuit responses

## Design

- Small and curated
- Focused on control-plane behavior
- Implements orchestrator-defined action traits
- Pluggable into the canonical orchestrator
- Separate from protocol and runtime crates

## Usage (Conceptual)

```rust
let actions = ActionRegistry::new()
    .register(BuiltinModifyParams)
    .register(BuiltinCallRemote);

let orchestrator = Orchestrator::builder()
    .with_interceptor_registry(interceptors)
    .with_action_registry(actions)
    .build();
```

## Scope

This crate only provides action implementations.

It does not include:

- protocol definitions
- orchestrator logic
- interceptor logic
- transport implementations

## Summary

`actrpc-action-builtins` is the default action pack for ActRPC.

It provides ready-made actions that plug into the canonical orchestrator.
