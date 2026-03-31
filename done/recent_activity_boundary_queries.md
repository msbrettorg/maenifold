---
id: recent_activity_boundary_queries
status: done
---

# Test Evaluation: recent_activity_boundary_queries

**File**: `crates/maenifold-core/tests/recent_activity_boundary_queries.rs`
**Traceability**: T-2.3 | FR-4 | RTM-020, RTM-021, RTM-022, RTM-023, RTM-024, RTM-026
**Evaluator**: Red Team / Gray (Skeptical Inquiry)
**Date**: 2025-07-14

---

## Summary

This file is one of three boundary-layer test files for the `RecentActivity` tool (alongside
`recent_activity_boundary_rendering.rs` and the higher-level `recent_activity_tool_behavior.rs`).
It carries 4 tests covering: ordering, filter variants, timespan restriction, and result-count
capping. All 4 use the shared `recent_activity_boundary/support.rs` fixture module with direct
DB seeding (no `Sync` invocation), which is the correct approach for boundary isolation.

**Strengths**: Fast, isolated (TempDir + direct SQL inserts), correctly avoids filesystem
overhead, good RTM traceability via comments.

**Concerns**: One fat test bundles three independent behavioural scenarios; a sister file
(`recent_activity_tool_behavior.rs`) re-covers filters + timespan + limit end-to-end with a
*single* mega-test of its own, creating substantial cross-file overlap; the timespan test does
not exercise the actual boundary (the cutoff moment itself); and `rendered_item_count` is a
fragile helper that counts any line starting with `**`.

---

## Verdicts

| # | Test Name | RTM | Load-Bearing? | Unique? | Assertions Correct? | Name Clear? | Parameterisable? | Verdict |
|---|-----------|-----|--------------|---------|--------------------|-----------|--------------------|---------|
| 1 | `recent_activity_default_query_orders_by_last_indexed_desc` | RTM-020 | ✅ Yes — sole ordering test | ✅ Yes | ✅ Yes | ✅ Yes | 🔲 Low value | **KEEP** |
| 2 | `recent_activity_filter_variants_select_expected_path_families` | RTM-021/022/023 | ⚠️ Partial — covered by `tool_behavior.rs` | ⚠️ Partial | ✅ Yes | ✅ Yes | ✅ High value | **REWRITE** |
| 3 | `recent_activity_timespan_restricts_results_to_requested_window` | RTM-024 | ✅ Yes | ⚠️ Partial — also in `tool_behavior.rs` | ⚠️ Misleading name | ⚠️ Borderline | 🔲 Low value | **KEEP** (with rename) |
| 4 | `recent_activity_limit_caps_number_of_results` | RTM-026 | ✅ Yes | ⚠️ Partial — also in `pipeline_collect.rs` | ⚠️ Implicit ordering coupling | ✅ Yes | ✅ High value | **KEEP** |

---

## Per-Test Analysis

---

### Test 1 — `recent_activity_default_query_orders_by_last_indexed_desc`

**RTM**: TC-4.1 / RTM-020

**What it does**: Seeds 4 items at 1h, 2h, 3h, 4h ago via `seed_mixed_activity`. Invokes
`RecentActivity` with `limit: 10`. Verifies that the 4 named items appear in rendered output
in strict position order (session-recent → workflow-middle → knowledge-note → assumption-log).

**Is it load-bearing?**
Yes. No other file in the `recent_activity_*` test suite specifically asserts ordering. The
`tool_behavior.rs` and `pipeline_collect.rs` files test filters and limits but never assert
the relative position of items in output. This test is the only thing standing between a silent
ordering regression and a green CI run.

**Is the ordering probe meaningful?**
The check uses `.find()` byte offsets in the rendered string — an indirect but valid proxy for
"did these items render in the right sequence?" It works because item names appear in bold
(`**session-recent**`) which is unique in the output. The positions correctly reflect
`last_indexed DESC` if the tool is working. However: SQLite does *not* guarantee insertion
order without `ORDER BY`, so removing the `ORDER BY` clause would break this test — which is
precisely what we want.

**What's questionable?**
The items in `seed_mixed_activity` are seeded at 1h, 2h, 3h, 4h gaps. These could theoretically
race if the test process is slow and the timestamps straddle a clock boundary. In practice the
1-hour gaps make this astronomically unlikely, but it is a latent flakiness risk.

**Verdict**: **KEEP** — sole ordering assertion, irreplaceable.

---

### Test 2 — `recent_activity_filter_variants_select_expected_path_families`

**RTM**: TC-4.2 / RTM-021, RTM-022, RTM-023

**What it does**: Three sub-scenarios inside one `#[test]`:
- `filter: "thinking"` → sequential + workflow present, memory + assumptions absent
- `filter: "memory"` → knowledge-note present, all others absent
- `filter: "assumptions"` → assumption-log present with `Title:` label, all others absent

**Is it load-bearing?**
Partially. Filter behaviour *is* tested, but the same 3 filter paths are exercised in:
- `recent_activity_tool_behavior.rs::recent_activity_filters_timespan_include_content_workflow_and_sequential`
  (integration layer, after Sync)
- `recent_activity_pipeline_collect.rs` (unit layer, against the pipeline `collect` function
  directly)

This file's value-add is that it tests filter dispatch at the *boundary layer with raw DB
inserts*, confirming the SQL query (not just the pipeline logic or the rendered output of a
synced file). That is a meaningful layer distinction.

**Is bundling 3 scenarios into 1 test problematic?**
Yes. The three filter variants are independent behaviours. When `thinking` filter assertions
fail, Rust aborts the function and the `memory` and `assumptions` scenarios never run — the
test suite loses signal on whether those filters also regressed. Three separate
`#[test]` functions with individual names would each produce a clear FAIL/PASS, cutting
triaging time from "which of 3 things broke?" to zero.

**The `assumptions` classification assumption**: The assumption-log item is seeded at path
`memory://chat/2026/03/assumption-log` and title `Assumption Log`. The assertions confirm it
appears in `filter: "assumptions"` output. This implicitly encodes that the classifier
recognises either the path segment `chat/` or the title keyword `Assumption`. This assumption
is not documented in the test — if the classification rule changes (e.g., relies on a tag
rather than path), the test will fail without a clear error message explaining *why*.

**Verdict**: **REWRITE** — split into 3 named tests (`_thinking_filter`, `_memory_filter`,
`_assumptions_filter`). Each should carry its own RTM comment. Consider a data-driven table
test if the language permits. The per-filter isolation also makes the classification contract
explicit.

---

### Test 3 — `recent_activity_timespan_restricts_results_to_requested_window`

**RTM**: TC-4.3 / RTM-024

**What it does**: Seeds one "fresh" item (2 hours ago) and one "stale" item (3 days ago).
Queries with `timespan: "24.00:00:00"`. Asserts fresh appears, stale does not.

**Is it load-bearing?**
Yes — it confirms timespan filtering at the boundary layer. No other boundary test in this
file does this. However `tool_behavior.rs` also exercises timespan at integration level (sets
`last_indexed` to 2020 and queries with 1-day window), and `pipeline_tool.rs` has an
end-to-end timespan test.

**Does it test the boundary?**
No, and this is the sharpest critique. The test name says "boundary queries" but uses values
that are maximally far from the cutoff: fresh = **2 hours** ago, stale = **3 days** ago,
window = **24 hours**. A true boundary test would seed items at 23h 55m (expected IN) and
24h 05m (expected OUT) to verify the cutoff is `>=` or `>`, and catches off-by-one errors in
interval arithmetic. The current values give strong confidence that the filter exists but no
confidence it's arithmetically precise.

**Name concern**: The name `…_restricts_results_to_requested_window` is accurate for what the
test *does*, but the test file name (`boundary_queries`) implies boundary-condition coverage
that isn't present.

**Verdict**: **KEEP** (the test is valid), but rename to
`recent_activity_timespan_excludes_rows_outside_window` and add a companion test:
`recent_activity_timespan_includes_items_at_exact_boundary_edge` that seeds items at ±5 minutes
of the cutoff to catch arithmetic rounding errors.

---

### Test 4 — `recent_activity_limit_caps_number_of_results`

**RTM**: TC-4.5 / RTM-026 (note: TC-4.4 is skipped in this file — belongs to `_rendering.rs`)

**What it does**: Seeds 4 items via `seed_mixed_activity`. Queries with `limit: 2`. Asserts
`rendered_item_count == 2`, verifies the top-2 names appear, and verifies bottom-2 do not.

**Is it load-bearing?**
Yes — it is the definitive boundary-layer assertion that `LIMIT N` is respected and that
over-limit items are not leaked. `pipeline_collect.rs` has a similar test at the collection
layer, and `tool_behavior.rs` tests `limit: 1` at integration level, but this is the only
one asserting both count accuracy *and* which specific items appear.

**Implicit coupling to ordering**: The assertions `contains("**session-recent**")` and
`!contains("**knowledge-note**")` only hold because ordering is `last_indexed DESC`. If
ordering changes, this test fails with a misleading message ("expected session-recent"). The
dependency on test #1's invariant is real but undocumented. A comment like
`// relies on DESC ordering: session-recent (1h ago) > workflow-middle (2h ago)` would make
this explicit.

**`rendered_item_count` fragility**: The helper counts `output.lines().filter(|l| l.starts_with("**"))`.
The seeded content bodies (`"## Thought 1\nFirst sequential thought\n..."`) do not start with
`**`, so no false positives are currently possible. But this is a silent assumption about the
renderer not ever emitting `**bold**` as the first character on a line in the output format.
If the renderer changes to emit bold-formatted section headers, this helper silently
over-counts. The helper should filter by a more specific pattern (e.g., the tool's actual
item-line format).

**Verdict**: **KEEP** — uniquely asserts both count and identity of limit-respecting items.
Add an inline comment documenting the ordering dependency.

---

## Recommendations

1. **Split test #2 into 3 focused tests** (P1 — immediately actionable):
   ```rust
   #[test] fn recent_activity_thinking_filter_shows_sequential_and_workflow() { … }
   #[test] fn recent_activity_memory_filter_shows_memory_files_only() { … }
   #[test] fn recent_activity_assumptions_filter_shows_chat_path_files() { … }
   ```
   Each should carry a single RTM comment (RTM-021, RTM-022, RTM-023 respectively).

2. **Add a true boundary test for timespan** (P1 — addresses the most significant correctness gap):
   ```rust
   #[test]
   fn recent_activity_timespan_includes_items_at_boundary_edge() {
       // seed at 23h55m ago → expect IN
       // seed at 24h05m ago → expect OUT
       // query with timespan: "24.00:00:00"
   }
   ```

3. **Document the ordering dependency in test #4** (P2):
   Add a comment before the `contains("**session-recent**")` assertions clarifying the test
   relies on `last_indexed DESC` ordering (covered by test #1).

4. **Harden `rendered_item_count`** (P2):
   Replace the `starts_with("**")` heuristic with a pattern that matches the actual item-line
   format the tool emits (e.g., starts with `**` and ends with `**` or ` (sequential)`).

5. **Cross-file deduplication** (P3 — longer-term):
   `recent_activity_tool_behavior.rs` re-tests all 3 filter variants plus timespan and limit
   in a single integration-level mega-test. Consider whether that file's filter coverage
   should be removed after this file's tests are split into focused named tests. Running the
   same logical assertion at boundary + integration + pipeline levels is justified for
   high-confidence tooling, but the current situation is three overlapping mega-tests that
   would all fail simultaneously on the same regression, offering no diagnostic advantage.

6. **Note the missing TC-4.4 gap**: TC-4.4 (RTM-025, `includeContent`) lives in
   `boundary_rendering.rs`. The RTM comment in `boundary_queries.rs` skips from TC-4.3 to
   TC-4.5. This is intentional and correct per the file split, but worth confirming in the
   RTM that TC-4.4 is fully covered in the rendering file.

---

## Load-Bearing Score: 3/4

| Test | Load-Bearing |
|------|-------------|
| `…_orders_by_last_indexed_desc` | ✅ Sole ordering test |
| `…_filter_variants_…` | ⚠️ Covered by `tool_behavior.rs`; boundary-layer isolation is the partial differentiator |
| `…_timespan_restricts_…` | ✅ Only boundary-layer timespan test; does not test the actual cutoff edge |
| `…_limit_caps_…` | ✅ Only test asserting both count and item identity under a limit |

> **Interpretation**: 3 of 4 tests are irreplaceable at this layer. Test #2
> (`filter_variants`) carries genuine boundary-layer value but is a poor diagnostic unit due
> to bundling and substantial overlap with `tool_behavior.rs`. Rewriting it as 3 focused tests
> elevates this file to 5/6 and makes every failure immediately actionable.

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- recent_activity_boundary_queries`: 4 passed, 0 failed
- Verdict: PASS
