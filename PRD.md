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


