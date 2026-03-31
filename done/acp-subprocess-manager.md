---
id: acp-subprocess-manager
status: done
---

# acp-subprocess-manager

**Requirement**: FR-32
**Test Cases**: TC-32.4, TC-32.5, TC-32.13, TC-32.14
**Component**: crates/maenifold-core/src/acp/manager.rs (new)

## Acceptance Criteria

1. `AcpManager` is per-connection — each WebSocket client spawns a dedicated agent subprocess with its own ACP session
2. `AcpManager` spawns an ACP agent as a child process given a command, args, and env vars from agent config
3. If the agent command is not found or fails to exec, `AcpManager::spawn()` returns `Err` with the OS error message — no panic
4. Communication uses the `agent-client-protocol` crate (from `https://github.com/agentclientprotocol/rust-sdk`) — implements the `Client` trait
5. `ClientSideConnection::new()` is established over the child process stdin/stdout
6. ACP `initialize` request is sent with maenifold's client info (name: "maenifold", version from Cargo.toml)
7. ACP `initialize` has a 10-second timeout; if exceeded, the agent subprocess is killed and `Err` returned
8. ACP `new_session` request is sent with the current working directory, returning a session ID
9. `prompt()` sends user messages to the agent and receives streaming `session_notification` responses
10. `session_notification` handler extracts `ContentBlock::Text` content and forwards to the WebSocket relay
11. Agent process is killed on session close or server shutdown (kill_on_drop)

## Verdicts

- TC-32.4 (ACP agent subprocess spawned from config): ADD — test does not exist yet
- TC-32.5 (ACP initialize + new_session handshake): ADD — test does not exist yet
- TC-32.13 (ACP spawn failure returns Err with OS error): ADD — test does not exist yet
- TC-32.14 (ACP initialize timeout kills subprocess and returns Err): ADD — test does not exist yet
