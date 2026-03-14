# Test Evaluation: session_sequential_errors.rs

> **Traceability**: T-12.7 · NFR-2 · NFR-3
> **Reviewer mode**: 🔘 Gray — Skeptical Inquiry
> **Scope**: `crates/maenifold-core/tests/session_sequential_errors.rs` (5 `#[test]` functions)
> **Comparators consulted**: `session_sequential_lifecycle.rs`, `session_sequential_status.rs`,
> `session_sequential_flow.rs`, `thinking/session.rs`, `thinking/validation.rs`

---

## Summary

The file is named `session_sequential_errors.rs`, signalling it covers failure/rejection paths for
the `SequentialThinking` tool. **It does not.** Two of the five tests are unqualified happy-path
tests with no error assertion whatsoever; they were either written here by accident or are refugees
from an earlier flat-file era before the suite was split. Three tests legitimately cover error
paths, but one of those carries an assertion that tests static implementation wording rather than
observable contract.

The structural problem is worse than it first appears: the four sibling files
(`_lifecycle`, `_status`, `_flow`, `_errors`) were presumably carved out to give each concern its
own home. When happy-path tests leak into the errors file, the split provides false organisation —
a reader searching for lifecycle coverage will not find the `analysisType` assertion, and a reader
auditing error coverage will wade through unrelated lifecycle noise.

**Net assessment**: the file is 60 % correctly positioned (3/5 genuine error tests), delivers one
unique happy-path assertion that must not be lost (`analysisType` frontmatter field — untested
anywhere else in the suite), and carries one fragile wording assertion that couples a test to a
sentence in `validation.rs`.

---

## Verdicts

| # | Test name | Belongs here? | Load-bearing? | Unique? | Correct assertions? | Good name? | Parameterisable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|:---:|---------|
| 1 | `complete_session_lifecycle_create_continue_complete_succeeds` | ❌ No | ✅ Partially | ⚠️ Partially | ✅ Yes | ❌ No (mismatch with file) | — | **MERGE** |
| 2 | `new_session_extracts_concepts_correctly` | ❌ No | ⚠️ Weakly | ❌ No | ⚠️ Imprecise | ✅ Yes | — | **MERGE** |
| 3 | `continue_session_with_nonexistent_session_id_returns_error` | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | **KEEP** |
| 4 | `continue_session_with_invalid_session_id_returns_error` | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | **KEEP** |
| 5 | `session_with_branch_from_thought_without_branch_id_returns_error` | ✅ Yes | ✅ Yes | ✅ Yes | ⚠️ One brittle assertion | ✅ Yes | — | **KEEP** (with assertion fix) |

---

## Per-Test Analysis

### Test 1 — `complete_session_lifecycle_create_continue_complete_succeeds`

**What it does**: Drives a 3-step session (create → continue → complete) and makes 11 assertions
across frontmatter and content.

**Belongs here?** No. Zero error assertions. The test name even says "succeeds". This is a
lifecycle test that was placed in the errors file.

**Load-bearing?** Partially. Most of its surface is covered by adjacent files:
- `session_sequential_lifecycle.rs::continue_session_with_valid_session_id_appends_thought`
  covers 2-step completion, `status=completed`, `thoughtCount=2`, content headings.
- `session_sequential_status.rs::session_with_conclusion_completes_session` covers
  `frontmatter.contains_key("completed")`, `thoughtCount`, `Conclusion` section.

However, **one assertion is unique across the entire session test suite**:
```rust
assert_eq!(frontmatter_string(&frontmatter, "analysisType"), "architecture");
```
No other test exercises `analysisType` propagation to frontmatter. If this test is deleted without
migration, that coverage gap is silent — `cargo test` stays green while the field could rot.

**Correct assertions?** Yes. All 11 are well-scoped and verify observable state.

**Good name?** The name describes the behaviour accurately but does not belong in this file.
It should live in `session_sequential_lifecycle.rs`.

**Recommendation**: Extract into `session_sequential_lifecycle.rs` as
`complete_three_step_session_preserves_analysis_type_in_frontmatter`. The `analysisType`
assertion is the load-bearing payload; the rest of the lifecycle exercise can be trimmed to avoid
redundancy with `continue_session_with_valid_session_id_appends_thought`.

---

### Test 2 — `new_session_extracts_concepts_correctly`

**What it does**: Creates a session whose `response` has three WikiLinks and whose ambient
`thoughts` field has three more, then checks the resulting file for two bare concept names and the
`Thoughts:` section header.

**Belongs here?** No. No error assertion; the word "correctly" in the name signals a positive
outcome test.

**Load-bearing?** Weakly.
- `session_sequential_lifecycle.rs::new_session_with_concepts_creates_session_file` already
  checks `content.contains("test thought with [[sequential-thinking]]")` — i.e. that raw response
  text appears verbatim with brackets intact.
- `workflow_progress.rs` already checks `content.contains("Thoughts:")` for the ambient thoughts
  section heading.
- The assertions `content.contains("machine-learning")` and `content.contains("AI")` will both
  pass simply because the raw Markdown contains `[[machine-learning]]` and `[[AI]]`; they do not
  isolate concept extraction from raw text storage. They are not actually testing what the test
  name claims.

**Correct assertions?** Imprecise. The test claims to verify "extracts concepts correctly" but:
```rust
assert!(content.contains("machine-learning")); // passes if [[machine-learning]] is in content
assert!(content.contains("AI"));               // same — does not prove extraction, only presence
```
A true concept-extraction test would assert on the graph DB row or on the frontmatter concept
list, not on bare substring matching of the raw content.

**Good name?** The name makes a claim the assertions cannot support.

**Recommendation**: Merge with `session_sequential_lifecycle.rs`. Rename to something honest,
e.g. `new_session_ambient_thoughts_appear_under_thoughts_heading`. Drop the misleading
"extracts concepts" framing unless assertions are upgraded to verify graph/frontmatter state.

---

### Test 3 — `continue_session_with_nonexistent_session_id_returns_error`

**What it does**: Passes a syntactically valid session ID (`session-9999999999999-12345`) that
resolves to a future-dated path no file will occupy, expects the error
`"Session … missing. Start with thoughtNumber=0."`.

**Belongs here?** Yes.

**Load-bearing?** Yes. No other test in the session suite exercises the "valid format, file absent"
code path in `thinking/session.rs:49`.

**Unique?** Yes. `workflow_supervisor_registration.rs` reuses `session-9999999999999-00001` but
for a different tool and different assertion.

**Correct assertions?**
```rust
assert!(result.starts_with("ERROR:"));
assert!(result.contains("missing"));
```
Both are correct and minimal. `"missing"` aligns with the implementation text
`"Session {} missing. Start with thoughtNumber=0."`. Appropriately loose — tests the
contract word, not the full sentence.

**Parameterisable?** Yes: could be a row in a table-driven test alongside test 4 (both are
"given malformed/absent sessionId → ERROR: containing substring"). Would save ~10 lines with
`rstest` or inline `for` loop. Optional improvement.

**Verdict**: **KEEP**.

---

### Test 4 — `continue_session_with_invalid_session_id_returns_error`

**What it does**: Passes `"session-not-a-timestamp"` — a string whose second dash-segment is not
a parseable millisecond epoch — and expects `"Invalid sessionId format"`.

**Belongs here?** Yes.

**Load-bearing?** Yes. Exclusively exercises the format-validation branch in
`thinking/session.rs:40`: `"Invalid sessionId format: {}"`. This is a distinct code path from the
"valid format, file missing" path in test 3.

**Unique?** Yes. No other test in the session suite hits the format-validation branch.

**Correct assertions?**
```rust
assert!(result.starts_with("ERROR:"));
assert!(result.contains("Invalid sessionId format"));
```
Exact and correct. The substring matches the implementation string without over-constraining the
full sentence (the `{}` interpolation value is not asserted, which is right).

**The inline comment adds real value**:
```rust
// T-14.10: RTM FR-16 - suffix-free legacy IDs are valid, so keep this case truly malformed.
```
This explains *why* the chosen value is the way it is — a future author cannot simplify it without
re-reading FR-16. Keep the comment.

**Parameterisable?** Yes: table-driven with test 3. Optional.

**Verdict**: **KEEP**.

---

### Test 5 — `session_with_branch_from_thought_without_branch_id_returns_error`

**What it does**: Creates a session whose first thought contains `[[multi-agent-coordination]]`,
then submits a `branchFromThought: 0` call with `branchId: null`. Expects two error substrings.

**Belongs here?** Yes.

**Load-bearing?** Yes. The only test exercising the `branchFromThought`-without-`branchId`
validation in `thinking/validation.rs:120`. `session_sequential_flow.rs` tests the *success* path
(`session_with_branch_creates_branch_heading`); this covers the *failure* path.

**Unique?** Yes.

**Correct assertions?**
```rust
assert!(result.starts_with("ERROR:"));
assert!(result.contains("branchId required"));       // ✅ tests the contract
assert!(result.contains("multi-agent coordination")); // ⚠️ tests implementation wording
```
The third assertion is load-bearing only as long as `validation.rs:120` reads exactly
`"branchId required when branchFromThought is specified for multi-agent coordination"`.
That sentence is a complete unit; the phrase "multi-agent coordination" is static, not derived
from test data (the WikiLink `[[multi-agent-coordination]]` in the test input is coincidental
naming, not causal). However, the assertion is still **coupling a test to prose** — if the team
rewrites the error message to be more concise (e.g., `"branchId is required with branchFromThought"`),
this test breaks without any functional regression. The `"branchId required"` assertion alone
fully establishes the observable contract. The second substring is noise.

**Good name?** Yes — precise and self-documenting.

**Verdict**: **KEEP**. Remove or relax the `multi-agent coordination` assertion:
```rust
// Before (fragile):
assert!(result.contains("multi-agent coordination"));

// After (contract-only):
// assertion removed — "branchId required" fully covers the rejection contract
```

---

## Recommendations

Ordered by impact:

1. **[High]** Move test 1 to `session_sequential_lifecycle.rs`. Rename it
   `complete_three_step_session_preserves_analysis_type_in_frontmatter`. Preserve the
   `analysisType` assertion — it is the only coverage of that field anywhere in the suite. Trim
   assertions already covered by `continue_session_with_valid_session_id_appends_thought` to
   avoid duplication (drop the redundant `status`, `type`, `completed`, content-heading checks
   that are already asserted in the adjacent lifecycle tests).

2. **[Medium]** Move test 2 to `session_sequential_lifecycle.rs`. Rename it
   `new_session_ambient_thoughts_field_appears_under_thoughts_heading`. Replace the imprecise
   concept-name substrings with the actual observable contract (e.g. assert on the section label
   rather than claiming concept extraction is proven by substring match).

3. **[Low]** Remove the `result.contains("multi-agent coordination")` assertion from test 5.
   The `"branchId required"` check is sufficient and more durable.

4. **[Optional]** Merge tests 3 and 4 into a parameterised table: `(session_id_input,
   expected_error_substring)`. With `rstest` this collapses two identical test bodies into one
   and makes adding new invalid-format cases trivial.

5. **[Hygiene]** After moving tests 1 and 2, this file will contain exactly 3 error tests — all
   correctly placed. At that point the file name accurately describes its content.

---

## Load-Bearing Score: 3/5

| Test | Properly placed | Unique coverage | Net load-bearing |
|------|:---:|:---:|:---:|
| 1 `complete_session_lifecycle…` | ❌ wrong file | ⚠️ `analysisType` only | Partial — must migrate, not delete |
| 2 `new_session_extracts_concepts…` | ❌ wrong file | ❌ covered elsewhere | No — assertions are redundant/imprecise |
| 3 `continue_session_with_nonexistent…` | ✅ | ✅ | **Yes** |
| 4 `continue_session_with_invalid…` | ✅ | ✅ | **Yes** |
| 5 `session_with_branch_from_thought…` | ✅ | ✅ | **Yes** |

Three tests (`3`, `4`, `5`) are correctly positioned, uniquely load-bearing error tests — 3/5.
Tests `1` and `2` carry some value that must be *migrated*, not discarded, before removal.
