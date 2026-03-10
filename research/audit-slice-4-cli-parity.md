# Audit Slice 4: CLI Behavior Parity

**Date**: 2026-03-10
**Traceability**: T-10.4 (FR-12), T-10.2 (FR-11), RTM-072, RTM-073, RTM-068–RTM-071
**Sequential Thinking Session**: session-1773104046670-38000 / branch: audit-slice-4-cli
**Status**: Complete — 4 DRIFTs, 4 MATCHes across 12 features

---

## Executive Summary

The Rust CLI is **functionally equivalent** for the core tool-invocation pipeline (`--tool`, `--payload`, `--json`). All 28 tool names match. Error handling codes and message format are aligned. Three **blocking** drifts exist: (1) usage display exits 0 instead of 1, (2) `--mcp` mode does not auto-start watchers (FR-12), and (3) MCP `resources` capability is absent (FR-11). Two **important** drifts exist in JSON envelope handling.

---

## Detailed Findings

### CLI Feature: --mcp flag
**C# behavior**: Detected via `args.Contains("--mcp")`. Builds .NET Host with MCP server, logging (Warning → stderr), and `WithResourcesFromAssembly()`. Auto-starts `IncrementalSyncTools.StartWatcher()` if `Config.EnableIncrementalSync` is true, always starts `AssetWatcherTools.StartAssetWatcher(mcpServer)`.
**Rust behavior**: Detected via `args.iter().any(|arg| arg == "--mcp")`. Calls `initialize_runtime()` then `MaenifoldMcpServer::serve_stdio()`. No watcher auto-start. No MCP resources capability. No logging framework.
**Status**: DRIFT
**Severity**: blocking
**RTM**: RTM-072 (auto-start sync watcher), RTM-073 (auto-start asset watcher), RTM-068 (resources capability)
**TODO**: T-10.4 (pending), T-10.2 (pending), T-10.3 (pending)

---

### CLI Feature: --tool flag
**C# behavior**: `Array.IndexOf(args, "--tool")` — returns index or -1. If missing, prints usage and exits 1. Extracts tool name from `args[toolIndex + 1]`.
**Rust behavior**: `args.iter().position(|arg| arg == "--tool")` — functionally equivalent. If missing, returns `Ok(usage())` (exits 0).
**Status**: MATCH (detection logic). DRIFT (exit code on missing — see "Usage exit code" below).
**Severity**: N/A for detection; see exit code entry.

---

### CLI Feature: --payload flag
**C# behavior**: REQUIRED. `Array.IndexOf(args, "--payload")`. If absent or missing value, prints usage and exits 1.
**Rust behavior**: OPTIONAL. If absent, defaults to `"{}"`. If present but missing value, prints usage (exits 0). Deliberate improvement per T-0.2 comment.
**Status**: DRIFT (intentional)
**Severity**: cosmetic
**Notes**: The Rust behavior is strictly more permissive — all valid C# invocations work identically. `--tool GetConfig --json` works in Rust without `--payload '{}'`.

---

### CLI Feature: --json flag
**C# behavior**: `args.Contains("--json")` → sets `OutputContext.Format = OutputFormat.Json`.
**Rust behavior**: `args.iter().any(|arg| arg == "--json")` → sets `output_format = OutputFormat::Json`.
**Status**: MATCH
**Severity**: N/A

---

### CLI Feature: --version / --help flags
**C# behavior**: Neither `--version` nor `--help` is explicitly handled. No flag exists. Version is 1.0.3 in `.csproj`.
**Rust behavior**: Neither flag handled. Version is 0.1.0 in `Cargo.toml`.
**Status**: MATCH
**Severity**: N/A
**Notes**: Both show usage text on bare invocation (no `--tool`).

---

### CLI Feature: Usage display exit code
**C# behavior**: Sets `Environment.ExitCode = 1` before printing usage (lines 51, 60 of Program.cs). No `--tool` → exit 1. No `--payload` → exit 1.
**Rust behavior**: Returns `Ok(usage())` which prints to stdout and exits 0 for missing `--tool`. Returns `Ok(usage())` for `--payload` present but missing value.
**Status**: DRIFT
**Severity**: blocking
**Notes**: Scripts that check `$?` to detect invalid invocations will get different results. C# treats bare invocation as an error (exit 1); Rust treats it as informational (exit 0).

---

### CLI Feature: JSON envelope detection for exit codes
**C# behavior**: `TryGetJsonEnvelopeSuccess()` checks for ALL three properties: `success` (boolean), `data`, and `error`. Only returns true (recognized as envelope) if all three exist. If not recognized as envelope, wraps in `JsonToolResponse.Ok(output)`.
**Rust behavior**: `is_json_failure_envelope()` only checks for `success` field being `false`. Does NOT verify `data` or `error` presence.
**Status**: DRIFT
**Severity**: important
**Notes**: Any JSON with `"success": false` (even without `data`/`error`) triggers exit 1 in Rust but would be treated as a bare string in C# (wrapped in an Ok envelope). In practice unlikely since all tools produce well-formed envelopes, but the strictness differs.

---

### CLI Feature: JSON normalization safety net (--json mode)
**C# behavior**: `WriteToolOutput()` applies a normalization step. If tool output is NOT a recognized JSON envelope (missing `success`+`data`+`error`), C# wraps it in `JsonToolResponse.Ok(output).ToJson()` → `{"success":true,"data":"...","error":null}`.
**Rust behavior**: No safety net. Tool output is passed through directly. Each tool is responsible for checking `output_format` and calling `json::success()` to produce envelopes.
**Status**: DRIFT
**Severity**: important
**Notes**: If any tool returns bare markdown when `--json` is set, C# would wrap it in an envelope; Rust would emit raw markdown. All current tools appear to handle `--json` correctly via the `ToolContext.output_format` mechanism, so this is a latent risk rather than an active bug.

---

### CLI Feature: Error message format
**C# behavior**: `Error invoking {toolName}: {ex.Message}` for all error types. Error codes: `UNKNOWN_TOOL`, `INVALID_PAYLOAD`, `INVALID_ARGUMENT`, `INTERNAL_ERROR`.
**Rust behavior**: Same format: `Error invoking {tool_name}: {err}`. Same error codes. Special handling for `MissingProperty` → `INVALID_ARGUMENT` with `Required property '{msg}' is missing from payload.` matching C# wording exactly.
**Status**: MATCH
**Severity**: N/A

---

### CLI Feature: Tool registry and case sensitivity
**C# behavior**: `Dictionary<string, ToolDescriptor>(StringComparer.OrdinalIgnoreCase)`. 28 tools. Aliases supported.
**Rust behavior**: `BTreeMap<String, ToolDescriptor>` with `to_ascii_lowercase()` on insert and lookup. 28 tools. Each has one lowercase alias. Tool names are identical across both registries.
**Status**: MATCH
**Severity**: N/A

---

### CLI Feature: learn flag
**C# behavior**: Each tool checks `payload.TryGetProperty("learn", ...)`. Delegates to `ToolHelpers.GetLearnContent()` which reads `~/maenifold/assets/usage/tools/{toolname}.md`.
**Rust behavior**: Each tool checks `payload.get("learn")`. Reads from same path via config.
**Status**: MATCH
**Severity**: N/A

---

### CLI Feature: Usage text completeness
**C# behavior**: 18-line usage including: modes, dev-from-source hint, 3 example invocations, "Available tools" grouped by category (Memory, Vector, Graph, Think, Flow, System, Sync, Repair, MCP, Bench).
**Rust behavior**: 8-line usage with only modes and one option. No examples, no tool list, no dev hint.
**Status**: DRIFT
**Severity**: cosmetic
**Notes**: Functional behavior unaffected. MCP clients never see usage text.

---

## Summary Table

| # | Feature | Status | Severity | RTM/TODO |
|---|---------|--------|----------|----------|
| 1 | --mcp watcher auto-start | DRIFT | blocking | RTM-072, RTM-073 / T-10.4 |
| 2 | --mcp resources capability | DRIFT | blocking | RTM-068–071 / T-10.2, T-10.3 |
| 3 | Usage display exit code | DRIFT | blocking | — (new finding) |
| 4 | JSON envelope detection | DRIFT | important | — (new finding) |
| 5 | JSON normalization safety net | DRIFT | important | — (new finding) |
| 6 | Usage text completeness | DRIFT | cosmetic | — (nice-to-have) |
| 7 | --payload optionality | DRIFT (intentional) | cosmetic | T-0.2 (deliberate) |
| 8 | --tool flag | MATCH | — | — |
| 9 | --json flag | MATCH | — | — |
| 10 | --version / --help | MATCH | — | — |
| 11 | Error messages & codes | MATCH | — | — |
| 12 | Tool registry (28/28) | MATCH | — | — |
| 13 | learn flag | MATCH | — | — |
| 14 | Case-insensitive lookup | MATCH | — | — |

---

## Recommendations

### Must Fix (blocking)
1. **Usage exit code** — Change `Ok(usage())` to `Err((usage(), 1))` in `cli.rs` lines 60, 63 to match C# exit-1 behavior.
2. **MCP watcher auto-start** — Already tracked as T-10.4. Implement after T-10.3.
3. **MCP resources** — Already tracked as T-10.2.

### Should Fix (important)
4. **JSON envelope detection** — Add `data` and `error` field checks to `is_json_failure_envelope()` to match C#'s `TryGetJsonEnvelopeSuccess()`.
5. **JSON normalization safety net** — Add a wrapping step in `CliApp::run_inner()` that detects non-envelope tool output in `--json` mode and wraps it in `json::success()`.

### Nice to Have (cosmetic)
6. **Usage text** — Add examples and tool list to match C#.
7. **--payload optionality** — Keep as-is (intentional improvement).

---

## Sources
- C#: `/Users/brett/src/ma-collective/maenifold/src/Program.cs` (250 lines)
- C#: `/Users/brett/src/ma-collective/maenifold/src/Utils/JsonToolResponse.cs` (68 lines)
- C#: `/Users/brett/src/ma-collective/maenifold/src/Tools/ToolRegistry.cs` (463 lines)
- C#: `/Users/brett/src/ma-collective/maenifold/src/Utils/ToolHelpers.cs` (27 lines)
- Rust: `crates/maenifold-core/src/cli.rs` (167 lines)
- Rust: `crates/maenifold-cli/src/main.rs` (4 lines)
- Rust: `crates/maenifold-core/src/json.rs` (136 lines)
- Rust: `crates/maenifold-core/src/registry.rs` (223 lines)
