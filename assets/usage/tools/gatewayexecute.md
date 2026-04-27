# GatewayExecute

Executes one exact gateway command selected from `GatewayIndex`.

## Parameters

- `domain` (string, required): Domain returned by `GatewayIndex`.
- `command` (string, required): Command name within the selected domain.
- `parameters` (object, required): Command-specific parameters.

## Returns

Returns the selected command's normal result. Native maenifold commands return
the same content they return through CLI direct invocation.

## Examples

```json
{
  "domain": "maenifold.memory",
  "command": "ListMemories",
  "parameters": { "path": "" }
}
```

```json
{
  "domain": "maenifold.system",
  "command": "GetConfig",
  "parameters": {}
}
```

## Errors

- Unknown domain or command
- Parameters rejected by the command schema
- Upstream MCP server failure for configured server domains

`GatewayExecute` is deterministic. It does not infer intent or choose commands
from natural language.
