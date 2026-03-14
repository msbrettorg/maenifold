# Test Evaluation: markdown_boundary_redteam
**File:** `crates/maenifold-core/tests/markdown_boundary_redteam.rs`
**Test count:** 3
**Evaluated:** 2025-07-18
**Traceability:** T-W1-2 (FR-28.9 / RTM-183 per file annotations — see findings below)

---

## Summary

This file is thin. Three tests claim adversarial red-team coverage for the markdown boundary but deliver substantially less than the name implies. One test has a **trivially-passing negative assertion** — the code it is supposedly guarding against never existed. Two tests exercise the same single code branch in `validate_uri`, meaning one is redundant by construction. Both traversal tests share a `red5` numbering prefix, a naming collision that suggests fragmented extraction from a larger file.

The file's stated threat model — filesystem path information disclosure in tool output, and path traversal prevention in URIs — is legitimate. The *execution* of that threat model is weak: the information-disclosure test guards against a leak the implementation never performs, and the two traversal tests cover a single `any()` predicate with identical outcomes.

Additionally, **the traceability tag is wrong on all three tests**. All three are annotated `T-W1-2: RTM FR-28.9`. FR-28.9 in RTM is `TC-28.9: WikiLink validation failure prevents persistence; workflow pauses` — an ACP orchestration requirement. The actual requirement these tests exercise is **RTM-183 / FR-29 (`path traversal attempts denied`)** and **RTM-171 (`fs/read_text_file denies outside boundaries`)** in spirit, though `validate_uri` belongs to the markdown layer, not the ACP permission layer. The mislabelling is non-trivial: if RTM-183 traceability is tracked automatically, these tests do not appear in the matrix.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `fr28_9_red1_concept_repair_tool_error_does_not_leak_filesystem_path` | REWRITE | Trivially-passing negative assertion; wrong RTM tag; overlaps existing test; positive assertion has three unrelated OR branches. |
| `fr28_9_red5_validate_uri_traversal_single_dotdot_segment_throws` | KEEP (rename + retag) | Unique: only direct unit test of `validate_uri` for traversal. Correct assertion. Needs renaming (red5 collision) and correct RTM tag. |
| `fr28_9_red5_validate_uri_traversal_double_dotdot_segment_throws` | MERGE | Exercises zero additional code paths over the single-dotdot test. Same `.any()` predicate exits on first match. Fold into test 2 as a parameterized input. |

---

## Detailed Findings

### Test 1 — `fr28_9_red1_concept_repair_tool_error_does_not_leak_filesystem_path`

**What it claims:** When `RepairConcepts` receives an invalid folder, the output must not contain the filesystem memory path.

**What it actually tests:** Nothing the code doesn't already enforce by construction.

**The key question: can the memory path ever appear in this output?** No. Tracing the call stack:

```
repair_concepts() in repair/api.rs
  → folder_path = context.config.memory_path.join(relative_folder)
  → if !folder_path.exists() {
        return Err(MaenifoldError::Message(format!("Directory not found: {relative_folder}")));
    }
```

The error message uses `relative_folder` — the bare folder name as supplied by the caller (`"invalid-nonexistent-folder-12345"`). The `memory_path` is joined to construct the existence check but is **never interpolated into the error string**. The negative assertion `!output.contains(env.cfg.memory_path)` passes trivially regardless of whether any path-leak protection exists, because the code never performs the disclosure it is supposedly preventing.

**The positive assertion is also weak:**
```rust
assert!(
    output.contains("ERROR")
        || output.contains("Scanning 0 markdown files")
        || output.contains("Files scanned: 0"),
    ...
);
```
These three branches describe three mutually exclusive code paths:
- `"ERROR"` — the error path (what actually fires here)
- `"Scanning 0 markdown files"` — the pipeline path when the folder is valid but empty
- `"Files scanned: 0"` — the summary line of a completed scan

The OR makes the assertion pass for any of these outcomes. A test that accepts three distinct behavioral outcomes as equivalent correct results is not testing a behavior — it is testing that `try_invoke` does not panic.

**Overlap:** `repair_concepts_replacement.rs::repair_concepts_dry_run_with_invalid_folder_returns_error_output` tests the same scenario and asserts `output.contains("ERROR") && output.contains("Directory not found")` — stricter positive assertions, no trivially-passing negative assertion, same path.

**Traceability:** Tagged `T-W1-2: RTM FR-28.9`. FR-28.9 is `WikiLink validation failure prevents persistence; workflow pauses` (ACP orchestration). The real requirement being exercised is closer to **RTM-183** (path traversal denied) or an information-disclosure sub-requirement that does not currently appear in the RTM.

**Verdict: REWRITE.** If path-leak prevention in `RepairConcepts` error output is a genuine security requirement, register it in the RTM and write a test that constructs a scenario where the memory path *could* plausibly appear (e.g., via `io::Error` propagation or debug formatting). If no such scenario exists, remove the test and add a code comment on the error format confirming the deliberate use of `relative_folder` over `folder_path`.

---

### Test 2 — `fr28_9_red5_validate_uri_traversal_single_dotdot_segment_throws`

**What it claims:** A URI with a single `..` segment (`memory://foo/../bar`) must be rejected with an `InvalidArgument` containing "path traversal".

**What it actually tests:** Correct. The implementation splits on `/` and calls `.any(|segment| segment == "..")`, which fires on the first `..` segment. This test is the only direct unit test of `validate_uri` itself; all traversal tests in `markdown_boundary_validation.rs` invoke `validate_uri` indirectly through `read_markdown`, `file_exists`, `delete_file`, etc.

**Is it load-bearing?** Yes. If `validate_uri` is refactored (e.g., the split-and-any pattern replaced with a regex or prefix check), this test provides the fastest signal that the core predicate broke. The indirect tests in the validation file would also fail, but this one names the contract precisely.

**Issues:**
- Name suffix `red5` collides with test 3 (both are `red5`). Numbering gap implies `red2`, `red3`, `red4` existed in a parent file and were not ported.
- Tagged `T-W1-2: RTM FR-28.9` — same mislabelling as test 1. Should reference RTM-183.
- No `isolated_env()` setup — correct, since `validate_uri` is a pure function with no filesystem side effects. Not an issue, noted as intentional.

**Verdict: KEEP.** Fix the name (assign distinct number, e.g., `red2`), fix the RTM tag to `RTM-183`.

---

### Test 3 — `fr28_9_red5_validate_uri_traversal_double_dotdot_segment_throws`

**What it claims:** A URI with a double `..` traversal (`memory://foo/../../bar`) must be rejected.

**What it actually tests:** The same `.any()` predicate in the same branch of `validate_uri` as test 2. The implementation:

```rust
if relative_part.split('/').any(|segment| segment == "..") { ... }
```

`.any()` short-circuits on the first `..`. For `foo/../../bar`, the iterator visits `["foo", "..", "..", "bar"]`; after the second element the predicate returns true and evaluation stops. The third `..` element is never visited. Test 3 exercises **zero additional code paths** over test 2.

**Name collision:** Both test 2 and test 3 are named with the `red5` prefix. In a test runner output, the only distinguishing information is the suffix (`single_dotdot` vs `double_dotdot`). If tests are filtered by prefix, both may be included or excluded together — unintentionally.

**Verdict: MERGE into test 2.** Replace the two separate functions with a single parameterized assertion (or inline both URIs into one test that checks the same property for several inputs):

```rust
#[test]
fn validate_uri_rejects_dotdot_traversal_segments() {
    for uri in [
        "memory://foo/../bar",           // single traversal
        "memory://foo/../../bar",        // double traversal
        "memory://../escape",            // leading traversal
        "memory://deep/path/../../../escape", // deep traversal
    ] {
        let err = markdown::validate_uri(uri, "uri")
            .unwrap_err_or_else(|| panic!("expected traversal rejection for: {uri}"));
        assert_invalid_argument(err, "path traversal");
    }
}
```

---

## Undetected Gaps (Missing Red-Team Coverage)

The following attack vectors are not covered by any test in this file, and some are not covered anywhere in the test suite:

| Attack Vector | Covered Elsewhere? | Risk |
|---|---|---|
| URL-encoded traversal: `memory://%2e%2e/escape` | No — `validate_uri` splits on `/` and checks for literal `..`. `%2e%2e` is not decoded before the check. | **HIGH** — `validate_uri` does NOT reject percent-encoded traversal. If a caller performs URL decoding before `uri_to_path`, the check is bypassed. |
| Mixed encoding: `memory://%2e./escape` | No | HIGH — same bypass path as above |
| Null byte injection: `memory://foo\0/../bar` | No | MEDIUM — OS path functions truncate at null; may escape sanitization |
| Windows separator: `memory://foo\..\bar` | No — covered elsewhere for sanitize_user_input | LOW for memory URIs (URI layer is `/`-only), but worth asserting explicitly |
| Empty relative part: `memory://` | Not in redteam file | LOW — `validate_uri` accepts this (no trailing segment to check) |
| Single dot: `memory://./foo` | Not in this file | LOW — `.` is not `..`, no traversal, but unexpected behavior possible |

The most actionable finding is the **percent-encoding gap**. `validate_uri` operates on the raw string; if any upstream caller URL-decodes the URI before validation (or if an MCP transport layer normalizes percent-encoding), `%2e%2e` traversal bypasses the check. This should be confirmed against the HTTP transport path introduced in FR-23 (T-17.2), where URIs may arrive from HTTP request bodies and could be processed by URL-decoding middleware before reaching `validate_uri`.

---

## Recommendations

1. **REWRITE test 1** (`fr28_9_red1_...`): Either (a) remove it and annotate the implementation comment in `repair/api.rs` explaining that `relative_folder` (not `memory_path`) is deliberately used in error formatting, or (b) if path-leak prevention is a genuine requirement, add an RTM entry and write a test that targets a realistic disclosure vector (e.g., `io::Error` formatting of a path, debug output, or panic messages). Do not test a negative property that cannot be violated by the current code.

2. **RENAME + RETAG test 2** (`fr28_9_red5_single_dotdot_...`): Assign a non-colliding suffix (e.g., `red2`). Change traceability tag from `T-W1-2: RTM FR-28.9` to `RTM-183`.

3. **MERGE test 3 into test 2** (`fr28_9_red5_double_dotdot_...`): Combine into a single parameterized traversal test covering at minimum: single `..`, double `..`, leading `..`, and a deep multi-hop traversal path.

4. **Add a test for percent-encoded traversal** against `validate_uri`. Confirm whether `memory://%2e%2e/escape` is accepted or rejected. If accepted, this is a vulnerability when the HTTP transport is active (FR-23). If there is no decode step anywhere in the call chain, document the assumption explicitly in `validate_uri`'s source.

5. **Fix RTM traceability across all three tests.** The annotation `T-W1-2: RTM FR-28.9` is incorrect. The traversal tests belong under RTM-183 (`path traversal attempts denied`). The path-leak test, if kept, should reference a new RTM entry for information disclosure prevention in tool error output.

---

## Load-Bearing Score: 1/3

Only test 2 is genuinely load-bearing. It is the sole direct unit test of `validate_uri`'s traversal predicate and would catch a regression in the core check before any of the indirect tests surface the failure. Test 1 has a trivially-passing negative assertion and overlaps a stronger test in another file. Test 3 covers an identical code path to test 2 and adds no independent failure detection.

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- markdown_boundary_redteam`: 2 passed, 1 failed
- Failures:
  - `fr28_9_red1_concept_repair_tool_error_does_not_leak_filesystem_path`: panicked at `support.rs:133` — `invoke_repair_concepts` calls `try_invoke(...).expect(...)` which unwraps `Err(Message("Directory not found: invalid-nonexistent-folder-12345"))`. The test never reaches its assertions because the support helper assumes `try_invoke` returns `Ok(error_text)` when the tool actually returns `Err(MaenifoldError)` for an invalid folder. This confirms the red-team's REWRITE verdict — the test is not merely trivially-passing; it is outright broken at the call-site level.
- Passing tests:
  - `fr28_9_red5_validate_uri_traversal_single_dotdot_segment_throws`: ✅ correctly rejects `memory://foo/../bar` with `InvalidArgument` containing "path traversal"
  - `fr28_9_red5_validate_uri_traversal_double_dotdot_segment_throws`: ✅ correctly rejects `memory://foo/../../bar` with same error
- Verdict: **FAIL**
