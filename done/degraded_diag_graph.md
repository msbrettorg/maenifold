---
id: degraded_diag_graph
status: done
---

# Test Evaluation: degraded_diag_graph

> **Reviewer perspective:** 🔘 Gray — Skeptical Inquiry
> **Source file:** `crates/maenifold-core/tests/degraded_diag_graph.rs`
> **Traceability:** T-0.1 (ported C# oracle ACs), NFR-2 (test coverage), NFR-1 (oracle parity)
> **Production code under test:** `src/graph/build_context.rs`, `src/util/json.rs`
> **Platform gate:** `#![cfg(unix)]` (all tests; required by `capture_stderr` FD redirect)

---

## Summary

This file contains three tests that verify the **degraded-diagnostic contract** — the
obligation that, when something fails internally, the system (a) continues to return
useful partial output rather than aborting and (b) emits a structured
`[DEGRADED][Component][Method]` tag to stderr so operators can detect the condition.

All three tests share the same `TEST_MUTEX` guard and the `capture_stderr` helper from
`support.rs`. The mutex is **not optional** for any of these tests: `capture_stderr`
redirects the process-level stderr file descriptor using raw `dup`/`dup2` syscalls, which
is inherently single-threaded state. Serialisation is architecturally required.

The tests are labelled `T-0.1: RTM FR-25.2`, which is a ported acceptance criterion from
the C# oracle. `T-0.1` does not appear as an open item in the current `TODO.md`; it is
either a completed-sprint reference (see `CHANGELOG.md`) or a legacy task ID. This
traceability gap is minor but worth noting.

Two tests exercise `BuildContext` via a real SQLite database. One tests the pure-function
`get_string_array` utility. Coverage of the degradation surface is thin: only three
triggerable paths exist in the two production files, and all three are tested. However,
`get_string_array` has silent `None` return paths (for `null`, `number`, `object`,
`boolean` inputs) that do **not** emit diagnostics; these are undocumented design
decisions that could confuse future readers.

---

## Verdicts

| Test name | Load-bearing? | Unique? | Assertions correct? | Name quality | Parameterisable? | Verdict |
|---|:---:|:---:|:---:|:---:|:---:|:---:|
| `build_context_emits_degraded_on_unreadable_file` | ✅ Yes | ✅ Yes | ⚠️ Partial — weak positive assertion | ✅ Good | ➖ Low value | **KEEP** |
| `enrich_with_community_data_emits_degraded_on_missing_table` | ✅ Yes | ✅ Yes | ⚠️ Partial — DirectRelations unchecked; scenario mislabelled | ❌ Misleading | ⚠️ Worth extending | **REWRITE** |
| `payload_reader_emits_degraded_on_malformed_json_string` | ⚠️ Moderate | ✅ Yes | ✅ Yes | ⚠️ Slightly imprecise | ➖ No new diagnostics | **KEEP** |

---

## Per-Test Analysis

### `build_context_emits_degraded_on_unreadable_file`

**Scenario:** Two concepts share an edge. The file `memory://ctx-a` exists in
`file_content` with a body of three raw bytes `[0xFF, 0xFE, 0xFD]` — invalid UTF-8.
`BuildContext` is called with `includeContent: true`, forcing the read path.

**What it probes:**
- `build_context.rs` must not crash or return `Err` when a content blob is unreadable.
- `DirectRelations` must still be populated (the graph is not affected by content failure).
- The `[DEGRADED][GraphTools][BuildContext]` tag must appear on stderr.

**Assertion audit (3 assertions):**

| # | Assertion | Necessary? | Correct? |
|---|-----------|:---:|:---:|
| 1 | `result.is_ok()` (via `.expect()`) | ✅ | ✅ — confirms graceful continuation |
| 2 | `!direct.is_empty()` | ✅ | ⚠️ — verifies *something* returned but not *what*; does not assert `ctx-b` is present |
| 3 | `stderr.contains("[DEGRADED][GraphTools][BuildContext]")` | ✅ | ✅ — confirms the diagnostic contract |

**Concerns:**

Assertion 2 is weaker than it should be. `!direct.is_empty()` passes even if the
`DirectRelations` array contained entirely unexpected entries. A tighter assertion —
checking that the relation to `ctx-b` is present by name — would catch a regression where
the graph query silently returned different results under the degraded path.

No assertion checks that the central concept name (`ctx-a`) is preserved in the output
envelope. Nor does it verify the file path `memory://ctx-a` appears in the diagnostic
message, which would confirm the *right* file was flagged, not a coincidental one.

**Verdict: KEEP** — load-bearing, unique, and correctly structured. Strengthen the
positive-result assertion before refactoring the content-loading path.

---

### `enrich_with_community_data_emits_degraded_on_missing_table`

**Scenario:** Two concepts share an edge. The `concept_communities` table is **dropped and
recreated** with a reduced schema: only `concept_name TEXT PRIMARY KEY` and
`detected_at TEXT NOT NULL` — missing whatever columns the production code's schema
validation expects. `BuildContext` is then called.

**What it probes:**
- `GraphDatabase::initialize_database` detects a schema mismatch and returns an error
  mentioning `"concept_communities"` or `"community_id"`.
- `build_context.rs:48` catches that error, emits the `[DEGRADED][GraphTools][EnrichWithCommunityData]`
  diagnostic, and continues.
- The output JSON still contains the concept name.

**Assertion audit (3 assertions):**

| # | Assertion | Necessary? | Correct? |
|---|-----------|:---:|:---:|
| 1 | `result.is_ok()` (via `.expect()`) | ✅ | ✅ |
| 2 | `parsed["data"]["ConceptName"] == "community-a"` | ✅ | ✅ — confirms the concept identity survives |
| 3 | `stderr.contains("[DEGRADED][GraphTools][EnrichWithCommunityData]")` | ✅ | ✅ |

**Concerns:**

**The test name is wrong.** The test title says `_on_missing_table`, but the table is not
missing — it is deliberately recreated with fewer columns. A truly missing table (DROP
with no subsequent CREATE) would behave differently: `GraphDatabase::initialize_database`
might silently recreate the table via `CREATE TABLE IF NOT EXISTS`, meaning the
`[DEGRADED]` path would *never* fire. The test currently does not cover a missing-table
scenario; it covers an *incompatible-schema* scenario. This distinction matters for
maintaining the test when `initialize_database` internals change.

**DirectRelations is not asserted.** Unlike test 1, this test does not verify that graph
neighbours survive the community-enrichment failure. The production code opens a separate
connection and loads the graph *after* the degraded init block, so the graph should be
unaffected — but this is not asserted. A future refactor could accidentally break that
guarantee silently.

**The schema manipulation is clever but fragile.** The test reconstructs
`concept_communities` with specific columns that differ from the production schema. If
the production schema changes, the test must be updated to ensure it still produces an
incompatible variant. A comment documenting the *intended* mismatch would help.

**Verdict: REWRITE** — fix the name to reflect the schema-mismatch scenario being
tested, add an assertion that `DirectRelations` is still populated, and add a brief
comment explaining why the schema is intentionally incomplete.

---

### `payload_reader_emits_degraded_on_malformed_json_string`

**Scenario:** A JSON payload contains `"items": "this-is-not-a-json-array"`. The
`get_string_array` utility is called requesting the `items` key.

**What it probes:**
- The `Value::String` arm of `get_string_array` attempts to deserialise the string as
  `Vec<String>` via `from_str_safe`. Failure emits `[DEGRADED][PayloadReader][GetStringArray]`
  and returns `None`.
- This is the **only codepath** in `get_string_array` that emits a diagnostic.

**Assertion audit (2 assertions):**

| # | Assertion | Necessary? | Correct? |
|---|-----------|:---:|:---:|
| 1 | `result.is_none()` | ✅ | ✅ |
| 2 | `stderr.contains("[DEGRADED][PayloadReader][GetStringArray]")` | ✅ | ✅ |

**Concerns:**

**The name is slightly imprecise.** The input `"this-is-not-a-json-array"` is perfectly
valid JSON — it is a JSON string, not malformed JSON. The failure is a *type mismatch* at
the semantic level: a string was provided where a JSON-array-of-strings was expected. A
more accurate name would be `payload_reader_emits_degraded_on_non_array_string_value`.

**Silent `None` paths are undocumented.** Inspecting `json.rs:69`: the `_ =>` arm returns
`None` for `null`, `number`, `bool`, and `object` inputs without emitting any diagnostic.
This is a deliberate design decision (matching C# oracle behaviour per the source
comment), but no test documents this intentional silence, and no comment in production
code explains it. A future contributor could reasonably add diagnostics to the `_` arm
and unknowingly break the C# parity contract. At minimum, a comment should explain the
intentional asymmetry.

**The `TEST_MUTEX` is correct but unexplained.** This test has no database and is a
pure-function call. A reader might wonder why it holds the global mutex. The reason
is that `capture_stderr` redirects the process-level stderr FD, which is shared across
all threads — concurrent use would produce interleaved captures. The mutex is structurally
necessary; a comment would prevent it from being removed as "unnecessary overhead."

**Verdict: KEEP** — the test correctly targets the only diagnostic-emitting codepath.
Rename for precision, and add a comment documenting the silent `_` arm.

---

## Positive Observations

- **Dual assertion discipline.** Every test checks *both* (a) the operation still returns
  useful output and (b) the diagnostic tag fires. This two-part contract is the right shape
  for a degraded-path test and should be maintained in any extension.

- **`capture_stderr` is a reusable, safe primitive.** The FD-redirect approach is the
  correct strategy for testing stderr diagnostics in an in-process Rust test without
  introducing external infrastructure. The SAFETY comments are adequate.

- **`#![cfg(unix)]` gate is correct.** Raw `dup`/`dup2` are not portable. Scoping the
  entire file to Unix prevents false failures on non-Unix CI targets.

- **Isolation is correct.** Each test creates its own `TempDir`-backed database via
  `TestEnv::setup()` and the `Drop` impl resets global config. There is no shared state
  between tests beyond the mutex.

---

## Recommendations

**Immediate (before next schema refactor):**

1. **REWRITE** `enrich_with_community_data_emits_degraded_on_missing_table`:
   - Rename to `enrich_with_community_data_emits_degraded_on_incompatible_schema`.
   - Add `assert!(!direct.is_empty(), "graph relations must survive community schema failure")`.
   - Add a comment explaining which expected columns are absent and why that triggers the diagnostic.

2. **Strengthen** `build_context_emits_degraded_on_unreadable_file`:
   - Replace `assert!(!direct.is_empty(), …)` with an assertion that `ctx-b` specifically
     appears in `DirectRelations` by name.

**Low priority / documentation:**

3. **Document** the silent `None` paths in `get_string_array` (`json.rs:69`):
   - Add a comment to the `_ =>` arm: `// Deliberate: non-string/non-array inputs degrade silently (C# oracle parity).`
   - Add a corresponding `// intentional: no diagnostic emitted for non-string inputs` comment in the test.

4. **Add missing-table sibling test** alongside the schema-mismatch test:
   - Drop `concept_communities` with no recreation.
   - Verify `initialize_database` either (a) recreates the table and succeeds normally,
     OR (b) also triggers the `[DEGRADED]` path.
   - This distinguishes the two scenarios and closes a gap in the stated coverage intent.

5. **Resolve stale traceability reference.** `T-0.1` does not appear in `TODO.md`.
   If completed, it should be cross-referenced to the relevant `CHANGELOG.md` entry.
   If still open, add it to `TODO.md` under the relevant sprint.

---

## Load-Bearing Score: 3/3

All three tests verify observable contracts that would not be caught by any other test in
the suite. Each test is the only test of its specific degradation path. Removing any of
the three would leave a gap in the degraded-diagnostic regression net.

The suite is **complete for the diagnostic paths that currently exist** but
**sparse for the degradation surface as a whole** — there are likely additional failure
modes in `build_context.rs` (e.g., unreadable file with `includeContent: false`, a
corrupted graph edge, `load_graph_neighbors` failure) that have no corresponding degraded
diagnostic test. This is an NFR-2 gap, not a fault with the existing three tests.
