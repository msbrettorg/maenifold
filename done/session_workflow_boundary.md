# Test Evaluation: session_workflow_boundary

**File**: `crates/maenifold-core/tests/session_workflow_boundary.rs`
**Evaluator**: Red Team (🔘 Gray — Skeptical Inquiry)
**Traceability**: NFR-2 (boundary layer test coverage) · NFR-3 (no regressions)
**Tests examined**: 6
**Date**: 2025-07

---

## Summary

`session_workflow_boundary.rs` is a small six-test file whose stated purpose is to
verify that the session-persistence contracts for `SequentialThinking`, `Workflow`,
and `AssumptionLedger` hold at the boundary layer. Most tests are load-bearing in
intent, but three systemic defects undercut the entire file:

1. **No traceability**: not one test carries a `T-*` or `RTM-*` comment. Per NFR-2
   and the project's own acceptance criteria, untraced tests are subject to
   rejection.

2. **No panic-safe cleanup**: four tests call `Config::reset_overrides()` at the
   end of their body, without a drop-guard. Any assertion that panics before that
   call leaves `OVERRIDES` pointing at a now-deleted `TempDir`, silently poisoning
   every subsequent test in the binary that calls `Config::current()`.

3. **Global ToolRegistry with per-test path isolation is an illusion**: the
   `isolated_root()` helper redirects where files land, but `ToolRegistry::global()`
   remains a process-global singleton. Parallel test binaries that share this
   singleton (e.g., `cargo nextest`) will race on the global config override.

One test (`registry_exposes_session_and_workflow_tools`) is an outright duplicate.
One test (`sequential_thinking_without_wikilinks_should_fail_validation_contract`)
has a meaningful near-twin in `session_sequential_lifecycle.rs` but survives because
it uniquely validates the structured JSON error contract.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Assertions Correct? | Name Descriptive? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|---------|
| 1 | `registry_exposes_session_and_workflow_tools` | ❌ No | ❌ No | ⚠️ Partial | ✅ Yes | ✅ Yes | **REMOVE** |
| 2 | `config_exposes_expected_session_roots_under_memory` | ⚠️ Partial | ✅ Yes | ❌ No | ✅ Yes | ❌ No | **REWRITE** |
| 3 | `sequential_thinking_session_ids_should_persist_under_dated_sequential_paths` | ✅ Yes | ✅ Yes | ⚠️ Partial | ✅ Yes | ✅ Yes | **KEEP** (with notes) |
| 4 | `workflow_session_ids_should_persist_under_dated_workflow_paths` | ✅ Yes | ✅ Yes | ⚠️ Partial | ✅ Yes | ✅ Yes | **REWRITE** |
| 5 | `assumption_ledger_append_persists_under_assumptions_year_month_path` | ✅ Yes | ✅ Yes | ⚠️ Partial | ✅ Yes | ✅ Yes | **KEEP** (with notes) |
| 6 | `sequential_thinking_without_wikilinks_should_fail_validation_contract` | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | ❌ No | **KEEP** |

---

## Per-Test Analysis

---

### Test 1 — `registry_exposes_session_and_workflow_tools`

**Lines**: 22–38

**What it does**: Calls `ToolRegistry::global()` and asserts that six names are
registered: `"SequentialThinking"`, `"sequentialthinking"`, `"Workflow"`,
`"workflow"`, `"AssumptionLedger"`, `"assumptionledger"`.

**Why it is a duplicate**: `tool_registry_tests.rs::new_registry_registers_all_expected_tool_names`
(line 40 of that file) already:
- Constructs a registry with `ToolRegistry::new()`
- Builds an exhaustive `BTreeSet` of all 28 canonical tool names
- Calls `assert_eq!(actual, expected, ...)` — a strict equality check that would
  catch any missing *or* unexpected registration
- Then verifies `registry.contains(tool_name)` for each canonical name

`session_workflow_boundary` checks only 3 of those 28 tools, with weaker
`assert!(registry.contains(...))` predicates. It adds zero signal. It uses
`ToolRegistry::global()` rather than `ToolRegistry::new()` — that _could_ be
testing global singleton initialization — but the test name makes no such claim,
and global init is covered implicitly by every other test in this file (and many
elsewhere) that calls `ToolRegistry::global()`.

The case-insensitive aliases (`"sequentialthinking"`, `"workflow"`,
`"assumptionledger"`) are covered by
`tool_registry_tests::try_invoke_with_lowercase_alias_dispatches_same_as_canonical`.

**Verdict: REMOVE.** If global-singleton initialization is the real concern, add a
dedicated 3-line test named `global_registry_initializes_successfully` that simply
calls `ToolRegistry::global()` and asserts it is non-empty.

---

### Test 2 — `config_exposes_expected_session_roots_under_memory`

**Lines**: 40–52

**What it does**: Reads `Config::current()` (the **global** config, no
`isolated_root()`) and asserts that three paths end with expected suffixes.

**Problems**:

1. **Unsafe suffix check vs equality**: `ends_with("memory/thinking")` passes for
   `/home/user/my-other-memory/thinking` as well as the intended
   `/home/user/.maenifold/memory/thinking`. An attacker-style mis-configuration
   that happens to share these suffixes would be invisible.

2. **Tests its own inline construction, not the config**: The `workflow_path`
   variable is built by the test itself:
   ```rust
   let workflow_path = config.memory_path.join("thinking").join("workflow");
   assert!(workflow_path.ends_with("memory/thinking/workflow"));
   ```
   There is no `config.workflow_path` field being read. The test is asserting
   that `PathBuf::join` works, not that the config exposes a workflow path. If the
   production code never computes this path, this test cannot catch the omission.

3. **No isolation**: reads the process-global `Config::current()`. Any test that
   calls `Config::override_root()` and then panics will leave this test reading
   a stale TempDir path (or a deleted one). The TEST_MUTEX is not held here,
   meaning concurrent test threads could modify the config while this test runs.

4. **No traceability**.

**Verdict: REWRITE.** The test should:
- Use `isolated_root()` to get a reproducible, isolation-safe config
- Use `assert_eq!` with full path equality (or at minimum
  `ends_with(expected_suffix)` documented as intentional with a comment)
- Verify actual config _fields_ (`config.thinking_path`, `config.sequential_path`,
  and a `config.workflow_path` if it exists) not inline-constructed paths
- Carry a `// T-* RTM-*` traceability comment

---

### Test 3 — `sequential_thinking_session_ids_should_persist_under_dated_sequential_paths`

**Lines**: 54–88

**What it does**: Invokes `SequentialThinking` in an isolated root, extracts the
returned `sessionId`, constructs the expected date-partitioned file path, and
asserts the file exists.

**Strengths**: This is the only test in the project that directly verifies the
_date-partitioned_ (`YYYY/MM/DD/{session_id}.md`) persistence contract for
sequential thinking sessions at the boundary level. If the tool stops writing files,
or writes them in a flat directory, this is the test that catches it.

**Problems**:

1. **No panic-safe cleanup**: `Config::reset_overrides()` is called at the end
   of the test body. A failing `assert!` before it leaves the global config
   pointing at the now-dead `TempDir`. Drop the reset into an RAII guard:
   ```rust
   struct ResetOnDrop;
   impl Drop for ResetOnDrop { fn drop(&mut self) { Config::reset_overrides(); } }
   let _reset = ResetOnDrop;
   ```

2. **Midnight flakiness**: The `now` capture happens _after_ the tool invocation.
   If the test runs at 23:59:59.999 and the file is written at 00:00:00.000 the
   date components diverge and the assertion fails. Capture `now` _before_ calling
   `try_invoke`, or accept a ±1-day window.

3. **Existence-only assertion**: The test only verifies the file was created. An
   empty file, a file with wrong content, or a file with no WikiLinks would pass.
   Adding a `fs::read_to_string` + `contains("session-paths")` check would raise
   the coverage quality.

4. **No traceability comment**.

**Verdict: KEEP**, but address panic-safe cleanup (high priority) and add
a pre-invocation timestamp capture (medium priority).

---

### Test 4 — `workflow_session_ids_should_persist_under_dated_workflow_paths`

**Lines**: 90–124

**What it does**: Invokes the `Workflow` tool in an isolated root, extracts the
session ID from the first line of markdown output, constructs the expected
date-partitioned path, and asserts the file exists.

**Unique coverage**: The only test verifying date-partitioned file persistence for
workflow sessions at the boundary layer. Worth keeping in principle.

**Problems**:

1. **Brittle output parsing** (high severity):
   ```rust
   let session_id = output
       .lines()
       .next()
       .and_then(|line| line.strip_prefix("Created session: "))
       .expect("session line");
   ```
   This panics (not fails) if the tool's output format ever changes. "Panic vs
   fail" matters: a panic surfaces as `SIGABRT` in some test runners, not a clean
   red test. Worse, the `expect` message gives no hint about what the first line
   actually was. If `Workflow` gains a log line before the session ID output, this
   silently starts extracting wrong data. Fix: parse the JSON response (the tool
   supports `OutputFormat::Json`), or search all lines for the session-ID prefix,
   or expose a typed result from the tool.

2. **Inconsistent output format with test 3**: Test 3 uses `OutputFormat::Json`,
   test 4 uses `OutputFormat::Markdown`. No comment explains why. If the markdown
   format ever drops the `"Created session: "` prefix, test 4 breaks silently
   while test 3 continues working.

3. **No panic-safe cleanup** (same as test 3).

4. **Midnight flakiness** (same as test 3).

5. **No traceability comment**.

**Verdict: REWRITE.** Switch to `OutputFormat::Json` and extract `sessionId` from
the parsed JSON (as test 3 does). Add panic-safe cleanup and pre-invocation
timestamp capture.

---

### Test 5 — `assumption_ledger_append_persists_under_assumptions_year_month_path`

**Lines**: 126–165

**What it does**: Invokes `AssumptionLedger` with `action: "append"`, extracts
the returned `memory://` URI from the output, validates the URI format, and asserts
the corresponding file exists on disk.

**Strengths**: This is the only test verifying the `YYYY/MM/assumption-*.md`
date-partitioned persistence contract for the assumption ledger. The URI format
assertion is meaningfully specific (`starts_with("memory://assumptions/{year}/{month}/assumption-")`).

**Problems**:

1. **Fragile URI extraction**:
   ```rust
   let uri = text
       .lines()
       .find_map(|line| {
           line.split("memory://")
               .nth(1)
               .map(|rest| format!("memory://{}", rest))
       })
       .expect("memory uri in output");
   ```
   `line.split("memory://").nth(1)` returns the content _between_ the first and
   second `"memory://"` if a line contains two occurrences. It also takes the
   _full rest_ of the line after the first split, which may include trailing
   punctuation (period, comma, right-paren) not part of the URI. A line like
   `"Wrote to memory://assumptions/2025/07/assumption-abc.md."` produces
   `"memory://assumptions/2025/07/assumption-abc.md."` — the trailing period is
   silently included, making the `expected.exists()` check fail on a valid file.

2. **No panic-safe cleanup** (same pattern as tests 3–4).

3. **Year/month only (not day)**: This test does not check the day component of
   the path, unlike tests 3 and 4. That asymmetry is _intentional_ (assumptions
   are partitioned by year/month per the spec), but the comment should say so
   explicitly.

4. **No traceability comment**.

**Verdict: KEEP**, but fix the URI extraction to strip trailing non-URI characters
(use a regex or trim at the first whitespace/punctuation boundary), and add
panic-safe cleanup.

---

### Test 6 — `sequential_thinking_without_wikilinks_should_fail_validation_contract`

**Lines**: 168–191

**What it does**: Invokes `SequentialThinking` with a plain-text response containing
no `[[WikiLinks]]`. Asserts `success == false` and `error.code == "WIKILINK_REQUIRED"`.

**Near-twin in another file**: `session_sequential_lifecycle.rs::new_session_without_concepts_fails_validation`
tests the same scenario but via `OutputFormat::Markdown`, asserting
`result.starts_with("ERROR:")` and `result.contains("Must include [[WikiLinks]]")`.

**Why this test survives the duplicate filter**: It exercises `OutputFormat::Json`
and validates the _structured_ error contract: the `{"success": false, "error": {"code": "WIKILINK_REQUIRED"}}`
JSON shape. The markdown test cannot catch regressions in the JSON error schema.
These are genuinely different contracts.

**Minor issues**:
- No traceability comment.
- No panic-safe cleanup (though the damage is lower here since `isolated_root()`
  sets an override that would need resetting — still applies).
- The `assert_eq!(value["success"], false)` and `assert_eq!(value["error"]["code"], "WIKILINK_REQUIRED")`
  assertions are correct and specific. No redundancy. No over-assertion.

**Verdict: KEEP.** Add a `// Complements session_sequential_lifecycle::new_session_without_concepts_fails_validation`
comment to pre-empt future "this looks like a duplicate" removals.

---

## Recommendations

Listed in priority order:

### P0 — Panic-Safe Config Cleanup (Systemic)

**Affects**: Tests 3, 4, 5, 6

All four tests that call `isolated_root()` must wrap `Config::reset_overrides()`
in a drop guard. Without this, a single panicking assertion poisons the global
`OVERRIDES` for the rest of the test run in the same binary. This is a correctness
defect, not a style preference.

```rust
struct ResetConfig;
impl Drop for ResetConfig {
    fn drop(&mut self) { Config::reset_overrides(); }
}
// Usage at top of each test:
let _guard = TEST_MUTEX.lock();
let (_tmp, config) = isolated_root();
let _reset = ResetConfig; // MUST come after isolated_root()
```

### P1 — Add Traceability Comments (Systemic)

**Affects**: All 6 tests

Per NFR-2 and the project traceability rules, every test must reference at least
one `T-*` item or `RTM-*` requirement. Suggested mappings:

| Test | Suggested traceability |
|------|------------------------|
| 1 (after REMOVE) | — |
| 2 | `// T-0.x: RTM NFR-2 (boundary layer)` |
| 3 | `// T-SESSION-BOUNDARY.3: RTM NFR-2` |
| 4 | `// T-SESSION-BOUNDARY.4: RTM NFR-2` |
| 5 | `// T-SESSION-BOUNDARY.5: RTM NFR-2` |
| 6 | `// T-SESSION-BOUNDARY.6: RTM NFR-2` |

### P2 — Remove or Redirect Test 1

`registry_exposes_session_and_workflow_tools` is a strict subset of
`tool_registry_tests::new_registry_registers_all_expected_tool_names`. Remove it.
If there is a real concern about `ToolRegistry::global()` initialization
specifically, add a one-line smoke test in `tool_registry_tests.rs`.

### P3 — Rewrite Test 4 Output Parsing

Replace `OutputFormat::Markdown` + fragile `strip_prefix("Created session: ")`
parsing with `OutputFormat::Json` + `value["data"]["sessionId"].as_str()`, mirroring
the approach in test 3. This makes the two date-persistence tests structurally
parallel and resilient to output-format changes.

### P4 — Rewrite Test 2 with Isolation and Equality

Replace `Config::current()` with `isolated_root()`, replace `ends_with()` with
`assert_eq!`, and remove the inline `workflow_path` construction in favour of
reading whichever config field actually stores that path (or adding one if absent).

### P5 — Pre-Timestamp for Midnight Safety (Tests 3, 4, 5)

Capture `chrono::Utc::now()` _before_ calling `try_invoke`, not after.
Add a comment explaining the midnight-boundary risk.

### P6 — Fix URI Extraction in Test 5

Trim trailing non-URI characters from the extracted URI before comparison and
path construction. A simple approach:

```rust
let uri_raw = /* extracted string */;
let uri = uri_raw.trim_end_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '_' && c != '/');
```

### P7 — Add Content Assertions to Tests 3–5

File existence is necessary but not sufficient. Add a `fs::read_to_string`
assertion confirming that the file contains at least one `[[WikiLink]]` and the
key input data, ensuring the tool wrote the correct session content.

---

## Load-Bearing Score: 4/6

| Test | Load-Bearing? | Reason |
|------|:---:|--------|
| 1 `registry_exposes_session_and_workflow_tools` | ❌ | Full duplicate of `tool_registry_tests::new_registry_registers_all_expected_tool_names` |
| 2 `config_exposes_expected_session_roots_under_memory` | ⚠️ 0.5 | Weak assertions on global config; tests its own inline logic, not a real config field |
| 3 `sequential_thinking_...persist_under_dated_sequential_paths` | ✅ | Only test verifying date-partitioned file persistence for SequentialThinking |
| 4 `workflow_...persist_under_dated_workflow_paths` | ✅ | Only test verifying date-partitioned file persistence for Workflow |
| 5 `assumption_ledger_...under_assumptions_year_month_path` | ✅ | Only test verifying date-partitioned file persistence for AssumptionLedger |
| 6 `sequential_thinking_without_wikilinks_should_fail_validation_contract` | ✅ | Only test verifying the JSON structured error contract for WikiLink rejection |

**Score: 4.5 / 6** (counting test 2 as 0.5).

The _concept_ this file covers is load-bearing. The _execution_ needs hardening on
panic safety, traceability, and output-format robustness before these tests can be
trusted to catch real regressions without producing spurious ones.

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- session_workflow_boundary`: 6 passed, 0 failed
- Verdict: PASS
