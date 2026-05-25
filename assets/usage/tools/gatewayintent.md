# GatewayIntent

Resolves natural-language intent to deterministic gateway candidates when the
gateway intent resolver is enabled.

## Availability

`GatewayIntent` is advertised when intent routing is enabled in configuration.
It is absent from `tools/list` when disabled. It does **not** require an
ACP/Jockey agent to be configured.

## Resolver paths (exactly two)

`GatewayIntent` resolves intent through exactly one of two paths, chosen at
request time based on the calling MCP client's capabilities:

1. **MCP client sampling** — when the MCP client advertises
   `sampling/createMessage` (VS Code, GitHub Copilot extension, other
   sampling-capable clients), the daemon calls `peer.create_message(...)` to
   ask the client's own model to produce the resolver output. The user sees
   the sampling request in their client's UI and approves or denies it.
2. **Direct inferencing** — when the calling MCP client does not advertise
   sampling (Copilot CLI, generic stdio clients), the daemon makes a direct
   inference call against the configured inference backend (Azure OpenAI,
   OpenAI, etc.) and uses that output as the resolver output.

Both paths write request/response evidence to `sampling://sessions` so the
desktop Logs tab can show every GatewayIntent inference (no hidden inference).
Logs distinguish the two paths via the event type: `sampling` for path (1),
`inferencing` for path (2).

## What `GatewayIntent` is NOT

- It is **not** backed by an ACP/Jockey CLI agent. Do not configure a resolver
  agent in `maenifold.json` for this surface. If direct inferencing is not
  configured and the client does not support sampling, `GatewayIntent` returns
  a setup-required response — it does not fall back to ACP.
- It does **not** spawn a Codex/Claude/Copilot CLI subprocess.
- It does **not** use `AcpExec`. `AcpExec` is for user-visible Maenifold UI
  / ACP agent sessions only.

## Parameters

- `goal` (string, required): Natural language request to resolve.
- `domainHints` (string[], optional): Domains to bias candidate selection.

## Behavior

- Returns deterministic command candidates and safety notes.
- May auto-execute only safe single read operations when policy allows it.
- Must not silently execute mutating commands.
- All inference (sampling or direct) is visible in `sampling://sessions` /
  Logs with the request, response, model, and chosen resolver path.

## Use GatewayExecute for normal operation

Prefer `GatewayIndex` + `GatewayExecute` whenever the caller already knows the
exact command. `GatewayIntent` is for discovery and assisted routing, not the
default execution path.
