# GatewayIntent

Resolves natural-language intent to deterministic gateway candidates when the
gateway intent resolver is enabled and configured.

## Availability

`GatewayIntent` is advertised only when intent routing is enabled and the
configured ACP/Jockey resolver agent is available. It is absent from
`tools/list` when disabled.

## Parameters

- `intent` (string, required): Natural language request to resolve.
- `domainHints` (string[], optional): Domains to bias candidate selection.

## Behavior

- Returns deterministic command candidates and safety notes.
- May auto-execute only safe single read operations when policy allows it.
- Must not silently execute mutating commands.

## Use GatewayExecute for normal operation

Prefer `GatewayIndex` + `GatewayExecute` whenever the caller already knows the
exact command. `GatewayIntent` is for discovery and assisted routing, not the
default execution path.
