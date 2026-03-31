---
id: frontmatter_write
status: done
pipeline: tdd
---

# Test Evaluation: frontmatter_write.rs

**File**: `crates/maenifold-core/tests/frontmatter_write.rs`
**Evaluator**: 🔘 Gray — Skeptical Inquiry
**Task traceability**: T-12.6 / T-META-001.6 (file header); FR-24.x (stale — see Finding A)
**Peer file reviewed for overlap**: `markdown_rw_write_tests.rs` (9 tests, T-12.7.1)

---

## Summary

Five tests. All five exercise `write_markdown` frontmatter envelope behavior: timestamps injected
on create, `created` preserved on rewrite, lifecycle fields not clobbered, raw envelope format.
The file was introduced as a split from C# oracle parity regressions (T-META-001.6, T-12.6).

**Central skeptical finding**: `markdown_rw_write_tests.rs` already covers four of these five
scenarios, and covers one of them *more rigorously* than this file does. The only assertion that
does not exist anywhere else in the suite is a single line in `AC4` — `raw_text.starts_with("---")`.
This means the file contributes **one net-new behavioural assertion** across 155 lines of test code.

**Traceability defect**: The header comments reference `RTM FR-24.x` for "frontmatter metadata
regression coverage." The current RTM's FR-24 is **Entra ID Authentication** (JWT validation,
OIDC, RFC 9728 — RTM-131 through RTM-136). These tests have nothing to do with Entra ID. The
reference is either from a renumbered sprint or was copied without update. `T-META-001.6` does
not appear in `TODO.md`, indicating this work is complete but the tests carry stale identifiers.

---

## Verdicts

| # | Test name | Load-bearing? | Unique coverage? | Correct assertions? | Good name? | Parameterisable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| 1 | `fr24_1_ac1_write_markdown_new_file_has_created_and_modified` | ⚠️ Partial | ❌ No | ✅ Yes | ✅ Yes | — | **REMOVE** |
| 2 | `fr24_1_ac2_write_markdown_existing_created_preserved_on_rewrite` | ✅ Yes | ❌ No | ⚠️ Weak | ✅ Yes | — | **REWRITE** |
| 3 | `fr24_1_ac3_write_markdown_missing_created_added_on_rewrite` | ✅ Yes | ❌ No | ✅ Yes | ✅ Yes | — | **REMOVE** |
| 4 | `fr24_1_ac4_write_markdown_null_frontmatter_dict_created` | ✅ Yes | ⚠️ One line | ✅ Yes | ❌ No | — | **REWRITE** |
| 5 | `fr24_1_ac5_write_markdown_lifecycle_timestamps_not_overwritten` | ✅ Yes | ❌ No | ✅ Yes | ✅ Yes | — | **REMOVE** |

---

## Per-Test Analysis

### Test 1 — `fr24_1_ac1_write_markdown_new_file_has_created_and_modified`

**What it does**: Writes a new file with `None` frontmatter, asserts `created` and `modified` keys
exist and are non-empty strings.

**Duplicate**: `markdown_rw_write_tests.rs::write_markdown_null_frontmatter_creates_envelope_with_created_and_modified`
tests the exact same scenario and makes the same key-existence assertions. AC1 adds a
`!value.is_empty()` guard — but that guard also appears in AC4, which tests the identical
null-frontmatter scenario with *more* assertions (raw envelope format check).

**Conclusion**: AC1 is strictly dominated by both AC4 (same file) and its counterpart in
`markdown_rw_write_tests.rs`. Running it provides zero information that the other two do not.

**Verdict**: **REMOVE**

---

### Test 2 — `fr24_1_ac2_write_markdown_existing_created_preserved_on_rewrite`

**What it does**: Writes a file, reads the generated `created` timestamp, waits 10ms, rewrites
with that timestamp passed back explicitly, asserts `created` is unchanged and `modified` is
non-empty.

**Duplicate**: `markdown_rw_write_tests.rs::write_markdown_existing_created_preserves_created_and_updates_modified`
tests the same invariant, but with a **hardcoded** past timestamp (`"2020-01-01T00:00:00…"`).

**Assertion quality defect**: AC2 only asserts `modified` is non-empty. It does **not** assert that
`modified` changed — i.e., it would pass even if `write_markdown` wrote the same timestamp twice.
The `markdown_rw_write_tests.rs` counterpart adds the critical assertion:

```rust
assert_ne!(
    scalar(&read_frontmatter, "modified").as_deref(),
    Some(original_created)
);
```

This makes AC2's assertion surface strictly weaker than what already exists in the suite. The
dynamic read-and-re-pass pattern adds 10+ lines of setup noise without catching any failure mode
that the hardcoded-timestamp version would miss.

**Verdict**: **REWRITE** — if retained, replace the dynamic setup with a hardcoded past timestamp
(simpler, more readable) and add `assert_ne!(modified_after, original_created)` to verify the
timestamp was actually advanced. Alternatively, delete and rely on `markdown_rw_write_tests.rs`.

---

### Test 3 — `fr24_1_ac3_write_markdown_missing_created_added_on_rewrite`

**What it does**: Passes `title` + `type` in frontmatter (no `created`), asserts `created` is
injected on write.

**Duplicate**: `markdown_rw_write_tests.rs::write_markdown_missing_created_adds_created_and_sets_modified`
tests the identical invariant (passes `title` only, no `created`). The behavioral difference
(presence of `type` field in AC3) exercises no distinct code path — the writer's injection logic
does not branch on the `type` field.

**Conclusion**: This test is a straight duplicate. The only variation is a cosmetic difference in
the input dict that has no bearing on the code under test.

**Verdict**: **REMOVE**

---

### Test 4 — `fr24_1_ac4_write_markdown_null_frontmatter_dict_created`

**What it does**: Writes with `None` frontmatter, reads the **raw file bytes**, asserts the file
starts with `---`, then reads structured frontmatter and asserts `created`/`modified` exist.

**Unique assertion (the only one in this file)**: `raw_text.starts_with("---")` — verifies the
physical file on disk has a YAML frontmatter envelope at the top. No other test in `frontmatter_write.rs`
or `markdown_rw_write_tests.rs` makes this assertion against the raw bytes.

**Name problem**: `…_null_frontmatter_dict_created` is misleading. The name implies it tests that
a frontmatter dict is created from a null input, but the *distinguishing* thing this test does is
validate the raw file format (YAML delimiters). The name buries the unique signal.

**Redundancy within the test**: The final six assertions (`frontmatter.contains_key("created")`,
`frontmatter.contains_key("modified")`) duplicate AC1 and the `markdown_rw_write_tests.rs`
counterpart. These could be removed, leaving only the raw-bytes assertion and the round-trip read.

**Verdict**: **REWRITE** — strip the redundant frontmatter-key assertions (covered elsewhere),
rename to reflect the raw envelope format check (e.g., `write_markdown_null_frontmatter_file_has_yaml_envelope`),
and tighten the assertion to the physical format invariant that makes it worth keeping.

---

### Test 5 — `fr24_1_ac5_write_markdown_lifecycle_timestamps_not_overwritten`

**What it does**: Passes `completed` and `cancelled` timestamps plus `type`/`status` fields,
asserts both timestamps survive the write unchanged.

**Duplicate**: `markdown_rw_write_tests.rs::write_markdown_completed_and_cancelled_not_overwritten`
tests the identical invariant with the same assertion pattern (`assert_eq!` on exact timestamp
strings). The only cosmetic variation: AC5 also sets `type` and `status`; the counterpart sets
only `status`. The writer's lifecycle-preservation logic is field-name-driven, not
`type`/`status`-conditional, so the extra fields in AC5 do not exercise any new branch.

**Conclusion**: Full semantic duplicate. No unique failure mode is detectable by AC5 that
the `markdown_rw_write_tests.rs` version would not catch.

**Verdict**: **REMOVE**

---

## Findings

### Finding A — Stale RTM Reference (Traceability Defect)

**Severity**: Medium

The file header reads:
```
// T-META-001.6: RTM FR-24.x — Ported frontmatter metadata regression coverage from the C# oracle.
```
and individual tests read:
```
// T-META-001.6: RTM FR-24.1 — WriteMarkdown envelope management.
```

In the **current RTM**, FR-24 is **Entra ID Authentication** (JWT validation, OIDC scopes,
RFC 9728 metadata — RTM-131 through RTM-136). These tests have nothing to do with Entra ID.
The FR-24.x label appears to reference a prior sprint's feature numbering that was overwritten
when the hosted-deployment PRD was layered in. `T-META-001.6` does not appear in `TODO.md`.

**Risk**: A reader using RTM traceability to find "tests for FR-24" will find these frontmatter
tests and incorrectly assume they cover Entra ID behavior. Conversely, the actual FR-24
(authentication) has no test traceability under that identifier.

**Recommendation**: Update comments to reference a stable, non-FR-numbered identifier — e.g.,
`CORE-FRONTMATTER-WRITE` — or map to the `NFR-1` (C# parity) entry that these tests legitimately
support.

---

### Finding B — AC1 and AC4 Test the Same Scenario

**Severity**: Low

Both `AC1` and `AC4` call `write_markdown(file_uri, None, …)` and assert `created`/`modified`
exist. AC4 is a strict superset. Two tests that invoke the same code path with identical inputs
and overlapping assertions create false confidence: if the feature breaks, both red — but one is
redundant noise in the failure report.

---

### Finding C — AC2's Assertion Does Not Verify the Invariant It Claims to Test

**Severity**: Medium

The test name says "existing created *preserved on rewrite*." But the assertions are:
1. `created` equals the value passed in ✅ (verifies preservation)
2. `modified` is non-empty ⚠️ (does NOT verify modification happened)

An implementation that writes `modified: <same-value-as-created>` would pass AC2. The
`markdown_rw_write_tests.rs` counterpart catches this with `assert_ne!(modified, original_created)`.
AC2 gives false assurance that modification-on-rewrite is tested.

---

## Recommendations

Ordered by priority:

1. **Fix the stale RTM reference** (Finding A) — update or remove the `RTM FR-24.x` comment
   across this file and `frontmatter_tool_flows.rs` before the hosted-deployment sprint lands and
   FR-24 tests are actually written. Confusion cost compounds as the suite grows.

2. **Rewrite AC4** to isolate its one unique assertion — the raw `---` envelope check. Name it
   `write_markdown_null_frontmatter_file_has_yaml_envelope` and remove the duplicate
   key-existence assertions. ~30 lines → ~12 lines, signal preserved.

3. **Remove AC1, AC3, AC5** — full behavioral duplicates of `markdown_rw_write_tests.rs` tests.
   No unique assertion, no unique input variant, no unique code path. The suite loses zero
   coverage and gains cleaner failure output.

4. **Rewrite or remove AC2** — if kept, add `assert_ne!(modified, original_created)` to close
   the assertion gap identified in Finding C. If removed, the `markdown_rw_write_tests.rs`
   counterpart already covers the invariant more rigorously.

5. **Consider consolidating** `frontmatter_write.rs` and `frontmatter_tool_flows.rs` into a
   single focused file after the above removals. After pruning, AC4-rewritten will be the sole
   test in this file — a one-test file with its own support module is an organizational smell.

---

## Load-Bearing Score: 1/5

Only one test (`AC4`) contributes an assertion that does not exist anywhere else in the suite —
the raw byte-level YAML envelope check (`raw_text.starts_with("---")`). The remaining four tests
either duplicate `markdown_rw_write_tests.rs` exactly, or duplicate each other, or both.

| Test | Net-new assertions vs. rest of suite |
|------|--------------------------------------|
| AC1  | 0 — fully covered by AC4 and `markdown_rw_write_tests.rs` |
| AC2  | 0 — weaker version of `markdown_rw_write_tests.rs` counterpart |
| AC3  | 0 — exact semantic duplicate |
| AC4  | 1 — `raw_text.starts_with("---")` (physical envelope format) |
| AC5  | 0 — exact semantic duplicate |
