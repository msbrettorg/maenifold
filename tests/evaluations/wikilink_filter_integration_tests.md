# Test Evaluation: wikilink_filter_integration_tests

**File:** `crates/maenifold-core/tests/wikilink_filter_integration_tests.rs`
**Test count:** 5
**Evaluated:** 2026-07-14
**Traceability:** T-2.4 (RTM FR-11.2 / NFR-11.2.1 / NFR-11.2.2 / NFR-11.2.3)

---

## Summary

This file occupies a specific and necessary layer: it exercises `memory::write_memory` and `memory::edit_memory` end-to-end with the WikiLink filter active — something neither the unit tests in `wikilink_filter_unit_tests.rs` (which test `WikiLinkFilter` in isolation) nor the sync tests in `sync_wikilink_filter.rs` (which test the sync engine, not the memory write API) cover. The 5 tests are justified at their layer.

The structural foundation is solid: `TEST_MUTEX` prevents shared-state races, `isolated_env()` correctly initialises a temp dir + DB, and `Drop` cleans up global config overrides. No test inherits leaked state.

The weakness is **assertion depth**. Three of the five tests have assertions that are *necessary* but not *sufficient*: they verify the error path was taken but do not verify the error content precisely enough to catch a regression in what the error reports. Two tests would benefit from rewriting to add the missing assertions. One test would benefit from being split into a parameterized pair (or a companion test added) to cover the JSON mode gap on `edit_memory`.

**Cross-layer position:** These tests are the *only* coverage of `memory::write_memory` and `memory::edit_memory` with the filter active. The inline unit tests in `src/memory/support.rs` cover `render_filtered_wikilinks_error` directly, but that is a rendering function, not the full API path. This file is load-bearing at its layer and should not be deleted.

**Assumptions questioned:**
- *"Testing ERROR and 'WikiLinks that are excluded' is enough."* — No. These string fragments only verify the error code path was entered. They do not verify that the correct *concept names* appear in the error, meaning a regression that swaps concept names would go undetected.
- *"Five tests is the right number."* — Partially. There is a missing test for `edit_memory` in JSON output mode (test 4 only covers markdown mode), and no test for prefix-pattern filtering through the write/edit API (only exact-match scenarios).

---

## Verdicts

| Test Function | Verdict | Load-Bearing | Reason |
|---|---|---|---|
| `write_memory_with_filtered_concept_returns_error` | **REWRITE** | Yes | Checks "ERROR" and "WikiLinks that are excluded" but does not verify `[[tool]]` appears in the output (concept name), nor that no file was written to disk. A bug that reports the wrong concept name would pass. |
| `write_memory_mixed_filtered_and_valid_still_blocked` | **KEEP** | Yes | Only test that verifies *multiple* filtered concept names appear in the error output (`[[tool]]`, `[[agent]]`). Correctly asserts the operation does not succeed. Missing: assert `[[react]]` is NOT in the error (react is unfiltered). |
| `write_memory_without_filter_file_allows_all` | **KEEP** | Yes | Only integration test for fail-open behavior on the write_memory path. Correctly asserts success. Could also verify a checksum is returned, but the current assertion is sufficient. |
| `edit_memory_with_filtered_concept_returns_error` | **KEEP** | Yes | **Only test that covers `edit_memory` with the filter active.** Without this test, a regression breaking the filter check in `editing.rs` would be invisible. Markdown mode only — JSON mode gap exists (see Recommendations). |
| `write_memory_filtered_concept_json_mode_returns_structured_error` | **REWRITE** | Partial | Verifies `success: false` and `error.code: "FILTERED_WIKILINKS"`, but does not assert `error.details.blocked` — the array of concept names and reasons. A bug that populates `blocked` with wrong concepts or an empty list would pass. The unit test in `support.rs` covers the rendering function; this test should cover the full wired path with specific concept content. |

---

## Detailed Findings

### Test 1: `write_memory_with_filtered_concept_returns_error`

**What it exercises:** `memory::write_memory` (markdown mode) → `WikiLinkFilter::try_check` → `render_filtered_wikilinks_error` with `[[tool]]` filtered and `[[react]]` not.

**Assertion analysis:**
```rust
assert!(result.contains("ERROR"), ...);
assert!(result.contains("WikiLinks that are excluded"), ...);
```

Both assertions verify the *error branch was entered* but neither verifies what concepts were reported. The actual rendered error message includes `[[tool]]: Generic hub - too connected` — the test does not assert this. A regression that renders the wrong concept name, an empty list, or swaps the reason would pass this test.

**Missing assertion:** `assert!(result.contains("[[tool]]"), ...);`  
**Missing assertion:** `assert!(!result.starts_with("Created memory FILE:"), ...);` (test 2 covers this, but test 1 should too — they are independent tests).

**Verdict: REWRITE** — add concept name assertion and file-not-created guard.

---

### Test 2: `write_memory_mixed_filtered_and_valid_still_blocked`

**What it exercises:** Same path as test 1, but with *two* filtered concepts (`tool`, `agent`) and one unfiltered (`react`). Verifies both filtered names appear in the error.

**Assertion analysis:**
```rust
assert!(result.contains("ERROR"), ...);
assert!(result.contains("[[tool]]"), ...);
assert!(result.contains("[[agent]]"), ...);
assert!(!result.starts_with("Created memory FILE:"), ...);
```

Strong set. The missing assertion is the complement: `[[react]]` should NOT appear in the error (it is not filtered). Without this, a bug that reports *all* concepts — including unfiltered ones — would pass.

**Missing assertion:** `assert!(!result.contains("[[react]]"), "unfiltered concept must not appear in error: {result}");`

**Verdict: KEEP** (assertion set is good enough to be load-bearing; the missing complement assertion is a hardening opportunity, not a blocking defect).

---

### Test 3: `write_memory_without_filter_file_allows_all`

**What it exercises:** `remove_filter_file` → `memory::write_memory` with concepts that would otherwise be filtered (`[[tool]]`, `[[react]]`). Verifies the write succeeds.

**Assertion analysis:**
```rust
assert!(result.starts_with("Created memory FILE:"), ...);
```

Correct and sufficient. The `starts_with("Created memory FILE:")` guard verifies the success path was taken. The folder is specified (`wikilink-filter-integration-tests`) ensuring the file would be placed in the expected location.

**No defects.** Could additionally assert that the returned URI is well-formed or that the file exists on disk, but the current assertion is load-bearing.

**Verdict: KEEP**

---

### Test 4: `edit_memory_with_filtered_concept_returns_error`

**What it exercises:** Writes a file without filter → adds filter → edits with `[[tool]]` (now filtered) → verifies edit returns error.

**Assertion analysis:**
```rust
assert!(edit_result.contains("ERROR"), ...);
assert!(edit_result.contains("WikiLinks that are excluded"), ...);
```

Same weakness as test 1: verifies the error branch but not the concept name in the output. However, this test is the *only coverage* of `edit_memory` through the filter — without it, a bug in `editing.rs`'s filter check (the code at `if let Err(blocked) = ...` around line 133 of `editing.rs`) would go completely undetected.

**Coverage gap:** JSON output mode is not tested for `edit_memory`. Test 5 covers JSON mode for `write_memory`, but there is no corresponding test for `edit_memory` in JSON mode. The rendering function is shared (`render_filtered_wikilinks_error`), so a JSON-mode regression would be caught at the unit level — but the wiring in `editing.rs` would not be exercised.

**Verdict: KEEP** — sole coverage of the edit path. Consider a companion `edit_memory_filtered_concept_json_mode_returns_structured_error` test.

---

### Test 5: `write_memory_filtered_concept_json_mode_returns_structured_error`

**What it exercises:** `memory::write_memory` (JSON mode) → filter → JSON error structure.

**Assertion analysis:**
```rust
let parsed: Value = serde_json::from_str(&result).expect("json result");
assert_eq!(parsed["success"], false, ...);
assert_eq!(parsed["error"]["code"], "FILTERED_WIKILINKS", ...);
```

Parses the JSON and checks the top-level structure. Does NOT check `parsed["error"]["details"]["blocked"]`. This means a bug that returns `"blocked": []` (empty array), or `"blocked": [{"concept": "wrong", "reason": "wrong"}]`, would pass. The inline unit test in `src/memory/support.rs` (`filtered_wikilinks_json_error_includes_code_and_details`) does assert `blocked` structure — but that test operates on the *rendering function* in isolation, not the full wired path.

**Missing assertion:**
```rust
let blocked = parsed["error"]["details"]["blocked"].as_array().expect("blocked array");
assert_eq!(blocked.len(), 1, "exactly one filtered concept: {result}");
assert_eq!(blocked[0]["concept"], "tool", "filtered concept name: {result}");
```

**Verdict: REWRITE** — add `blocked` array assertion.

---

## Coverage Gaps (Not Existing Tests)

These scenarios have no test in this file and no integration-layer coverage elsewhere:

| Gap | Risk | Recommendation |
|---|---|---|
| `edit_memory` with JSON output mode + filter | Medium — the wiring in `editing.rs` is not exercised in JSON mode | Add `edit_memory_filtered_concept_json_mode_returns_structured_error` |
| `write_memory` with prefix-pattern filter (`T-*`) | Low — prefix logic is covered at unit and sync layers; missing at memory API layer | Add one prefix-pattern test through `write_memory` |
| File non-existence after blocked `write_memory` | Low — test 2 partially covers this but tests 1, 4, 5 do not | Strengthen tests 1, 4, 5 to assert no file was created |

---

## Recommendations

1. **Rewrite test 1**: Add `assert!(result.contains("[[tool]]"))` and `assert!(!result.starts_with("Created memory FILE:"))`.

2. **Rewrite test 5**: Add assertion on `error.details.blocked` array — verify concept name and non-empty list:
   ```rust
   let blocked = parsed["error"]["details"]["blocked"].as_array().expect("blocked array");
   assert!(!blocked.is_empty(), "blocked list must not be empty: {result}");
   assert_eq!(blocked[0]["concept"], "tool", "must name the filtered concept: {result}");
   ```

3. **Add companion test for test 4**: `edit_memory_filtered_concept_json_mode_returns_structured_error` — mirrors test 5 but exercises `edit_memory`. The wiring in `editing.rs` is distinct from `access.rs`.

4. **Test 2 hardening** (optional): Add `assert!(!result.contains("[[react]]"))` to verify unfiltered concepts are absent from the error.

5. **Do not remove any test.** All five are load-bearing at the memory API integration layer, which is not covered by the unit tests or sync tests.

---

## Load-Bearing Score: 4/5

Tests 1 and 5 have assertion weaknesses that would allow specific regressions to pass undetected. Tests 2, 3, and 4 are sufficiently load-bearing. The overall file is correctly positioned in the test hierarchy and should not be deleted.
