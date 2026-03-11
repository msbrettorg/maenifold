# CLI Parity Test Report — Help, JSON Mode, Error Envelope

<!-- T-13.8: RTM FR-7, FR-8 | T-1.5: RTM FR-1, FR-2, FR-3 | NFR-1: C# Oracle Parity -->

**Date**: 2025-07-18
**Rust CLI**: `target/debug/maenifold`
**C# CLI**: `../maenifold/src/bin/Debug/net9.0/maenifold`

---

## Summary

| # | Test | Result | Severity |
|---|------|--------|----------|
| 1 | CLI `--help` output | **FAIL** | Medium |
| 2 | JSON success envelope (`MemoryStatus`) | **PASS** | — |
| 3 | JSON error envelope (`ReadMemory` not-found) | **PASS** | — |
| 4 | JSON success envelope (`GetConfig`) | **PASS** | — |
| 5 | No arguments | **FAIL** | Medium |
| 6 | Invalid tool name | **PASS** | — |
| 7 | Missing `--payload` | **FAIL** | High |
| 8 | Invalid JSON payload | **PASS** (structural) | Low (msg text differs) |

**PASS: 5 | FAIL: 3**

---

## Test 1: CLI `--help` output — FAIL

### Exit Codes
| CLI | Exit Code |
|-----|-----------|
| Rust | 0 |
| C# | 1 |

**Difference #1 — Exit code**: Rust returns 0 for `--help`, C# returns 1.

**Difference #2 — Dev source hint** (expected/acceptable):
```
Rust: (For development from source: cargo run -p maenifold-cli -- ...)
C#:   (For development from source: dotnet run -- ...)
```

**Difference #3 — Tool groupings** (3 discrepancies):

| Group | Rust | C# | Issue |
|-------|------|----|-------|
| `Flow:` | `Workflow, AssumptionLedger` | `Workflow` | Rust has extra `AssumptionLedger` |
| `Adopt:` | `Adopt, ListAssets, ReadMcpResource` | *(absent)* | Rust-only group |
| `MCP:` | *(absent)* | `ListAssets, ReadMcpResource` | C#-only group |
| `Assets:` | `UpdateAssets` | *(absent)* | Rust-only group |

### Matching Elements (Correct)
- Header, Usage, Options, Examples sections: identical ✓
- Groups Memory, Vector, Graph, Think, System, Sync, Repair, Bench: identical ✓

---

## Test 2: JSON Success Envelope (`MemoryStatus`) — PASS

Both CLIs produce identical envelope:
```json
{"success":true,"data":"# Memory System Status\n...","error":null}
```
Exit code: 0/0. Field names, order, null handling: exact match.

---

## Test 3: JSON Error Envelope (`ReadMemory` not-found) — PASS

Both CLIs produce character-identical output:
```json
{"success":false,"data":null,"error":{"code":"NOT_FOUND","message":"Memory file not found: nonexistent-xyz"}}
```
Exit code: 1/1. Exact match.

---

## Test 4: JSON Success Envelope (`GetConfig`) — PASS

Both CLIs produce character-identical output. Exit code: 0/0.

---

## Test 5: No Arguments — FAIL

Both print help text (exit 1/1). FAIL due to same content differences as Test 1.

---

## Test 6: Invalid Tool Name — PASS

Plain: `Unknown tool: NonExistentToolXYZ` — exact match, exit 1/1.
JSON: `{"success":false,"data":null,"error":{"code":"UNKNOWN_TOOL","message":"Unknown tool: NonExistentToolXYZ"}}` — exact match.

---

## Test 7: Missing `--payload` — FAIL (HIGH)

| CLI | Behavior | Exit |
|-----|----------|------|
| Rust | **Executes tool** with implicit `{}` payload, returns results | 0 |
| C# | **Rejects invocation**, prints help | 1 |

Rust silently defaults missing `--payload` to `{}`; C# requires it.

---

## Test 8: Invalid JSON Payload — PASS (structural)

Envelope structure, error code `INVALID_PAYLOAD`, message prefix identical.
Parse error detail naturally differs (serde_json vs System.Text.Json). Expected.
Exit code: 1/1.

---

## Findings

| # | Sev | Issue | Fix Target |
|---|-----|-------|------------|
| F-1 | **High** | Missing `--payload` → Rust executes, C# rejects | cli.rs arg parsing |
| F-2 | **Med** | `--help` exit code 0 (Rust) vs 1 (C#) | cli.rs |
| F-3 | **Med** | Group `Adopt:` (Rust) vs `MCP:` (C#) for ListAssets/ReadMcpResource | cli.rs help |
| F-4 | **Med** | `AssumptionLedger` in Rust help, absent in C# | cli.rs help |
| F-5 | **Med** | `UpdateAssets` / `Adopt` in Rust help, absent in C# | cli.rs help |
