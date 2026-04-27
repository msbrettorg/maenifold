# GatewayIndex

Lists deterministic gateway domains and commands. Filter by domain and command
to inspect exact schemas and safety classes before executing a command.

## Parameters

- `domain` (string, optional): Gateway domain to inspect, such as
  `maenifold.memory`, `maenifold.system`, `gate`, or a configured upstream MCP
  server name.
- `command` (string, optional): Command name within `domain`. Requires
  `domain`.
- `includeSchemas` (bool, optional): Include command JSON schemas. Default:
  `false`.
- `includeExamples` (bool, optional): Include examples when available. Default:
  `false`.

## Returns

- No filter: `{ "domains": [{ "name", "description", "command_count" }] }`
- Domain filter: `{ "domain": "...", "commands": [{ "name", "description",
  "safety_class" }] }`
- Domain + command filter: `{ "domain": "...", "command": { ... } }`

## Examples

```json
{}
```

```json
{ "domain": "maenifold.memory" }
```

```json
{
  "domain": "maenifold.system",
  "command": "GetConfig",
  "includeSchemas": true
}
```

## Notes

Use `GatewayIndex` before `GatewayExecute`. First-party commands are not flat
public MCP tools; their domains come from `GatewayIndex`.
