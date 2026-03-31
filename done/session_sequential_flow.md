---
id: session_sequential_flow
status: done
---

# Test Evaluation: session_sequential_flow.rs

**Traceability**: T-12.7 | NFR-1 (C# Oracle Parity) | NFR-2 (Test Coverage)  
**Reviewer**: Red Team — 🔘 Gray (Skeptical Inquiry)  
**File**: `crates/maenifold-core/tests/session_sequential_flow.rs`  
**Tests evaluated**: 5

---

## Summary

`session_sequential_flow.rs` covers five distinct behaviors of the `SequentialThinking` tool:
three positive flow variants (revision heading, branch heading, `needsMoreThoughts` expansion)
and two error/guard paths (nonexistent parent workflow, `parentWorkflowId` on non-first thought).

**All five tests cover behaviors not exercised by any sibling test file.** Cross-referencing
`session_sequential_lifecycle.rs`, `session_sequential_errors.rs`, and
`session_sequential_status.rs` confirms zero duplication of the specific features
(`isRevision`, `branchId`, `needsMoreThoughts`, nonexistent parent, thought-ordering guard)
under test here.

**The file has a coherence problem**: two tests (4, 5) are error/validation guards, not
flow behaviors. They belong in `session_sequential_errors.rs` by the apparent naming
convention for that module. Mixing guard-path tests into a flow file makes triage harder
and obscures the module's intent.

**One test has a structural flaw**: `session_with_needs_more_thoughts_expands_total_thoughts`
only asserts on the *response message*, never on the session file state. The behavioral
contract (what does `needsMoreThoughts=true` actually do to the document?) is untested.

**One test has a silent setup assumption**: `session_with_branch_creates_branch_heading`
discards the result of its second `invoke_markdown` call (thought 1, completing the
session). If that call silently fails or errors, the setup state is wrong but the test
will still pass because the assertion never verifies thought 1 was actually persisted.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|---------|
| 1 | `session_with_revision_creates_revision_heading` | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | Low | **KEEP** |
| 2 | `session_with_branch_creates_branch_heading` | ✅ Yes | ✅ Yes | ⚠️ Partial | ✅ Yes | Low | **REWRITE** |
| 3 | `session_with_needs_more_thoughts_expands_total_thoughts` | ✅ Yes | ✅ Yes | ❌ Incomplete | ✅ Yes | Low | **REWRITE** |
| 4 | `new_session_with_invalid_parent_workflow_returns_error` | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | Low | **MERGE** |
| 5 | `continue_session_with_parent_workflow_on_thought_2_returns_error` | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | Low | **MERGE** |

---

## Per-Test Analysis

### Test 1 — `session_with_revision_creates_revision_heading`

**What it does**: Creates a session, then submits a second thought with `isRevision: true`
and `revisesThought: 0`. Asserts the response message confirms the thought was added and
that the persisted session file contains the string `"(Revises: 0)"`.

**Load-bearing**: Yes. This is the sole test for the `isRevision`/`revisesThought` feature
path. No other test file exercises it. Removal leaves zero coverage of revision heading
rendering.

**Unique**: Confirmed via cross-reference — `isRevision` appears nowhere else in the test
corpus.

**Assertions correct**: Yes. The response-message check (`"Added thought 1 to session:"`)
and the file-content check (`"(Revises: 0)"`) together verify both the tool output and the
persisted document. These are the two things that matter for this behavior.

**Name**: Accurate and unambiguous.

**Verdict**: ✅ **KEEP** — clean, minimal, correct.

---

### Test 2 — `session_with_branch_creates_branch_heading`

**What it does**: Creates a session (thought 0), completes it (thought 1 with conclusion),
then creates a branch from thought 0 with `branchId: "alternative-analysis"`. Asserts
the branch call returns `"Created session:"` and that the *original* session's content
contains `"(Branch: alternative-analysis)"`.

**Load-bearing**: Yes. This is the only test for the positive branch-creation flow and
parent-session annotation. `session_sequential_errors.rs` tests the *missing-branchId*
error path. Neither covers the positive case.

**Unique**: Confirmed — `branchId: "alternative-analysis"` and `"(Branch: alternative-analysis)"`
appear nowhere else.

**Assertions — problems**:

1. **Silent setup failure**: The second `invoke_markdown` call (completing the session
   with a conclusion) has its return value discarded. In Rust the call still executes, but
   if it returns an error message the harness won't panic, and the branch will be created
   against a session that is only partially set up. The test will pass with wrong state.
   Fix: bind the result and assert `!result.starts_with("ERROR:")`.

2. **Branch session not verified**: The test confirms `"Created session:"` in
   `branch_result` but never reads the *new* branch session URI to verify its own content.
   If branching creates a new file (as the response implies), that file's content is
   entirely unexamined — `"(Branch: alternative-analysis)"` is only checked in the parent.
   This leaves the new session's structure unvalidated.

3. **Branching from a completed session**: The test completes the parent before branching.
   Whether branching from a completed session is intended (or should be guarded) is
   unspecified. The test implicitly asserts it is allowed, but there is no comment or
   separate error test asserting it is *not* allowed from an already-cancelled session.
   This is an implicit behavioral assumption with no documented intent.

**Name**: Good.

**Verdict**: ⚠️ **REWRITE** — bind and assert the second `invoke_markdown` result; add
assertions on the newly-created branch session's content.

---

### Test 3 — `session_with_needs_more_thoughts_expands_total_thoughts`

**What it does**: Creates a session with `totalThoughts: 2`, submits thought 1 with
`needsMoreThoughts: true`. Asserts the response contains `"Continue with thought 2/?"` and
`"extending beyond initial estimate"`.

**Load-bearing**: Yes — `needsMoreThoughts` appears in no other test file. Removal means
zero coverage of this parameter.

**Unique**: Confirmed.

**Assertions — problems**:

1. **Only the response message is checked**. The test never reads the session file. If
   `needsMoreThoughts=true` should update the document (e.g., annotate the thought with
   an open-ended count, change frontmatter, or leave `totalThoughts` as `null`/`?`) that
   state is completely unverified. The feature could silently stop writing to the file and
   this test would still pass.

2. **The "2/?" format is a string snapshot**. If the output format changes to
   `"thought 2 of (expanding)"` or similar, the test breaks without any semantic content
   change. A more durable assertion would check for the session ID in the response and
   then verify the session file's state.

3. **`thoughtCount` is not checked**. After `needsMoreThoughts=true` the session should
   have recorded 2 thoughts. The test never verifies this, nor verifies the session's
   `status` remains `"active"`.

**Name**: Good — "expands_total_thoughts" is accurate.

**Verdict**: ❌ **REWRITE** — after asserting the response message, read the session file
and assert: `status == "active"`, `thoughtCount == "2"`, thought content is present, and
the thought heading reflects the open-ended count (e.g., `"Thought 1/?"`).

---

### Test 4 — `new_session_with_invalid_parent_workflow_returns_error`

**What it does**: Attempts to create a session with `parentWorkflowId` set to a
hard-coded nonexistent ID (`"nonexistent-workflow-12345"`). Asserts the response starts
with `"ERROR:"` and contains `"not found"`.

**Load-bearing**: Yes — the positive parent-workflow path is in `session_sequential_lifecycle.rs`
and the closed-parent guard is in `session_sequential_status.rs`. The *nonexistent parent*
guard is only tested here.

**Unique**: Confirmed.

**Assertions correct**: Yes. `starts_with("ERROR:")` verifies the tool surface contract;
`contains("not found")` verifies the error identifies the right problem. Adequate for a
guard test.

**Name**: Good.

**Placement problem**: This is a guard/validation test, not a flow test. By the naming
convention of the test module group, this belongs in `session_sequential_errors.rs` alongside
`continue_session_with_nonexistent_session_id_returns_error` and
`continue_session_with_invalid_session_id_returns_error`. Keeping it in `_flow.rs` breaks
the organizational contract.

**Verdict**: 🔀 **MERGE** into `session_sequential_errors.rs`. No logic changes needed.

---

### Test 5 — `continue_session_with_parent_workflow_on_thought_2_returns_error`

**What it does**: Creates a session, then submits thought 1 with a valid `parentWorkflowId`.
Asserts the response starts with `"ERROR:"` and contains the exact string
`"Parent workflow can only be set on first thought"`.

**Load-bearing**: Yes — this tests a specific input-ordering invariant. No other test
enforces that `parentWorkflowId` is only valid on the first thought.

**Unique**: Confirmed.

**Assertions correct**: Mostly. `starts_with("ERROR:")` and `contains("Parent workflow can
only be set on first thought")` together make a strong assertion. The concern is that the
second assertion is brittle against exact error message text changes — but given this is
an internal tool with controlled message strings, this is acceptable. It is better to be
specific here than to accept any error.

**Name**: Excellent — fully describes the scenario without ambiguity.

**Placement problem**: Same as Test 4. This is a guard/error test, not a flow test.
It belongs in `session_sequential_errors.rs`.

**Verdict**: 🔀 **MERGE** into `session_sequential_errors.rs`. No logic changes needed.

---

## Recommendations

### R-1 (High): Move Tests 4 and 5 to `session_sequential_errors.rs`

`session_sequential_flow.rs` should contain only tests that exercise the positive
structural variants of the flow (revision, branching, expansion). Error-path tests for
input validation belong in `session_sequential_errors.rs`. The current mixing degrades
the diagnostic signal: a failure in the errors file now hides whether the flow or a
guard is broken.

**Action**: Move `new_session_with_invalid_parent_workflow_returns_error` and
`continue_session_with_parent_workflow_on_thought_2_returns_error` to
`session_sequential_errors.rs`.

### R-2 (High): Rewrite Test 3 to assert on session file state

`session_with_needs_more_thoughts_expands_total_thoughts` only confirms the response
message. It does not verify that `needsMoreThoughts=true` actually changes anything about
the persisted document. Add post-invocation assertions:

```rust
let uri = session_uri("sequential", &session_id);
let (frontmatter, content) = harness.read(&uri);
assert_eq!(frontmatter_string(&frontmatter, "status"), "active");
assert_eq!(frontmatter_string(&frontmatter, "thoughtCount"), "2");
assert!(content.contains("Thought 1/?"));
```

### R-3 (Medium): Bind and assert the discarded result in Test 2

The unchecked `invoke_markdown` call for thought 1 (completing the session) in
`session_with_branch_creates_branch_heading` creates a silent failure mode. The setup
can silently fail while the test continues to pass.

```rust
let complete_result = harness.invoke_markdown("SequentialThinking", json!({ ... }));
assert!(!complete_result.starts_with("ERROR:"), "unexpected error completing session: {complete_result}");
```

### R-4 (Medium): Add branch session content verification in Test 2

`session_with_branch_creates_branch_heading` confirms the parent is annotated but never
inspects the newly-created branch session. Extract the new session ID from `branch_result`
and verify the branch session file's content and frontmatter (e.g., `type == "sequential"`,
`status == "active"`, thought 0 content present).

### R-5 (Low): Clarify branch-from-completed-session intent

The test branches from a session that has already been completed (thought 1 has
`nextThoughtNeeded: false` and `conclusion`). If this is intentional (completed sessions
can be branched), add a comment stating the intent. If it is unintentional, revise to
branch from an active session, or add a separate test to assert that branching from a
completed session is correctly handled (allowed or rejected, whichever is correct).

---

## Load-Bearing Score: 5/5

All five tests cover behaviors not exercised elsewhere in the session sequential test
corpus. Removal of any single test would create a genuine gap in behavioral coverage.
However, the quality of those tests is uneven:

- **Full coverage (assertion-complete)**: Tests 1, 4, 5 — correct and verifiable as written  
- **Partial coverage (assertion-incomplete)**: Test 3 — covers the call path, misses state verification  
- **Structurally unsound setup**: Test 2 — covers behavior but has a silent-failure risk in setup

**Effective quality-adjusted score: 3.5/5**

The file earns credit for covering unique ground. It loses credit for a missing-state
assertion in T3, a silent setup risk in T2, and two tests misplaced in the wrong module
by the file-naming convention of this test corpus.

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- session_sequential_flow`: 5 passed, 0 failed
- Verdict: PASS
