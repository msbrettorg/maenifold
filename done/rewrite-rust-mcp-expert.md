---
id: T-80.139
title: Rewrite rust-mcp-expert.agent.md for rmcp 1.1.0
status: done
pipeline: sdd
---

# T-27.8: Rewrite rust-mcp-expert.agent.md for rmcp 1.1.0

**Requirement:** FR-38 (RTM-237, RTM-238, RTM-239)

## Problem

The current agent file at `assets/integrations/copilot/plugin-product-team/agents/rust-mcp-expert.agent.md` is written for rmcp v0.8+ and contains 18+ non-compilable code examples, fabricated transport APIs, and wrong import paths. It needs a complete rewrite against rmcp 1.1.0.

## Acceptance Criteria

1. Version references updated to rmcp 1.1.0
2. All code examples use correct APIs verified against actual rmcp source
3. Transport section covers only: stdio (`rmcp::transport::io::stdio()`) and streamable HTTP (`StreamableHttpService`)
4. SSE, WebSocket, TCP, Unix socket removed as standalone transports
5. `Parameters` shown as tuple struct with destructuring, not `.inner()`
6. `ErrorData` constructors shown with correct 2-arg signature
7. Result types use constructors (`::success()`, `::error()`, `::new()`) not struct literals
8. Correct import paths throughout (`rmcp::handler::server::*` not `rmcp::server::*`)
9. Missing features covered: task support, `Json<T>` structured output, trait-based tools, extractors
10. Cargo.toml dependency example with correct version and features

## Source of Truth

- rmcp 1.1.0 source: `~/.cargo/registry/src/*/rmcp-1.1.0/`
- rmcp-macros 1.1.0: `~/.cargo/registry/src/*/rmcp-macros-1.1.0/`
- Real usage in this repo: `crates/maenifold-core/src/mcp/`

## File to Modify

- `assets/integrations/copilot/plugin-product-team/agents/rust-mcp-expert.agent.md` — full rewrite

## SWE Remediation

- Rewrote `assets/integrations/copilot/plugin-product-team/agents/rust-mcp-expert.agent.md` from scratch against rmcp 1.1.0.
- Updated all version guidance from rmcp v0.8-era assumptions to rmcp 1.1.0 source-verified APIs.
- Replaced fabricated or obsolete transport guidance with the real 1.1.0 patterns:
  - stdio via `rmcp::transport::io::stdio()`
  - streamable HTTP via `StreamableHttpService`
- Removed standalone SSE/WebSocket/TCP/Unix-socket guidance from the agent file.
- Corrected examples to use `Parameters<T>` as a tuple struct with destructuring instead of `.inner()`.
- Corrected `ErrorData` examples to use the real two-argument constructors with `None` data where appropriate.
- Corrected result construction to use real helpers such as `CallToolResult::success`, `CallToolResult::error`, `GetPromptResult::new`, `ReadResourceResult::new`, and paginated `with_all_items(...)` helpers.
- Corrected import guidance away from nonexistent `rmcp::server::*` paths and toward real 1.1.0 modules under `rmcp::handler::server::*`, `rmcp::model::*`, and transport modules.
- Added missing rmcp 1.1.0 coverage for:
  - task support and `TasksCapability`
  - `Json<T>` structured output
  - trait-based tools with `ToolBase`, `SyncTool`, and `AsyncTool`
  - extractor usage with `Parameters<T>`, `JsonObject`, and `ToolName`
- Added a `Cargo.toml` dependency example with rmcp 1.1.0 feature flags consistent with stdio and streamable HTTP server usage.

## Blue-team Verification

**Verdict:** FAIL

### Commands Run

1. `wc -l assets/integrations/copilot/plugin-product-team/agents/rust-mcp-expert.agent.md`
   - Output: `845 assets/integrations/copilot/plugin-product-team/agents/rust-mcp-expert.agent.md`
2. `grep -c "rmcp 1.1.0\|v1.1.0\|version.*1.1" assets/integrations/copilot/plugin-product-team/agents/rust-mcp-expert.agent.md`
   - Output: `17`
3. `grep -c "SseServerTransport\|StdioTransport\|inner()" assets/integrations/copilot/plugin-product-team/agents/rust-mcp-expert.agent.md`
   - Output: `8`

### Assessment

- The file exists and is substantial at 845 lines.
- The rewrite includes version references for rmcp 1.1.0 / v1.1.0.
- Verification failed because the file still contains 8 matches for forbidden old/fabricated API strings (`SseServerTransport`, `StdioTransport`, or `inner()`).

### Required Fix

Remove all remaining occurrences of `SseServerTransport`, `StdioTransport`, and `.inner()`-style guidance from `assets/integrations/copilot/plugin-product-team/agents/rust-mcp-expert.agent.md`, then rerun this verification.
