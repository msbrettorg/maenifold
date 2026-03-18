---
name: agent-client-protocol
description: Build, debug, and explain Agent Client Protocol (ACP) integrations using the official ACP protocol documentation. Use when agents need protocol details for initialization, sessions, prompt turns, tool calls, filesystem, terminals, session modes, content, extensibility, and related ACP message flow.
---

# Agent Client Protocol

The Agent Client Protocol allows Agents and Clients to communicate by exposing methods that each side can call and sending notifications to inform each other of events.

This skill is intentionally **hybrid**:

- **Local**: official ACP protocol pages are mirrored in `references/` with only minimal cleanup edits.
- **Dynamic**: exact schema details and less-common protocol pages should be fetched from the official ACP protocol docs when needed.

## When to use this skill

Use this skill when you need to:

- implement an ACP-compatible **agent** or **client**
- debug ACP **handshake**, **session**, or **prompt-turn** behavior
- understand **tool calls**, **permissions**, **filesystem**, or **terminal** integration
- add or inspect **session modes**, **content blocks**, or **protocol extensibility**

## Local reference map

Read these local mirrors first:

| Topic | Local file |
|---|---|
| Protocol overview | `references/overview.md` |
| Capability negotiation | `references/initialization.md` |
| Session creation/loading | `references/session-setup.md` |
| Prompt lifecycle | `references/prompt-turn.md` |
| Tool call lifecycle and permissions | `references/tool-calls.md` |
| Client file access | `references/file-system.md` |
| Session modes | `references/session-modes.md` |
| Terminal execution | `references/terminals.md` |
| Content blocks | `references/content.md` |
| `_meta` and custom methods | `references/extensibility.md` |

## Essential ACP rules

Keep these details in working memory:

- ACP uses **JSON-RPC 2.0** with both **methods** and **notifications**.
- Clients **MUST** call `initialize` before session methods.
- Clients create or resume work using `session/new` or `session/load`.
- A prompt turn runs through `session/prompt` plus streamed `session/update` notifications, and completes with a `stopReason`.
- Agents **MAY** request permission with `session/request_permission` before executing sensitive tool calls.
- Filesystem and terminal methods are gated by **client capabilities** and **MUST NOT** be used unless advertised.
- All file paths in ACP **MUST be absolute**.
- Line numbers are **1-based**.
- On cancellation, agents should stop work and return the semantic `cancelled` stop reason rather than leaking transport/runtime exceptions.
- Terminals must be **released** when no longer needed.

## Quick start path

1. Read `references/introduction.md`
2. Read `references/overview.md`
3. Read `references/initialization.md`
4. Read `references/session-setup.md`
5. Read `references/prompt-turn.md`
6. If tools are involved, read `references/tool-calls.md`
7. If local file or shell integration is involved, read `references/file-system.md` and `references/terminals.md`

See:

- `sample_codes/getting-started/minimal-session.md`
- `sample_codes/common-patterns/request-permission.md`

## Dynamic lookup guidance

For exact protocol details or pages not mirrored locally, fetch the official ACP protocol docs directly.

### Recommended official pages

- Overview: `https://agentclientprotocol.com/protocol/overview`
- Schema: `https://agentclientprotocol.com/protocol/schema`
- Initialization: `https://agentclientprotocol.com/protocol/initialization`
- Session setup: `https://agentclientprotocol.com/protocol/session-setup`
- Prompt turn: `https://agentclientprotocol.com/protocol/prompt-turn`
- Tool calls: `https://agentclientprotocol.com/protocol/tool-calls`
- File system: `https://agentclientprotocol.com/protocol/file-system`
- Terminals: `https://agentclientprotocol.com/protocol/terminals`
- Session modes: `https://agentclientprotocol.com/protocol/session-modes`
- Content: `https://agentclientprotocol.com/protocol/content`
- Extensibility: `https://agentclientprotocol.com/protocol/extensibility`
- Slash commands: `https://agentclientprotocol.com/protocol/slash-commands`

### Use dynamic fetches when

- you need exact field shapes from the schema
- you need a page not mirrored in `references/`
- you need to confirm evolving or experimental behavior

## Minimal-edit provenance

The files in `references/` are local mirrors of the official ACP docs with only minimal cleanup edits such as:

- removing site wrapper text
- adding a source comment
- preserving original headings, prose, examples, and relative structure where practical

When accuracy matters more than convenience, prefer the official source URLs above.
