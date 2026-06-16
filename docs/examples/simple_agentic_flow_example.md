```bash
actrpc --config actrpc.yaml call app run --params '{"task":"build"}'
```

Example setup:

```text
Root method:
  provider = app
  method   = run

Outbound interceptor:
  name = planner_interceptor

Child method requested by interceptor:
  provider = tools
  method   = read_context
```

IDs/UUIDs below are simplified.

---

## 1. User starts CLI

```text 
user:
  actrpc --config actrpc.yaml call app run --params '{"task":"build"}'
```

```text 
cli:
  parses args
  loads OrchestratorConfig
  builds EndpointCatalog
  builds MethodCatalog
  builds InterceptorCatalog
  builds OrchestratorResources
  creates CallExecutionFactory
  creates DefaultOrchestrator
```

No JSON-RPC yet between CLI and orchestrator. That is an in-process Rust call.

---

## 2. CLI calls orchestrator

```text 
cli:
  orchestrator.call(
    prov
    method = "run",
    params = {"task":"build"}
  )
```

```text 
orchestrator:
  creates root CallExecution
  asks MethodCatalog to build root request message
```

Root method request message is internally represented as:

```json 
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "run",
  "params": {
    "task": "build"
  }
}
```

Important: `provider = app` is **not usually inside the JSON-RPC request**. The provider is selected by `MethodCatalog`. The remote JSON-RPC method only sees `"method": "run"`.

---

## 3. Orchestrator runs outbound pipeline

```text 
orchestrator:
  starts outbound PhaseRuntime
  takes first outbound interceptor: planner_interceptor
  sends InterceptionRequest to it
```

If `planner_interceptor` is remote, `JsonRpcBackedInterceptor` sends this JSON-RPC request:

```json 
{
  "jsonrpc": "2.0",
  "id": 100,
  "method": "actrpc.interceptor.intercept",
  "params": {
    "origin": {
      "kind": "external",
      "id": "cli"
    },
    "target": {
      "provider": "app",
      "method": "run"
    },
    "message": {
      "jsonrpc": "2.0",
      "id": 1,
      "method": "run",
      "params": {
        "task": "build"
      }
    },
    "call_id": "root-call-id",
    "interception_id": "root-outbound-interception-id"
  }
}
```

The interceptor receives:

```text 
planner_interceptor:
  sees user wants app.run
  decides it needs extra context first
  requests call_method(tools.read_context)
  asks to be reinvoked after action result exists
```

Interceptor response:

```json 
{
  "jsonrpc": "2.0",
  "id": 100,
  "result": {
    "actions": [
      {
        "kind": "call_method",
        "params": {
          "provider": "tools",
          "method": "read_context",
          "params": {
            "topic": "build"
          }
        }
      }
    ],
    "continuation": "reinvoke"
  }
}
```

---

## 4. Orchestrator executes `call_method`

```text 
orchestrator:
  validates planner_interceptor is allowed to request call_method
  finds call_method in ActionRegistry
  dispatches to CallMethodHandler
```

```text 
CallMethodHandler:
  creates child origin:
    kind = interceptor
    id = planner_interceptor

  calls:
    CallExecutionFactory::run_piped(
      provider = tools,
      method = read_context,
      params = {"topic":"build"},
      parent_call = root call
    )
```

This creates a **child call**.

---

## 5. Child call is created

```text 
orchestrator:
  creates child CallExecution
  parent_call_id = root-call-id
  child call_id = child-call-id
  root_call_id = root-call-id
```

Child method JSON-RPC request:

```json 
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "read_context",
  "params": {
    "topic": "build"
  }
}
```

---

## 6. Child call runs its own outbound pipeline

```text 
orchestrator:
  runs child outbound pipeline
  sends child InterceptionRequest to interceptors
```

Example child outbound interception request:

```json 
{
  "jsonrpc": "2.0",
  "id": 101,
  "method": "actrpc.interceptor.intercept",
  "params": {
    "origin": {
      "kind": "interceptor",
      "id": "planner_interceptor"
    },
    "target": {
      "provider": "tools",
      "method": "read_context"
    },
    "message": {
      "jsonrpc": "2.0",
      "id": 1,
      "method": "read_context",
      "params": {
        "topic": "build"
      }
    },
    "call_id": "child-call-id",
    "interception_id": "child-outbound-interception-id"
  }
}
```

Assume no interceptor blocks it:

```json 
{
  "jsonrpc": "2.0",
  "id": 101,
  "result": {
    "continuation": "stop"
  }
}
```

---

## 7. Child call goes through MethodCatalog → MethodProvider → Endpoint

```text 
orchestrator:
  child outbound pipeline finished
  sends child method request through MethodCatalog
```

```text 
MethodCatalog:
  finds provider = tools
  finds MethodProvider for tools
```

```text 
MethodProvider:
  sends JsonRpcMessage through JsonRpcEndpoint
```

```text 
JsonRpcEndpoint:
  uses configured TransportTarget
  maybe stdio, tcp, http, websocket, ipc, etc.
```

Raw JSON-RPC sent to the actual `tools` method provider:

```json 
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "read_context",
  "params": {
    "topic": "build"
  }
}
```

Remote provider responds:

```json 
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "context": "Use release profile and run integration tests."
  }
}
```

---

## 8. Child inbound pipeline runs

```text 
orchestrator:
  stores child response as in-flight message
  runs child inbound pipeline
```

Child inbound interception request:

```json 
{
  "jsonrpc": "2.0",
  "id": 102,
  "method": "actrpc.interceptor.intercept",
  "params": {
    "origin": {
      "kind": "interceptor",
      "id": "planner_interceptor"
    },
    "target": {
      "provider": "tools",
      "method": "read_context"
    },
    "message": {
      "jsonrpc": "2.0",
      "id": 1,
      "result": {
        "context": "Use release profile and run integration tests."
      }
    },
    "call_id": "child-call-id",
    "interception_id": "child-inbound-interception-id"
  }
}
```

Assume no inbound changes:

```json 
{
  "jsonrpc": "2.0",
  "id": 102,
  "result": {
    "continuation": "stop"
  }
}
```

Child call finishes with:

```json 
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "context": "Use release profile and run integration tests."
  }
}
```

---

## 9. `call_method` action becomes resolved action

```text 
CallMethodHandler:
  decodes child JsonRpcResponse::Success
  returns ResolvedAction<CallMethod>
```

Orchestrator stores it as `ResolvedActionRecord`:

```json 
{
  "kind": "call_method",
  "params": {
    "provider": "tools",
    "method": "read_context",
    "params": {
      "topic": "build"
    }
  },
  "result": {
    "Ok": {
      "context": "Use release profile and run integration tests."
    }
  }
}
```

Then:

```text 
orchestrator:
  pushes this into round_actions
  pushes round_actions into resolved_action_history
```

So root outbound history now contains:

```json 
[
  [
    {
      "kind": "call_method",
      "params": {
        "provider": "tools",
        "method": "read_context",
        "params": {
          "topic": "build"
        }
      },
      "result": {
        "Ok": {
          "context": "Use release profile and run integration tests."
        }
      }
    }
  ]
]
```

---

## 10. Orchestrator reinvokes original interceptor

Because original interceptor returned:

```json 
{
  "continuation": "reinvoke"
}
```

orchestrator calls the same interceptor again.

Second root outbound interception request:

```json 
{
  "jsonrpc": "2.0",
  "id": 103,
  "method": "actrpc.interceptor.intercept",
  "params": {
    "origin": {
      "kind": "external",
      "id": "cli"
    },
    "target": {
      "provider": "app",
      "method": "run"
    },
    "message": {
      "jsonrpc": "2.0",
      "id": 1,
      "method": "run",
      "params": {
        "task": "build"
      }
    },
    "call_id": "root-call-id",
    "interception_id": "root-outbound-interception-id",
    "resolved_action_history": [
      [
        {
          "kind": "call_method",
          "params": {
            "provider": "tools",
            "method": "read_context",
            "params": {
              "topic": "build"
            }
          },
          "result": {
            "Ok": {
              "context": "Use release profile and run integration tests."
            }
          }
        }
      ]
    ]
  }
}
```

Now the interceptor can read the child result.

```text 
planner_interceptor:
  reads resolved_action_history
  sees call_method result
  decides to modify root params
```

It returns:

```json 
{
  "jsonrpc": "2.0",
  "id": 103,
  "result": {
    "actions": [
      {
        "kind": "modify_params",
        "params": {
          "params": {
            "task": "build",
            "context": "Use release profile and run integration tests."
          }
        }
      }
    ],
    "continuation": "stop"
  }
}
```

---

## 11. Orchestrator executes `modify_params`

```text 
orchestrator:
  executes modify_params
  changes root in-flight JsonRpcRequest params
```

Root request becomes:

```json 
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "run",
  "params": {
    "task": "build",
    "context": "Use release profile and run integration tests."
  }
}
```

Because continuation is `stop`, orchestrator does **not** reinvoke that interceptor again.

---

## 12. Root outbound pipeline finishes

```text 
orchestrator:
  outbound pipeline finished
  root call was not rejected
  sends root method request through MethodCatalog
```

Raw JSON-RPC sent to root provider `app`:

```json 
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "run",
  "params": {
    "task": "build",
    "context": "Use release profile and run integration tests."
  }
}
```

Remote app provider responds:

```json 
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "status": "done",
    "used_context": true
  }
}
```

---

## 13. Root inbound pipeline runs

```text 
orchestrator:
  stores root provider response as in-flight message
  runs inbound pipeline
```

Inbound interception request:

```json 
{
  "jsonrpc": "2.0",
  "id": 104,
  "method": "actrpc.interceptor.intercept",
  "params": {
    "origin": {
      "kind": "external",
      "id": "cli"
    },
    "target": {
      "provider": "app",
      "method": "run"
    },
    "message": {
      "jsonrpc": "2.0",
      "id": 1,
      "result": {
        "status": "done",
        "used_context": true
      }
    },
    "call_id": "root-call-id",
    "interception_id": "root-inbound-interception-id"
  }
}
```

Assume inbound interceptor does nothing:

```json 
{
  "jsonrpc": "2.0",
  "id": 104,
  "result": {
    "continuation": "stop"
  }
}
```

---

## 14. Orchestrator returns final response to CLI

```text 
orchestrator:
  final in-flight message is root JsonRpcResponse
  returns it to CLI
```

Final message:

```json 
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "status": "done",
    "used_context": true
  }
}
```

---

## 15. CLI prints final JSON-RPC response

```text 
cli:
  serde_json::to_string_pretty(final_message)
  prints to stdout
```

User sees:

```json 
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "status": "done",
    "used_context": true
  }
}
```

---

## The whole flow in one compact version

```text 
user:
  actrpc --config actrpc.yaml call app run --params '{"task":"build"}'

cli:
  initializes config/resources/orchestrator
  calls orchestrator.call(app, run, params)

orchestrator:
  creates root CallExecution
  builds root JSON-RPC request: app.run
  runs outbound pipeline

orchestrator -> planner_interceptor:
  InterceptionRequest for app.run

planner_interceptor -> orchestrator:
  InterceptionResponse:
    action = call_method(tools.read_context)
    continuation = reinvoke

orchestrator:
  executes call_method
  creates child CallExecution

child call:
  runs outbound pipeline
  sends JSON-RPC request to tools.read_context
  receives JSON-RPC response
  runs inbound pipeline
  returns child result

orchestrator:
  stores child result as ResolvedActionRecord
  appends it to resolved_action_history
  reinvokes planner_interceptor

orchestrator -> planner_interceptor:
  InterceptionRequest for app.run
  includes resolved_action_history with call_method result

planner_interceptor -> orchestrator:
  InterceptionResponse:
    action = modify_params(add context)
    continuation = stop

orchestrator:
  executes modify_params
  root request params are updated
  outbound pipeline finishes
  sends root JSON-RPC request to app.run

app provider -> orchestrator:
  JSON-RPC success response

orchestrator:
  runs inbound pipeline
  returns final JsonRpcMessage to CLI

cli:
  prints JSON-RPC response

user:
  sees final result
```

The crucial mechanism is this:

```text 
call_method action result does not return directly to the interceptor as a function return.

It returns through:

ResolvedActionRecord
  → resolved_action_history
  → next InterceptionRequest after reinvoke
```

So an interceptor that wants to use a `call_method` result should usually return:

```json 
{
  "actions": [
    {
      "kind": "call_method",
      "params": {
        "provider": "tools",
        "method": "read_context",
        "params": {}
      }
    }
  ],
  "continuation": "reinvoke"
}
```

Then on the next invocation, it reads:

```json 
{
  "resolved_action_history": [
    [
      {
        "kind": "call_method",
        "result": {
          "Ok": {
            "some": "child result"
          }
        }
      }
    ]
  ]
}
```

