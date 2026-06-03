# LAN exposure for MCP over Tailscale

This document explains how to expose maenifold's MCP endpoint to a private Tailnet without widening the local API surface. The mechanism lives in `crates/maenifold-core/src/mcp/http.rs`, the ACP Origin guard lives in `crates/maenifold-core/src/api/acp_relay.rs`, the workspace pins `rmcp 1.7.0`, and the transport guidance here is aligned with `MCP spec 2025-11-25` for non-stdio Host and Origin enforcement.

## Threat model

The split-listener architecture is meant to reduce the blast radius when an operator wants remote MCP access from another trusted machine on the same Tailnet. It defends against ordinary LAN peer scanning because `/mcp` can stay on loopback and be reached through Tailscale Serve instead of a raw `0.0.0.0` bind; it defends against DNS rebinding by relying on the Host and Origin allowlist hooks configured in `crates/maenifold-core/src/mcp/http.rs`; and it helps contain sandboxed-iframe RCE attempts because the ACP relay Origin checks in `crates/maenifold-core/src/api/acp_relay.rs` do not treat opaque origins as first party. It does not defend against a compromised tailnet peer that already has approved network reachability, tailnet identity compromise that lets an attacker join as an allowed principal, or host process compromise on Mac A where the local daemon and its loopback listeners are already under attacker control.

## Architecture summary

The design is intentionally narrow: maenifold keeps `/api/*` on loopback for local UI and ACP use, while only `/mcp` is made reachable through the Tailnet-facing reverse proxy. In practice that means Tailscale terminates the remote HTTPS request on Mac A and forwards just the MCP path to `127.0.0.1:8080/mcp`, while the local API listener remains bound to loopback and never becomes a LAN or Tailnet surface. This matches the split in `crates/maenifold-core/src/mcp/http.rs`, where the MCP router is nested at `/mcp` and the separate local API router serves `/api/*` on its own loopback listener.

```text
Mac B (tailnet) ──HTTPS──► tailscaled (Mac A) ──loopback──► maenifold :8080/mcp
                                                                  (only /mcp exposed)
                                                     maenifold :8081/api/* ◄── UI / Tauri
                                                                  (loopback only)
```

## Tailscale setup (operator playbook)

Install Tailscale on both Mac A, which runs maenifold, and Mac B, which acts as the remote MCP client, then sign both devices into the same Tailnet identity so policy can be enforced centrally. Bring Mac A up with the daemon tag and Mac B up with the client tag, for example `tailscale up --ssh=false --advertise-tags=tag:mcp-daemon` on Mac A and an equivalent `tailscale up --ssh=false --advertise-tags=tag:mcp-client` on Mac B after the tags exist in ACL policy; if your environment uses additional posture settings, keep those but do not widen exposure beyond the Tailnet. On Mac A, obtain the Tailscale certificate with `tailscale cert mac-a.<tailnet>.ts.net`, then publish only the MCP path with the exact command `tailscale serve --bg --set-path=/mcp http://127.0.0.1:8080/mcp` so requests to `https://mac-a.<tailnet>.ts.net/mcp` are forwarded to the loopback backend at `http://127.0.0.1:8080/mcp`, the Serve frontend remains the only remote entrypoint, and the maenifold listener itself stays on loopback. Verify from Mac B with:

```bash
curl -i -X POST https://mac-a.<tailnet>.ts.net/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"verify","version":"1"}}}'
```

Expect HTTP 200 plus a JSON-RPC response body containing `"result":{...}` with `serverInfo`; then verify the negative case from a non-tailnet LAN device with the same POST pointed at `http://<mac-a-LAN-ip>:8080/mcp` and expect connection refused or timeout, because the default listener posture keeps `/mcp` off the raw LAN even though Tailnet clients can reach it through Serve.

## Tailnet ACL configuration

Use a dedicated tag split so only approved clients can connect to approved daemons, and keep the policy explicit enough that a future operator can audit it quickly. The minimum viable pattern is to let administrators assign `tag:mcp-daemon` and `tag:mcp-client`, then permit only the client tag to reach TCP 443 on the daemon tag because the Tailscale Serve frontend is the public Tailnet surface. Critically, there should be no `funnel` grant anywhere in the ACL for this setup, because Funnel would publish beyond the private Tailnet boundary and breaks the intent of this document.

```json
{
  "tagOwners": {
    "tag:mcp-daemon": ["autogroup:admin"],
    "tag:mcp-client": ["autogroup:admin"]
  },
  "acls": [
    {
      "action": "accept",
      "src": ["tag:mcp-client"],
      "dst": ["tag:mcp-daemon:443"]
    }
  ],
  "ssh": [],
  "nodeAttrs": [
    {
      "target": ["tag:mcp-daemon"],
      "attr": ["no-funnel"]
    }
  ]
}
```

## Tailnet Lock + FIDO2

Network policy is not enough if identity enrollment is weak, so the operator should harden the Tailnet control plane before relying on remote MCP access. Enable Tailnet Lock so new node authorization requires tailnet-key approval rather than silent control-plane acceptance, require a FIDO2 hardware key on the SSO identity that administers the Tailnet, and document the recovery path before rollout by storing printed recovery codes or equivalent break-glass materials offline for the people who hold the admin role. If a hardware key is lost and recovery has not been documented, the safe response is to stop and restore administrative access first instead of leaving the Tailnet in a half-locked state.

## maenifold.json opt-in

The daemon-side allowlist is opt-in and should be populated deliberately when exposing MCP over a Tailnet hostname. In `maenifold.json`, set both `mcpAllowedHosts` and `mcpAllowedOrigins` to include the Tailnet hostname you actually serve, and remember that in `rmcp 1.7.0` these arrays replace defaults when non-empty rather than merging with them implicitly. That means an operator who wants to preserve loopback access must keep the loopback entries in the explicit arrays alongside the Tailnet host, otherwise a local client can start failing even though the Tailnet path works.

```json
{
  "mcpAllowedHosts": [
    "localhost",
    "127.0.0.1",
    "::1",
    "mac-a.<tailnet>.ts.net"
  ],
  "mcpAllowedOrigins": [
    "http://localhost",
    "http://127.0.0.1",
    "http://[::1]",
    "https://mac-a.<tailnet>.ts.net"
  ]
}
```

## Verification (end-to-end)

Run the checks from each trust boundary and treat each failure mode as a clue about which layer is misconfigured. From Mac B, use a real JSON-RPC initialize request:

```bash
curl -i -X POST https://mac-a.<tailnet>.ts.net/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"verify","version":"1"}}}'
```

Expected: HTTP 200 + JSON-RPC response body containing `"result":{...}` with `serverInfo`.
If it times out or cannot connect, check that Tailscale Serve is running and ACLs allow `tag:mcp-client` to reach `tag:mcp-daemon:443`.
If it returns a Host or Origin-related rejection, re-check the `maenifold.json` allowlists described above and the Host/Origin enforcement points in `crates/maenifold-core/src/mcp/http.rs` plus `crates/maenifold-core/src/api/acp_relay.rs`.
From Mac A itself, send the same POST to `http://127.0.0.1:8080/mcp` and expect HTTP 200 plus the JSON-RPC initialize result, which proves the loopback MCP listener is healthy before Serve is layered on top.
From a non-tailnet LAN device, send the same POST to `http://<mac-a-LAN-ip>:8080/mcp` and expect connection refused or timeout; if it succeeds, the MCP listener was bound too broadly and the deployment no longer matches this playbook.
For local API isolation, `curl http://<mac-a-LAN-ip>:8081/api/health` should also fail from a remote device.
On Mac A, `curl http://127.0.0.1:8081/api/health` should return HTTP 200, confirming `/api/*` stayed loopback-only.

## Decommissioning

To remove the exposure, reverse every policy layer instead of assuming the proxy alone is enough. On Mac A, run `tailscale serve reset` to remove the Serve mapping, then remove the `tag:mcp-daemon` assignment or withdraw the ACL path that allowed `tag:mcp-client` to reach it, and finally revert the `maenifold.json` allowlists so `mcpAllowedHosts` and `mcpAllowedOrigins` no longer include the Tailnet hostname if that host should not remain trusted. After rollback, repeat the negative checks so Tailnet requests to `https://mac-a.<tailnet>.ts.net/mcp` fail and local loopback access still works only where intended.

## What this does NOT do

This setup does not expose `/api/*`; it does not expose inferencing or sampling surfaces that still assume the local client and local operator trust boundary; it does not create multi-tenant isolation between different Tailnet peers sharing the same daemon; and it does not add an audit log specifically attributing tailnet-originated requests beyond whatever generic daemon logging already exists. It is strictly a controlled way to publish `/mcp` to a private Tailnet path while leaving the rest of the process model unchanged, so any requirement beyond that scope needs a separate design rather than being inferred from this operator playbook.
