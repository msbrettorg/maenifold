# Product Requirements Document — maenifold-rs

## Overview

maenifold-rs is the Rust reimplementation of the maenifold MCP server — a persistent cognitive manifold backed by a knowledge graph, memory corpus, and hierarchical state machines.

**Current state**: 28/28 tools implemented with C# oracle parity, 500+ tests passing, 15 sprints completed.
For completed requirements (FR-1 through FR-21), see [CHANGELOG.md](CHANGELOG.md).

---

### FR-23: Streamable HTTP Transport

The system SHALL support MCP Streamable HTTP transport alongside existing stdio transport, selectable via CLI flags.

- **CLI**: `maenifold --http --port <PORT>` starts HTTP server. `maenifold --mcp` continues to start stdio server.
- **Implementation**: Uses `rmcp` crate `transport-streamable-http-server` feature with `axum` web framework.
- **Dual transport**: Both transports can be compiled into the same binary. Only one is active at runtime based on CLI flags.
- **Session management**: HTTP sessions managed per MCP spec with `Mcp-Session-Id` headers.

### FR-31: OpenCode Plugin Runtime Config Helper

The system SHALL package OpenCode local plugin shared runtime helpers inside the plugin directory so relative imports resolve correctly when `~/.config/opencode/plugins` is loaded directly by OpenCode.

- **Shared helper**: `assets/integrations/opencode/plugins/lib/config.ts` SHALL provide `getMaenifoldPluginEnv(name)` and `getMaenifoldPluginFlag(name)` for all OpenCode plugin modules.
- **Resolution order**: The helper SHALL resolve `process.env` first, then fall back to `~/.config/opencode/opencode.json` → `mcp.maenifold.environment`.
- **Fail-open behavior**: Missing config files, malformed JSON/JSONC, or absent keys SHALL return `undefined` without throwing.
- **Plugin-loader safety**: Runtime plugin modules SHALL expose only callable plugin factories at top level so OpenCode's plugin loader does not attempt to invoke non-plugin helper exports.
- **Shared usage**: `persistence.ts`, `confession.ts`, and `compaction.ts` SHALL all consume the shared helper rather than duplicating config resolution logic.

### FR-32: Web UI (`--web` flag)

The system SHALL support a `--web` CLI flag that launches a browser-accessible web interface on the same port as the MCP HTTP endpoint, combining an ACP chat panel for interacting with AI coding agents and a force-directed knowledge graph visualization.

- **CLI**: `maenifold --web [--port <PORT>]` starts HTTP server with MCP endpoints at `/mcp` AND serves the web UI SPA at `/`. Superset of `--http`. Mutually exclusive with `--mcp`.
- **Feature flag**: Gated behind `web` cargo feature flag to keep the base binary lean. `web` implies `http`.
- **ACP integration**: Uses `agent-client-protocol` Rust crate to spawn AI coding agents as subprocesses and communicate via ACP JSON-RPC 2.0 over stdin/stdout.
- **WebSocket relay**: axum WebSocket endpoint at `/ws/acp` bridges browser ↔ ACP agent stdio, streaming agent responses (session notifications) to the chat panel in real time.
- **Embedded SPA**: Frontend based on [chrome-acp](https://github.com/Areo-Joe/chrome-acp) shared React components (git submodule at `vendor/chrome-acp`), compiled to static assets and embedded in the binary via `rust-embed`.
- **Graph visualization**: Force-directed graph view of the knowledge graph (concepts as nodes, co-occurrence as edges, communities as clusters), exposed via REST API endpoints at `/api/graph/*`.
- **Agent configuration**: `agents.json` config file (in maenifold config dir) defining available ACP agents with command, args, and env vars. Supports fetching the [ACP registry](https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json).
