# Copilot CLI MCP setup for gate access

Copilot CLI sessions access gate operations through maenifold's integrated
Gateway. All gate operations use the maenifold MCP Gateway exclusively.

## Prerequisites

Start the maenifold HTTP MCP daemon on port 8080:

```bash
maenifold --http --port 8080
```

The MCP endpoint is:

```text
http://127.0.0.1:8080/mcp
```

## Copilot CLI MCP config

`~/.copilot/mcp-config.json` must include the maenifold MCP server:

```json
{
  "mcpServers": {
    "maenifold": {
      "url": "http://127.0.0.1:8080/mcp",
      "type": "http",
      "tools": ["*"],
      "headers": {
        "Authorization": "Bearer <token from ~/maenifold/auth/localhost.token>"
      }
    }
  }
}
```

## Token sync

The bearer token in `~/.copilot/mcp-config.json` must match the current contents
of `~/maenifold/auth/localhost.token`.

If the config contains a stale token, authentication fails and maenifold Gateway
tools will not appear in Copilot CLI.

## Gate commands exposed by Gateway

Use `GatewayExecute` with:

```text
domain="maenifold.gate"
```

The Gateway exposes these gate commands:

- `gate_Status`
- `gate_Sync`
- `gate_Pass`
- `gate_Fail`
- `gate_Append`
- `gate_History`
- `gate_TaskBody`

## Troubleshooting

If Gateway tools do not appear in Copilot CLI:

1. Confirm the maenifold daemon is running:
   `maenifold --http --port 8080`
2. Confirm the MCP config token matches `~/maenifold/auth/localhost.token`.
3. Restart the Copilot CLI session after changing MCP config or token values.
