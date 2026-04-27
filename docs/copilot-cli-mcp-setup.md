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

Maenifold's HTTP MCP transport targets the MCP Authorization specification
revision `2025-06-18`. Unauthenticated requests to `/mcp` return `401` with a
`WWW-Authenticate` `resource_metadata` pointer so MCP clients can discover the
local OAuth metadata.

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

Copilot CLI can continue to use this direct bearer-token configuration.

## OpenCode OAuth setup

OpenCode can use MCP OAuth discovery against the same endpoint:

```text
opencode mcp auth maenifold
```

The local server exposes these unauthenticated OAuth endpoints outside the
`/mcp` bearer-token middleware:

- `/.well-known/oauth-protected-resource`
- `/.well-known/oauth-protected-resource/mcp`
- `/.well-known/oauth-authorization-server`
- `/.well-known/oauth-authorization-server/mcp`
- `/register`
- `/authorize`
- `/token`

The authorization flow is OAuth authorization code with PKCE `S256` and RFC
8707 `resource` validation for `http://127.0.0.1:8080/mcp` (or the actual
loopback host/port used by the request).

## Generic MCP HTTP clients

Generic MCP HTTP clients may either:

1. Read `~/maenifold/auth/localhost.token` and send it on every MCP HTTP
   request as `Authorization: Bearer <token>`, or
2. Follow the MCP Authorization 2025-06-18 discovery flow from the
   `WWW-Authenticate` header, dynamically register a loopback redirect URI,
   authorize with PKCE `S256`, and exchange the code at `/token`.

Redirect URIs are accepted only for trusted loopback HTTP origins:
`http://127.0.0.1`, `http://localhost`, or `http://[::1]`.

## Trusted-loopback OAuth token model

The OAuth bridge is a local convenience bridge for trusted loopback clients. It
issues the existing shared local bearer token from
`~/maenifold/auth/localhost.token` as the OAuth `access_token`. It is not a
general multi-user authorization server:

- loopback-only redirect URIs;
- local-user trust boundary;
- shared local bearer token;
- no refresh token;
- no token expiry;
- no per-client scoping or per-client audience isolation beyond validating the
  local MCP `resource` value during authorize/token exchange.

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
