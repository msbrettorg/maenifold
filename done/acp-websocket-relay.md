---
id: T-80.5
title: Acp Websocket Relay
status: done
pipeline: sdd
requirement: FR-32
---

# acp-websocket-relay

**Requirement**: FR-32
**Test Cases**: TC-32.6, TC-32.7
**Component**: crates/maenifold-core/src/acp/relay.rs (new)

## Acceptance Criteria

1. axum WebSocket endpoint at `/ws/acp` accepts browser connections
2. Server assigns a unique session_id upon WebSocket connection and returns it as `{ "type": "session", "session_id": "..." }`. All subsequent messages from the browser include this server-issued session_id.
3. Browser sends maenifold relay protocol messages with `{ "type": "prompt", "session_id": "...", "messages": [...] }` structure
4. Relay forwards prompt to the AcpManager which calls `conn.prompt()` on the ACP agent
5. ACP `session_notification` responses (streaming text chunks) are forwarded to the browser over the same WebSocket as maenifold relay protocol JSON `{ "type": "chunk", "content": "..." }`
6. Multiple concurrent WebSocket clients can connect (each gets its own ACP session)
7. WebSocket close triggers ACP session cleanup (agent process teardown)
8. Malformed messages from browser return maenifold relay protocol JSON error `{ "type": "error", "message": "..." }` without closing the connection
9. If the ACP agent process crashes or exits unexpectedly, the relay sends `{ "type": "error", "message": "Agent process exited unexpectedly" }` and closes the WebSocket with code 1011 (unexpected condition)

## Verdicts

- TC-32.6 (WS relays maenifold relay protocol messages between browser and agent): ADD — test does not exist yet
- TC-32.7 (agent responses stream to browser via WS): ADD — test does not exist yet
