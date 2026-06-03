# Security Model

maenifold now serves two HTTP listeners, so the security boundary is no longer "one loopback socket for everything." This reference describes the shipped posture only and matches the split-listener implementation in `crates/maenifold-core/src/mcp/http.rs`, `crates/maenifold-core/src/api/acp_relay.rs`, and `crates/maenifold-core/src/config.rs`.

## Listener topology

| Listener | Default bind | Surface | Exposable? |
| --- | --- | --- | --- |
| MCP (`/mcp`) | `127.0.0.1:8080` | MCP JSON-RPC only | YES, via transport-layer auth (Tailscale Serve / mTLS / SSH tunnel) |
| Local API (`/api/*`) | `127.0.0.1:8081` | config / user-prompt / acp/session / machine-id / health | NO — always loopback, no CLI flag |

The split is enforced in `crates/maenifold-core/src/mcp/http.rs`: `serve_split_with_shutdown` binds the MCP listener from the configurable MCP bind address, then separately binds the Local API listener to `127.0.0.1:{local_api_port}`.
Only `/mcp` is intended for optional LAN or tailnet exposure, and even then only behind transport-layer auth chosen by the operator.
`/api/*` stays on the loopback-only listener because it carries privileged endpoints such as config writes, the user-prompt bridge, `/api/acp/session`, machine-id, and health.
There is no flag that moves the Local API listener off loopback, so operators must treat `127.0.0.1:8081` as non-exposable by design.
This means the attack surface exposed to remote peers is intentionally narrower than the full daemon surface.

## Trust model

Loopback peers are trusted on BOTH listeners by policy because the model assumes the operator controls local processes on the host. Loopback TCP is not a cryptographic or per-UID auth boundary, so any local process that can reach the socket can speak to it; the trust decision is operational, not enforced by the TCP transport itself.
Tailnet peers are trusted only if the operator deliberately exposes the MCP listener through a transport edge such as Tailscale Serve and trusts the tailnet identity that the edge authenticates.
That trust decision is outside Rust middleware: the daemon does not parse Authorization headers or inject application auth on `/mcp`; the authenticated boundary lives at the transport edge.
Tailnet peers CANNOT reach `/api/*` because the Local API listener remains loopback-only on `127.0.0.1:8081`, independent of the MCP bind setting.
Tailnet peers CANNOT spawn local CLI subprocesses because the ACP relay that launches them is on `/api/acp/session` in the loopback-only listener, not on the exposable `/mcp` listener.

## DNS rebinding

The MCP HTTP stack in `crates/maenifold-core/src/mcp/http.rs` builds `StreamableHttpServerConfig` and can call `with_allowed_hosts` and `with_allowed_origins` before the `/mcp` service is exposed.
rmcp `StreamableHttpServerConfig::with_allowed_hosts` enforces a Host allowlist whose default is `["localhost", "127.0.0.1", "::1"]`; when maenifold passes a non-empty `mcpAllowedHosts` vector to `with_allowed_hosts`, that call REPLACES rmcp's default vector rather than extending it. If you supply a tailnet hostname, you must also include `localhost`, `127.0.0.1`, and `::1` explicitly in `mcpAllowedHosts` if you want those loopback names preserved.
rmcp `with_allowed_origins` enforces an Origin allowlist populated from `mcpAllowedOrigins`; when maenifold passes a non-empty vector to `with_allowed_origins`, that call REPLACES rmcp's default `[]` rather than extending it. Operators therefore must list every allowed origin explicitly, and if they are preserving loopback host access via a parallel host policy such as a tailnet hostname plus `localhost`, `127.0.0.1`, and `::1`, they must keep the full desired origin set in `mcpAllowedOrigins` as well.
The ACP WebSocket path has a separate hand-rolled Origin guard in `crates/maenifold-core/src/api/acp_relay.rs`, specifically the `check_origin` function, because `/api/acp/session` is not protected by rmcp's MCP transport stack.
Opaque origins are intentionally not first-party there: `Origin: null` and `Origin: file://` are excluded because sandboxed iframes can synthesize `Origin: null`; operators who truly need those origins must list them explicitly in `mcpAllowedOrigins`.

## What auth is NOT in scope

No Authorization header parsing is in scope for the daemon.
No JWT validation is in scope for the daemon.
No OAuth 2.1 in-process is in scope for the daemon.
No bearer tokens are in scope for the daemon.
No mTLS termination is in scope for the daemon; transport-layer TLS termination is delegated to the exposure mechanism, and Tailscale handles TLS when Tailscale Serve is the edge.
All of the above are intentionally excluded per Constitution P0 #4: auth lives at the transport edge, not in Rust middleware.

## Operator obligations when exposing to LAN

If an operator exposes the MCP listener beyond loopback, the minimum bar is the LAN guidance in `docs/lan-exposure.md`, plus explicit review of the MCP/ACP split described above.
Tailnet Lock must be enabled so the tailnet trust decision is rooted in locked node identity rather than a soft default.
The SSO identity used to administer that tailnet must require a FIDO2 hardware key, not password-only or software-only second factors.
The tailnet policy must use tag-based ACLs with no default-allow reachability inside the tailnet, and it must include no Funnel grant at all.
Before exposure, the operator must explicitly populate both `mcpAllowedHosts` and `mcpAllowedOrigins`, because relying on defaults is not sufficient for a LAN-facing or tailnet-facing deployment.
