# Canonical Actions

## Actions

### modify_params

**Action**

```json
{
  "action": "modify_params",
  "action_params": "<object | array>"
}
```

### modify_result

**Action**

```json
{
  "action": "modify_result",
  "action_params": "<any>"
}
```

### modify_error

**Action**

```json
{
  "action": "modify_error",
  "action_params": "<JSON-RPC 2.0 Error Object>"
}
```

### call_external_method

**Action**

```json
{
  "action": "call_external_method",
  "action_params": "<JSON-RPC 2.0 Request Object>"
}
```

**Result**

```json
{
  "result": "<JSON-RPC 2.0 Result Object>"
}
```

### list_external_methods

**Action**

```json
{
  "action": "list_external_methods"
}
```

**Result**

```json
{
  "result": "<string[]>"
}
```

### get_interceptor_order

**Action**

```json
{
  "action": "get_interceptor_order"
}
```

**Result**

```json
{
  "result": "<string[]>"
}
```

### get_interceptor_state

**Action**

```json
{
  "action": "get_interceptor_state"
}
```

**Result**

```json
{
  "result": "<interceptor_state_object[]>"
}
```

### get_interceptor_policy

**Action**

```json
{
  "action": "get_interceptor_policy"
}
```

**Result**

```json
{
  "result": "<interceptor_policy_object[]>"
}
```

### enable_interceptors

**Action**

```json
{
  "action": "enable_interceptors",
  "action_params": "<string[]>"
}
```

### disable_interceptors

**Action**

```json
{
  "action": "disable_interceptors",
  "action_params": "<string[]>"
}
```

### get_transcript

**Action**

```json
{
  "action": "get_transcript"
}
```

**Result**

```json
{
  "result": "<transcript_object[]>"
}
```

#### Orchestrator -> Interceptor

```json
{
  "from": { "type": "orchestrator", "id": "<string>" },
  "to": { "type": "interceptor", "id": "<string>" },
  "seq": "<int>",
  "ts": "<float>",
  "message": {
    "jsonrpc": "2.0",
    "id": "<string | int | null>",
    "method": "intercept",
    "params": {
      "origin": "<string>",
      "message": "<JSON-RPC 2.0>", // Phase(Inbound) -> Response | Phase(Outbound) -> Request
      "executed_actions?": [
        { "<action_name>": "<string>", "<action_params?>": "<object>" }
      ]
    }
  }
}
```

#### Interceptor -> Orchestrator

- If actions does not exist or final does not exist then final assumed to be true.

```json
{
  "from": { "type": "interceptor", "id": "<string>" },
  "to": { "type": "orchestrator", "id": "<string>" },
  "seq": "<int>",
  "ts": "<float>",
  "message": {
    "jsonrpc": "2.0",
    "id": "<string | int | null>",
    "result": { "actions?": "<action_object[]>", "is_final": "<boolean>" }
  }
}
```

#### Orchestrator -> User

```json
{
  "from": { "type": "orchestrator", "id": "<string>" },
  "to": { "type": "user", "id": "<string>" },
  "seq": "<int>",
  "ts": "<float>",
  "message": "<JSON-RPC 2.0 Response>"
}
```

#### User -> Orchestrator

```json
{
  "from": { "type": "user", "id": "<string>" },
  "to": { "type": "orchestrator", "id": "<string>" },
  "seq": "<int>",
  "ts": "<float>",
  "message": "<JSON-RPC 2.0 Request>"
}
```

## Custom Objects

### action_object

```json
{
  "action": "<string>",
  "action_params?": "<object>"
}
```

### interceptor_policy_object

```json
{
  "interceptor_name": "<string>",
  "allowed_actions": "<string[]>"
}
```

### interceptor_state_object

```json
{
  "interceptor_name": "<string>",
  "enabled": "<boolean>",
  "executed": "<boolean>"
}
```

### transcript_object

```json
{
  "from": { "type": "user | interceptor | orchestrator", "id": "<string>" },
  "to": { "type": "user | interceptor | orchestrator", "id": "<string>" },
  "seq": "<int>",
  "ts": "<float>",
  "message": "<JSON-RPC 2.0>"
}
```

## JSON-RPC 2.0 Object

- ? = optional

### Request Object (non-notification - expects response)

```json
{
  "jsonrpc": "2.0",
  "id": "<int | string | null>",
  "method": "<string>",
  "params?": "<object | array>"
}
```

### Request Object (notification - does not expect response)

```json
{
  "jsonrpc": "2.0",
  "method": "<string>",
  "params?": "<object | array>"
}
```

### Result Object

```json
{
  "jsonrpc": "2.0",
  "id": "<int | string | null>",
  "result": "<any>"
}
```

### Error Object

```json
{
  "jsonrpc": "2.0",
  "id": "<int | string | null>",
  "error": { "code": "<int>", "message": "<string>", "data?": "<any>" }
}
```
