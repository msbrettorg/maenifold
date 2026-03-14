# Test Evaluation: sync_wikilink_filter

**File:** `crates/maenifold-core/tests/sync_wikilink_filter.rs`
**Test count:** 9
**Evaluated:** 2026-07-14
**Traceability:** T-22.4 (RTM-151–156), T-22.5 (RTM-152, 155, 156)

## Summary

The strongest of the three files evaluated. Tests assert directly against the `concept_mentions` and `concept_graph` database tables, bypassing string output parsing entirely — that is the right instinct. Two tests have real defects: one weak assertion that cannot distinguish a correct count from an accidental digit, and one test with a dead variable revealing a missing assertion.

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `sync_filters_exact_blocked_concepts` | **KEEP** | Load-bearing. Queries `concept_mentions` directly. Exact-match blocking verified at DB level. Assertions are unambiguous. |
| `sync_filters_prefix_blocked_concepts` | **KEEP** | Load-bearing. Both `T-*` and `RTM-*` prefix patterns tested, DB assertions. Covers a genuinely distinct code path from exact matching. |
| `sync_output_reports_filtered_count` | **REWRITE** | Assertion is `filtered_line.chars().any(|ch| matches!(ch, '1'..='9'))` — this passes if the output says "filtered 0 (total: 1)" or if any unrelated digit appears on the same line as "filtered". It does not assert the actual filtered count. With 3 filtered concepts across 2 files, assert the specific count (3). |
| `incremental_sync_also_filters` | **KEEP** | Load-bearing. Exercises `sync_single_file` (distinct code path from full `sync`). DB assertions. Without this test, a regression isolating `sync_single_file` to skip filter loading would go undetected. |
| `sync_works_without_filter_file` | **KEEP** | Load-bearing. Graceful degradation when `.wikilink-filter.json` is absent. Tests a real failure mode — startup configuration state that is legitimately absent. |
| `sync_with_filter_produces_fewer_concepts` | **KEEP** | Borderline. It only checks `count_with < count_without` (no specific number), which is a weak aggregate assertion. However it exercises the `reset_database` path and guards against filter code silently running but failing to delete rows from the `concepts` table — which `sync_filters_exact_blocked_concepts` does NOT cover (that test queries `concept_mentions`, not `concepts`). Worth keeping but document the distinction. |
| `sync_prefix_filter_does_not_block_unrelated_concepts` | **KEEP** | Load-bearing. Tests false-positive prevention — that `T-*` does not match `tool` or `template`. Case sensitivity of the prefix matcher is a real correctness requirement. This is the only test that would catch a case-insensitive prefix match bug. |
| `sync_filter_excludes_blocked_from_concept_graph` | **REWRITE** | `allowed_b` is declared but never asserted — `let _ = allowed_b;` is a suppressed dead-variable warning, not an intentional no-op. The test verifies `allowed_a` has concept_graph rows but silently skips `allowed_b`. This suggests the test was written with a second co-occurrence assertion in mind and left incomplete. Add: `assert!(count_concept_graph_entries(&cfg, &allowed_b) > 0, "allowed_b must have relations");` |
| `sync_does_not_crash_with_malformed_filter_file` | **KEEP** | Load-bearing. Malformed JSON is a real operational input. Asserts both that sync completes AND that all concepts pass through unfiltered (correct fallback behavior). |

## Recommendations

1. **`sync_output_reports_filtered_count`**: Replace the digit-scan with an exact count assertion. The filter has 3 entries (agent, tool, T-*) across 2 files — the filtered count is deterministic. Assert it:
   ```rust
   assert!(filtered_line.contains('3') || /* expected count */,
       "filtered summary must report exact count: {filtered_line}");
   ```
   Better: expose a structured return from `sync` and assert the integer directly, instead of parsing markdown.

2. **`sync_filter_excludes_blocked_from_concept_graph`**: Add the missing `allowed_b` graph assertion. The dead variable is a test smell that will confuse future maintainers — it looks like an assertion was removed.

3. **Consider**: `sync_with_filter_produces_fewer_concepts` could assert the *exact* delta (4 concepts filtered: agent, tool, T-14.1, RTM-042 = exactly 4 fewer rows in `concepts`). This would make the test significantly more load-bearing.

4. **No parameterization opportunity** here — each test covers a genuinely distinct scenario. The suite is appropriately decomposed.

## Load-Bearing Score: 7/9

Tests 3 and 8 have assertion defects that would fail to catch real bugs. Test 3's digit-scan assertion is too weak to be reliable. Test 8 silently skips its second intended assertion.
