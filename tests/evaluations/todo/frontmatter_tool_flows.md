# Test Evaluation: `frontmatter_tool_flows`

**File**: `crates/maenifold-core/tests/frontmatter_tool_flows.rs`
**Evaluator**: Red Team / Gray (Skeptical Inquiry)
**Traceability**: T-META-001.6 (originating sprint), NFR-2 (test coverage quality gate)
**Date**: 2026-07-14

---

## Summary

`frontmatter_tool_flows.rs` contains **6 integration tests** organised into two groups:

| Group | Tests | Layer |
|---|---|---|
| `fr24_2` — `append_h2_section` frontmatter side-effects | 3 | Markdown layer (direct API) |
| `fr24_3` — Tool-delegated timestamp writes | 3 | Tool API (write / edit / move memory) |

### Critical finding: stale RTM references (systemic)

Every test in this file embeds `RTM FR-24.x` traceability comments. In the **current PRD/RTM**, FR-24 is **Entra ID Authentication** (T-18.x). The original FR-24 covering frontmatter timestamp metadata was renumbered when hosted-deployment requirements were introduced. Any engineer reading the current RTM and then these tests will be confused: there is no RTM item that maps from `FR-24.2 — AppendH2Section routes through WriteMarkdown` to the current requirement set. This is a systemic traceability hazard, not a test quality defect per se, but it has test-quality implications because it makes the tests harder to navigate and audit.

### Critical finding: duplicate coverage for the `fr24_2` group

`markdown_rw_write_tests.rs` contains three tests that cover the **same behavioral properties** as `fr24_2_ac1`, `fr24_2_ac2`, and `fr24_2_ac3`:

| `frontmatter_tool_flows.rs` | Near-duplicate in `markdown_rw_write_tests.rs` |
|---|---|
| `fr24_2_ac1_append_h2_section_updates_modified` | `append_h2_section_existing_file_updates_modified` |
| `fr24_2_ac2_append_h2_section_preserves_frontmatter` | `append_h2_section_existing_file_preserves_all_frontmatter_fields` |
| `fr24_2_ac3_append_h2_section_preserves_content` | `append_h2_section_existing_file_preserves_existing_content` |

Tests 2 and 3 have small **unique sub-assertions** (discussed per-test below). Test 1 has zero unique value.

The `fr24_3` group (Tests 4–6) operates at the **tool API layer** and has no substantive duplicates in other test files, making them more load-bearing.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Assertions Correct? | Name Good? | Parameterizable? | Verdict |
|---|---|---|---|---|---|---|---|
| 1 | `fr24_2_ac1_append_h2_section_updates_modified` | ✗ | ✗ | ✓ (weak) | Partial | With T2/T5 | **REMOVE** |
| 2 | `fr24_2_ac2_append_h2_section_preserves_frontmatter` | Partial | Partial | ✓ (over-specified) | ✓ | With T1/T3 | **REWRITE** |
| 3 | `fr24_2_ac3_append_h2_section_preserves_content` | Partial | Partial | ✓ (redundant body) | ✓ | With T1/T2 | **REWRITE** |
| 4 | `fr24_3_ac1_write_memory_timestamps_via_writer` | ✓ | ✓ | ✗ (under-specified) | ✓ | With T5/T6 | **KEEP** (strengthen assertions) |
| 5 | `fr24_3_ac2_edit_memory_modified_via_writer` | ✓ | ✓ | ✓ | ✓ | With T4/T6 | **KEEP** |
| 6 | `fr24_3_ac3_move_memory_modified_via_writer` | ✓ | ✓ | ✗ (trivially weak) | ✓ | With T4/T5 | **REWRITE** |

---

## Detailed Findings

### Test 1 — `fr24_2_ac1_append_h2_section_updates_modified`

**What it claims to prove**: `append_h2_section` updates the `modified` frontmatter field.

**Is it load-bearing?** No. `markdown_rw_write_tests.rs::append_h2_section_existing_file_updates_modified` covers the identical behavioral property: a stale `modified` value is superseded after an append. The difference — that `frontmatter_tool_flows` starts from an *auto-generated* timestamp rather than a *hardcoded-stale* timestamp — carries no additional diagnostic power. Both call the same `append_h2_section` function; both assert `modified` changed. If `append_h2_section` broke timestamp updating, both tests would fail simultaneously; this test adds no independent trip-wire.

**Assertion quality**: Asserts `!second_modified.is_empty()` (trivially true if the field exists) and `second_modified != first_modified` (the meaningful assertion). The emptiness check is redundant given the change assertion.

**Name quality**: `fr24_2_ac1` prefix references a stale RTM item. The suffix is accurate but the prefix misleads.

**Parameterizable?** This test, together with the sleep-based timestamp check in `fr24_3_ac2`, could be extracted into a shared helper `assert_modified_advances(fn)` that accepts a mutating closure.

**Verdict: REMOVE.** The `markdown_rw_write_tests.rs` counterpart is strictly stronger (uses hardcoded timestamps, removes all temporal ambiguity about whether the initial write is "new enough" to differ).

---

### Test 2 — `fr24_2_ac2_append_h2_section_preserves_frontmatter`

**What it claims to prove**: `append_h2_section` preserves existing custom frontmatter fields and does not alter `created`.

**Is it load-bearing?** Partially. `markdown_rw_write_tests.rs::append_h2_section_existing_file_preserves_all_frontmatter_fields` already asserts that `title`, `type`, and `status` survive an append. **However**, that counterpart does NOT assert that `created` is unchanged. The assertion `yaml_string(&after_frontmatter, "created") == yaml_string(&before_frontmatter, "created")` is the test's unique contribution.

**What's redundant**: The three assertions for `title`, `type`, `status` all duplicate the rw_write counterpart verbatim. They add no new information.

**Assertion quality**: The unique assertion (`created` unchanged) is correct and meaningful. The duplicate assertions are diluting the test's clarity and coupling it to unrelated requirements.

**Name quality**: Stale `fr24_2_ac2` prefix; suffix `preserves_frontmatter` is too broad — it does the same as the rw_write test. If rewritten to focus on `created`, the name should be `append_h2_section_does_not_advance_created`.

**Verdict: REWRITE.** Remove the three duplicate field assertions (already verified elsewhere). Retain only the `created` before == `created` after assertion. Rename to reflect the unique claim.

```rust
// Suggested rewrite sketch
fn append_h2_section_preserves_created_timestamp() {
    // write → capture created → append → assert created unchanged
}
```

---

### Test 3 — `fr24_2_ac3_append_h2_section_preserves_content`

**What it claims to prove**: `append_h2_section` preserves prior content AND positions new content after old content.

**Is it load-bearing?** Partially. `markdown_rw_write_tests.rs::append_h2_section_existing_file_preserves_existing_content` asserts: old content present ✓, new heading present ✓, new body present ✓. It does **not** assert ORDER. The assertion `content.find("## Appended Section") > content.find("Original [[content]] here.")` is the test's unique contribution.

**What's redundant**: The three `assert!(content.contains(…))` calls duplicate the rw_write counterpart exactly. They succeed if and only if the rw_write test succeeds.

**Assertion quality**: The ordering assertion is correct and meaningful — it would catch a prepend-instead-of-append regression. The presence assertions are redundant.

**Name quality**: Stale prefix. Suffix `preserves_content` undersells the ordering check. Suggested: `append_h2_section_appends_after_existing_content`.

**Verdict: REWRITE.** Remove the three presence assertions. Keep only the `find()` ordering assertion. Rename.

---

### Test 4 — `fr24_3_ac1_write_memory_timestamps_via_writer`

**What it claims to prove**: The `memory::write_memory` tool delegates timestamp management to `MarkdownWriter` and produces a file with both `created` and `modified` in the frontmatter.

**Is it load-bearing?** Yes. `frontmatter_write.rs` tests `write_markdown` (the markdown layer) directly. This test exercises the full tool dispatch path: JSON input → `memory::write_memory` → `MarkdownWriter`. That delegation chain is a separate failure surface: a regression could silently bypass `MarkdownWriter` for the tool-level call and still pass the markdown-layer tests.

**Unique?** Yes. No other test in the test corpus calls `memory::write_memory` and then directly inspects the frontmatter for timestamp fields.

**Assertion quality**: **Under-specified.** The test asserts only `contains_key("created")` and `contains_key("modified")`. A map with `created: ~` (null YAML value) would satisfy these assertions. Compare with `frontmatter_write.rs::fr24_1_ac1`, which additionally asserts `is_some_and(|v| !v.is_empty())`. The same defensive check is missing here.

**Name quality**: Stale prefix; suffix is accurate. The word "via_writer" encodes implementation knowledge into the test name, which couples the name to a specific implementation strategy. Preferred: `write_memory_produces_created_and_modified_timestamps`.

**Verdict: KEEP.** Strengthen to assert non-empty values:
```rust
assert!(yaml_string(&frontmatter, "created").is_some_and(|v| !v.is_empty()));
assert!(yaml_string(&frontmatter, "modified").is_some_and(|v| !v.is_empty()));
```

---

### Test 5 — `fr24_3_ac2_edit_memory_modified_via_writer`

**What it claims to prove**: The `memory::edit_memory` tool (1) returns the expected output format and (2) updates the `modified` timestamp.

**Is it load-bearing?** Yes. Tests two orthogonal but correlated behaviors: the output format contract (`"Updated memory FILE:"` prefix) and the timestamp side-effect. A regression in either would be caught here and not elsewhere.

**Unique?** Yes for the combination. `memory_tools_editing_parity.rs` tests edit operations but does not inspect timestamps. No other test asserts the output string format alongside the `modified` update.

**Assertion quality**: Correct. The `sleep(20ms)` before the edit is appropriate to ensure timestamp resolution (though see note on time-dependent testing below). The `assert_ne` between before/after `modified` values is the right assertion — stronger than `is_empty()`.

**Name quality**: Stale prefix; suffix is accurate. The `via_writer` coupling is present but acceptable here since the test's focus is delegation correctness.

**Note on time-dependency**: The `sleep(20ms)` creates a fragile assumption that timestamp resolution is sub-20ms. On a heavily loaded CI runner with low-resolution clocks, two `Utc::now()` calls could collide within 20ms and cause a false failure. This is unlikely but not impossible. An alternative: seed the "before" timestamp with an explicit past value using `write_markdown` with a hardcoded `modified` field.

**Verdict: KEEP.** Consider eliminating time-dependency by seeding a known-stale `modified` value rather than relying on `sleep`.

---

### Test 6 — `fr24_3_ac3_move_memory_modified_via_writer`

**What it claims to prove**: The `memory::move_memory` tool leaves a `modified` key on the destination file.

**Is it load-bearing?** Yes (in principle). Tests the tool-layer delegation for move operations. No other test combines `move_memory` with frontmatter inspection.

**Unique?** Yes (no other file covers this).

**Assertion quality**: **Critically weak.** The test asserts only `frontmatter.contains_key("modified")`. This will pass even if `modified` is `null`, empty, or is simply copied verbatim from the source without being updated. A broken `move_memory` implementation that forgot to write any timestamps but happened to preserve the original frontmatter block (with its old `modified`) would pass this test. The test does not establish any of the following:
  - `modified` is non-empty
  - `modified` is a valid timestamp
  - `modified` reflects the time of the move (not just the original creation time)

Compare with Test 5 (`edit_memory`), which correctly uses `assert_ne` between before and after values.

**Name quality**: Stale prefix; suffix is weak. The name implies the test verifies delegation behaviour but the assertion only verifies key presence.

**Verdict: REWRITE.** At minimum, assert non-empty. Preferably, establish that the destination's `modified` is either equal to the time of the move (hard to assert precisely) or that the destination's `modified` is present and non-null. The strongest rewrite follows Test 5's pattern: capture `source_uri` modified before move, assert destination's `modified` is non-empty and matches a post-move timestamp.

```rust
// Suggested strengthening sketch
let (src_fm, _, _) = read_markdown(&source_uri, &env.cfg)?;
let src_modified = yaml_string(&src_fm.unwrap(), "modified").unwrap();

// perform move ...

let (dest_fm, _, _) = read_markdown(dest_uri, &env.cfg)?;
let dest_modified = yaml_string(&dest_fm.unwrap(), "modified");
assert!(dest_modified.is_some_and(|v| !v.is_empty()));
// Optionally: assert destination modified >= source modified
```

---

## Systemic Issues

### 1. Stale RTM references (all 6 tests)

All six tests carry `// T-META-001.6: RTM FR-24.x` comments. In the current PRD/RTM (as of the hosted-deployment sprints), **FR-24 is Entra ID Authentication** (T-18.x). The original FR-24 that covered frontmatter timestamp metadata has been superseded by renumbering. This creates a false traceability chain: a reader auditing RTM coverage for FR-24 (auth) will find false positive coverage from these tests; a reader auditing what tests cover the timestamp requirements will miss these tests.

**Recommendation**: Update comments to reference the actual requirement these tests enforce. The closest current RTM items are NFR-1 (C# oracle parity) and NFR-2 (test coverage). A new, dedicated RTM item for frontmatter metadata correctness should be added if this is a first-class requirement.

### 2. Global-state mutex forces serial execution (all 6 tests)

All tests lock `TEST_MUTEX` because `Config::override_root` modifies global state. This means these tests cannot run in parallel with any other test that touches config. A future move to `Config` being passed explicitly (or using a per-thread config context) would eliminate this constraint and allow parallelisation.

### 3. Time-dependent sleep pattern (Tests 1, 5)

Tests 1 and 5 rely on `thread::sleep(Duration::from_millis(20))` to ensure clock advancement between write and mutation. On CI runners with low-resolution clocks or high load, this is a latent flakiness source. Prefer seeding a known-stale `modified` value in the initial write so no sleep is needed.

---

## Recommendations

1. **REMOVE Test 1** (`fr24_2_ac1`). The behavioral property it tests is already fully covered by `markdown_rw_write_tests.rs::append_h2_section_existing_file_updates_modified`, which uses a more explicit setup.

2. **REWRITE Test 2** (`fr24_2_ac2`). Strip the three field-presence assertions duplicated from rw_write. Keep only the `created` unchanged assertion. Rename to `append_h2_section_does_not_advance_created`.

3. **REWRITE Test 3** (`fr24_2_ac3`). Strip the three `contains()` assertions duplicated from rw_write. Keep only the `find()` ordering assertion. Rename to `append_h2_section_appends_after_existing_content`.

4. **KEEP Test 4** (`fr24_3_ac1`) but strengthen: add `is_some_and(|v| !v.is_empty())` assertions on both `created` and `modified` to match the pattern established in `frontmatter_write.rs`.

5. **KEEP Test 5** (`fr24_3_ac2`). Consider eliminating `sleep` by seeding a hardcoded-stale `modified` in the initial write.

6. **REWRITE Test 6** (`fr24_3_ac3`). Strengthen from `contains_key` to a non-empty value assertion, matching Test 5's pattern.

7. **Fix RTM references** (all tests). Update `RTM FR-24.x` comments to the current requirement identifiers, or remove and replace with an accurate cross-reference to NFR-1/NFR-2.

---

## Load-Bearing Score: 4/6

| Test | Load-Bearing |
|---|---|
| `fr24_2_ac1_append_h2_section_updates_modified` | ✗ — pure duplicate |
| `fr24_2_ac2_append_h2_section_preserves_frontmatter` | ✓ — unique assertion (created unchanged), buried in duplicates |
| `fr24_2_ac3_append_h2_section_preserves_content` | ✓ — unique assertion (ordering), buried in duplicates |
| `fr24_3_ac1_write_memory_timestamps_via_writer` | ✓ — unique tool-layer coverage, weak assertions |
| `fr24_3_ac2_edit_memory_modified_via_writer` | ✓ — unique tool-layer + format assertion |
| `fr24_3_ac3_move_memory_modified_via_writer` | ✗ — `contains_key` assertion is too weak to catch real regressions |

**4 tests carry genuine load** (2 via unique sub-assertions that need rescue by rewrite; 2 via tool-layer coverage). **2 tests should be removed or rewritten** before they mislead future readers into believing the suite has stronger coverage than it does.

---

## SWE Remediation

- REMOVE `fr24_2_ac1_append_h2_section_updates_modified`: pure duplicate of `markdown_rw_write_tests::append_h2_section_existing_file_updates_modified`
- REWRITE `fr24_2_ac2_append_h2_section_preserves_frontmatter` → `append_h2_section_preserves_created_timestamp`: stripped 3 duplicate field assertions (title/type/status already covered by rw_write), retained unique `created` unchanged assertion
- REWRITE `fr24_2_ac3_append_h2_section_preserves_content` → `append_h2_section_appends_after_existing_content`: stripped 3 duplicate `contains()` assertions (already covered by rw_write), retained unique `find()` ordering assertion
- KEEP `fr24_3_ac1` → `write_memory_produces_created_and_modified_timestamps`: strengthened from `contains_key` to `yaml_string(...).is_some_and(|v| !v.is_empty())` per P0
- KEEP `fr24_3_ac2` → `edit_memory_advances_modified_timestamp`: renamed to remove stale prefix; logic unchanged
- REWRITE `fr24_3_ac3` → `move_memory_destination_has_nonempty_modified`: strengthened from `contains_key("modified")` to `yaml_string(...).is_some_and(|v| !v.is_empty())`
- Fixed stale RTM references: all `FR-24.x` comments updated to `NFR-1` (C# oracle parity)
- Removed unused imports: `serde_yaml::Value as YamlValue`, `std::collections::BTreeMap`
- Result: 6 tests → 5 tests; all 5 pass
- Commit: aacc63e
