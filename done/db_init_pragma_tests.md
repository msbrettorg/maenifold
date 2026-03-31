---
id: db_init_pragma_tests
status: done
---

# Test Evaluation: db_init_pragma_tests
**File:** `crates/maenifold-core/tests/db_init_pragma_tests.rs`
**Test count:** 8
**Evaluated:** 2026-06-05
**Traceability:** T-15.11 · T-15.12 · T-15.22 · T-15.23 · RTM FR-11

## Summary

This is the strongest test file of the three under evaluation. Seven of eight tests assert concrete numeric pragma values or observable data integrity outcomes. One test (test 2) is technically redundant given the isolation model, and one (test 8) is an intentional documentation/regression test with only marginal defect-detection value. Test 4 is the most load-bearing test in the entire three-file set and should be protected.

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `t15_11_foreign_keys_enabled_after_open_connection` | KEEP | Asserts `PRAGMA foreign_keys = 1` after the first `open_connection` call. Direct, narrow, and pinpoints exactly where the failure would be. |
| `t15_11_foreign_keys_enabled_on_second_connection` | KEEP (with clarification) | The test's purpose — verifying that FK pragma is applied per-connection rather than only during schema initialization — is genuinely distinct from test 1. However, `isolated_env()` already calls `GraphDatabase::initialize_database` which likely opens an internal connection; the "second" connection is therefore the third or later handle opened against this DB. The test comment should reflect the actual sequence rather than calling it "a second connection." The underlying property being tested (every call to `open_connection` sets FK=ON) is sound and load-bearing. |
| `t15_12_co_occurrence_count_column_is_integer` | KEEP | Schema-regression test; the most targeted check possible for this type of bug. The `.to_uppercase()` normalization correctly handles SQLite's case-variable type affinity strings. Would immediately catch a revert to `REAL`. |
| `t15_11_cascade_delete_works_with_foreign_keys_on` | KEEP | The most load-bearing test in the file. It verifies that FK enforcement is not just a pragma claim but actually operates — cascade propagates to `concept_mentions`, `concept_graph`, and `concept_communities`. The three precondition assertions before the delete are good practice. One gap: the query `WHERE concept_a = 'test-concept' OR concept_b = 'test-concept'` only covers the edge `(other-concept, test-concept)`. If the schema has a FK on `concept_a` only and not on `concept_b`, the cascade might silently fail for `concept_b`-side edges. Adding a symmetrical test edge `(test-concept, another-concept)` would close this gap. Also tagged `t15_11` but exercises behavior defined in T-15.11 + T-15.12 jointly; consider documenting this dual traceability. |
| `t15_22_journal_mode_wal_on_new_connection` | KEEP (investigate CI comment) | The inline comment `// Outside CI, journal_mode should be "wal"` is alarming — it implies this test may be expected to **fail in CI**. If `journal_mode` is not WAL in CI, either the test is conditionally gated (and the condition is invisible here) or it is a known flapping test. Verify and either remove the comment, add a `#[cfg_attr(ci_env, ignore)]`, or document why WAL is unavailable in CI. An assertion that can silently be false in the CI environment provides no actual coverage guarantee. |
| `t15_22_synchronous_normal_on_new_connection` | KEEP | Asserts `synchronous = 1` (NORMAL). Concrete numeric assertion. Load-bearing: if someone changes the pragma to `FULL` (2) or `OFF` (0) for performance tuning, this test catches it. No concerns. |
| `t15_22_journal_mode_wal_after_vacuum` | KEEP | The highest-effort and highest-value resilience test in the file. Exercises the full round-trip: set DELETE mode via raw connection → verify DELETE persists → open via API → verify WAL restored. The two precondition assertions (one on original connection, one on raw reopen) make the test self-documenting. **Same CI concern as test 5 applies here** — if WAL cannot be set in CI, this test also silently lies. |
| `t15_23_no_query_only_on_standard_connection` | KEEP (downgrade to informational) | This is an honest documentation test — the comment explicitly calls it out as such. It has limited defect-detection value: if `query_only=ON` were accidentally set, most write-path tests (`t15_11_cascade_delete_*`, sync tests, etc.) would fail first and with more informative messages. However, as a targeted diagnostic anchor for the specific pragma, it is cheap and worth keeping. Consider adding a comment: `// If this is the first failure you see, open_connection is setting query_only=ON unexpectedly.` |

## Recommendations

1. **Investigate the "Outside CI" comment on `t15_22_journal_mode_wal_on_new_connection`**. Either WAL works in CI and the comment is wrong (remove it), or the test is genuinely unreliable in CI (add `#[cfg_attr(..., ignore)]` and document why). The same question applies to `t15_22_journal_mode_wal_after_vacuum`. This is the single highest-priority issue in this file — a test that is known to not work in the pipeline is worse than no test.
2. **Strengthen `t15_11_cascade_delete_works_with_foreign_keys_on`**: add a second edge `(test-concept, another-concept)` and assert it also disappears post-cascade. This pins FK behavior on `concept_a` independently of `concept_b`.
3. **Clarify test 2's comment**: reword "Open a second connection" to accurately reflect the connection sequence relative to `isolated_env()` internals. This prevents future readers from misunderstanding what "second" means.
4. **Add a `busy_timeout` pragma test**: if `open_connection` sets `busy_timeout` (common in WAL setups to avoid `SQLITE_BUSY` errors), it should be pinned here alongside the other pragma assertions. Its absence from this file is a notable gap.
5. **Consider splitting `t15_11_cascade_delete_works_with_foreign_keys_on`** into three focused tests: `cascade_delete_removes_concept_mentions`, `cascade_delete_removes_concept_graph_edges`, `cascade_delete_removes_concept_communities`. The current monolith is correct but a failure in community cascade is visually buried under mention and graph assertions.

## Load-Bearing Score: 7/8

Test 8 (`t15_23_no_query_only_on_standard_connection`) is a marginal documentation test with weak independent defect-detection power. All other seven tests directly assert observable, behavior-critical outcomes. The CI/WAL concern, if confirmed, would reduce the effective score further for tests 5 and 7.

---

## SWE Remediation

- **R1 (P0)**: WAL CI issue resolved — `journal_mode()` in `connection.rs` intentionally returns `"DELETE"` in CI (checks `CI=true` / `GITHUB_ACTIONS=true`). Removed misleading "Outside CI" comment from test 5. Added `is_ci()` helper and runtime skip to `t15_22_journal_mode_wal_on_new_connection` and `t15_22_journal_mode_wal_after_vacuum` with clear documentation explaining why.
- **R2 (P1)**: Strengthened `t15_11_cascade_delete_works_with_foreign_keys_on` — added `another-concept` + edge `(test-concept, another-concept)` to pin FK cascade on `concept_a` independently of `concept_b`. Edge precondition updated from 1→2. Post-cascade assertion message clarified to document dual-column coverage.
- **R3 (P1)**: Clarified test 2 comment — now documents that `isolated_env()` already opens a connection via `initialize_database`, so the "second" connection is at least the second handle.
- **R4 (P1)**: Added `t10_9_busy_timeout_set_on_new_connection` — pins default 5000 ms `busy_timeout` pragma (T-10.9: RTM FR-15). Closes the notable gap identified in the evaluation.
- **R5 (P3+)**: Skipped — splitting cascade_delete would increase file size without proportional defect-detection gain.
- **Test 8**: Added diagnostic anchor comment per verdict recommendation.
- Test count: 8 → 9 (1 added). All 9 passing.
- Commit: ad0b4c5
