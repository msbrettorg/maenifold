# MCP authentication setup for maenifold HTTP

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

The HTTP MCP transport requires a bearer token on every request to `/mcp`.
Unauthenticated requests return `401` with a generic
`WWW-Authenticate: Bearer realm="maenifold"` header.

## Copilot CLI MCP config

`~/.copilot/mcp-config.json` must include the maenifold MCP server:

```json
{
  "mcpServers": {
    "maenifold": {
      "url": "http://127.0.0.1:8080/mcp",
      "type": "http",
      "tools": ["*"]
    }
  }
}
```

## Generic MCP HTTP clients

Point any MCP HTTP client at `http://127.0.0.1:8080/mcp`. No headers, no token,
no auth. The server is a localhost loopback service that trusts its caller.

If the MCP surface needs to be exposed beyond loopback, that is APIM's job —
not the Rust backend's. See `AGENTS.md` "Cargo-cult security" P0 trigger.

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
2. Restart the Copilot CLI session after changing MCP config values.
