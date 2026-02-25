# TODO

## Verified status (2026-02-08)

- Runtime: `.venv/bin/python` → Python 3.12.12
- Tests: `./.venv/bin/python -m pytest -v` ✅ 142 passed (99 unit + 43 CLI smoke)
- CLI Parity: 5/5 tests byte-identical with C# output
- Cross-compatibility: Python reads C# SQLite DB and vice versa ✅
- Types: `./.venv/bin/python -m mypy src` ❌ FAIL (51 errors / 16 files) — tracked as T-007

## Backlog (T-*)

### T-006: Fix MCP RepairConcepts kwarg mismatch

- **Status**: ✅ COMPLETE (2026-02-08)
- **Fix**: Renamed `create_wiki_links` → `create_wikilinks` in `mcp_server.py` (lines 443, 478)

### T-007: Establish mypy baseline (fix or configure)

- **Traceability**: TBD
- **Status**: ⏳ PENDING
- **Why**: mypy currently reports 51 errors (metaclass typing in `config.py`, generics in `safe_json.py`, bytes/str typing in watcher/search tools, missing stubs for PyYAML, etc.).
- **Acceptance**:
  - Either (A) `mypy src` passes cleanly, or (B) `mypy` is configured so failures represent actionable, high-signal issues (documented exceptions only).

### T-008: Add --json structured output support

- **Status**: ✅ COMPLETE (2026-02-08)
- **Fix**: Added `--json` flag to CLI argparse, `_json_error()` helper in `main.py`, handles `UNKNOWN_TOOL` and `INVALID_PAYLOAD` error codes matching C# `JsonToolResponse`

### T-009: Improve missing-param error messages

- **Status**: ✅ COMPLETE (2026-02-08)
- **Fix**: Added `KeyError` handler in `ToolRegistry.try_invoke()` that produces `"Required property 'X' is missing from payload."` matching C# `PayloadReader.GetString()`

---

## Legacy notes (unverified; likely stale)

The content below predates the 2026-01-28 verification above and may reference non-Python artifacts (e.g., C# Program.cs).

### CLI Parity Status: 5/5 tests passing ✅

| Test | Status | Issue |
|------|--------|-------|
| Test 1 (Help) | ✅ PASS | - |
| Test 2 (Invalid tool) | ✅ PASS | - |
| Test 3 (Bad JSON) | ✅ PASS | Fixed C# exception handling |
| Test 4 (Missing param) | ✅ PASS | Fixed C# exception handling |
| Test 5 (Valid op) | ✅ PASS | - |

### TEST-MATRIX.md Status: 153/153 tests passing (100%)

| Category | Pass | Fail | Total |
|----------|------|------|-------|
| Memory Operations | 22 | 1 | 23 |
| Graph Operations | 15 | 0 | 15 |
| Search Operations | 9 | 0 | 9 |
| System Tools | 10 | 0 | 10 |
| Session Tools | 21 | 0 | 21 |
| Asset + Repair | 16 | 0 | 16 |
| RAG + Failures + Scripts | 17 | 1 | 18 |
| Red Team | 27+ | 0 | 30 |

**Remaining Issue**: None. Shell escaping handled via @file and @- payload support.

---

### TASK-001: Fix C# Exception Handling

**Status**: ✅ COMPLETE

Fixed C# Program.cs to handle JsonException and ArgumentException gracefully.
All 5 CLI parity tests now pass.

---

### TASK-002: Red Team Validation

**Status**: ✅ COMPLETE

- Tested all edge cases (empty, null, array, deeply nested, unicode, SQL injection)
- No stack traces leak
- Exit codes consistent
- Discovered and fixed RT-SEC-009 (triple-dot path traversal)

---

### TASK-003: Blue Team Validation

**Status**: ✅ COMPLETE

Security review passed. No information disclosure or DoS vectors found.

---

### TASK-004: Implement UpdateAssets Tool

**Status**: ✅ COMPLETE

Implemented full AssetManager functionality with SHA256 hash comparison, dry_run support, and formatted output.

---

### TASK-005: Wire RepairConcepts to Registry

**Status**: ✅ COMPLETE

RepairConcepts was already implemented in repair_tools.py - fixed tool_registry.py to call it properly.

---

## Completion Criteria

### CLI Parity (5 tests) - ✅ COMPLETE

All 5 tests produce identical output and exit codes.

### TEST-MATRIX.md (153 tests) - 99% COMPLETE

152/153 tests passing. Remaining:
- 1 shell escaping test harness issue (not code bug)

### Final Gate

Output `<promise>ALLCOMPLETE</promise>` only when:
- All 5 CLI parity tests pass ✅
- All 153 TEST-MATRIX tests pass (currently 152/153)
- Database cross-compatibility verified ✅
