# Test Evaluation: incremental_sync_tools_processing

**File**: `crates/maenifold-core/tests/incremental_sync_tools_processing.rs`  
**Traceability**: T-12.7 · NFR-1 (C# Oracle Parity) · NFR-2 (Test Coverage)  
**Evaluator**: Red Team (Gray — Skeptical Inquiry)  
**Date**: 2025-07-11  

---

## Summary

This file is one half of a two-part test split (the other being `incremental_sync_tools_watcher.rs`) covering
`process_path_for_test` / `process_pending_paths_for_test` — thin test-only shims into `sync::sync_single_file`
and the watcher deduplication loop. The production code exercised is real and critical; the test quality is uneven.

**Scope**: 5 `#[test]` functions covering create, update, delete, file-type filtering, and path deduplication.

**Key findings**:

1. **Test 2** has a load-bearing omission: it never asserts the old concept was removed, which is the exact regression the test should catch.
2. **Test 5** has a vacuously true assertion: `concept_v1 = None` cannot fail because `concept_v1` was never indexed. The test passes regardless of whether the deduplication code is correct.
3. **Test 1** substantially overlaps with `sync_unification_single_file.rs::process_file_new_file_inserts_content_mentions_and_index_rows` — same scenario, same assertions, weaker entry point.
4. **Tests 3 and 4** are well-formed and load-bearing.

---

## Per-Test Analysis

---

### `process_file_created_with_wikilinks_inserts_concept_mentions_into_db`

**What it does**: Creates a markdown file with two UUID-namespaced WikiLinks, calls `process_path_for_test`,
and asserts both concepts appear in `concept_mentions` and the file URI appears in `file_content`.

**Load-bearing?**  
Yes — but `sync_unification_single_file.rs::process_file_new_file_inserts_content_mentions_and_index_rows`
tests an almost identical scenario with identical DB assertions (`get_mention_count = Some(1)` for two concepts,
`get_file_content_row` exists). The unification test additionally asserts the sync output text
(`"Synced memory://..."`), making it the *stronger* of the two. The only meaningful distinction is the
entry-point: this test calls `process_path_for_test` directly (bypassing leader election); the unification
test calls `sync_one` (through a higher-level path). That distinction has some value but does not justify
fully redundant DB assertions.

**Unique?**  
Partially. The entry point (`process_path_for_test` vs `sync_one`) differs, but the observable behavior
being asserted is identical. Does not add unique negative-path or boundary coverage beyond what the
unification test already provides.

**Correct assertions?**  
Yes — `mention_count = Some(1)` for both concepts, `file_row = Some(uri)`. However, does not assert:
- `file_md5` is non-empty (hash stored on first index)
- Vector/embedding row created (if applicable at this layer)

These omissions are acceptable for a focused processing test. The core contract is correct.

**Good name?**  
Yes — accurately describes the arrange/act/assert chain.

**Parameterizable?**  
The two-concept scenario is sufficient. UUID namespacing prevents cross-test pollution. Not a candidate for
meaningful parameterization at this level.

---

### `process_file_changed_with_new_content_adds_mention_for_new_concept_and_updates_file_content`

**What it does**: Creates a file with `concept_a`, indexes it, backdates `last_indexed` to force re-processing,
rewrites the file with `concept_b` only, processes again, and asserts `concept_b` is present and the stored
MD5 has changed.

**Load-bearing?**  
Yes — the update path is a distinct code branch from create, and must be tested. However, the test fails to
cover the most important regression surface: **that concept_a's mention is removed after the update**.

**Critical omission**: The file was rewritten to contain only `concept_b`. The production code is expected
to reconcile `concept_mentions` (delete rows for concept_a that no longer appear). This test does NOT assert
`get_mention_count(&cfg, &memory_uri, concept_a) == None`. If the sync code were changed to *append* rather
than *replace* concept mentions on update, this test would still pass.

By contrast, `sync_unification_single_file.rs::process_file_content_changed_reprocesses_all_mentions`
verifies exactly this — it uses `concept_sources("alpha").is_empty()` and `concept_names()` to confirm
removal. The processing test is the weaker cousin with a meaningful gap.

**Unique?**  
Partially. The backdated-`last_indexed` mechanism is different from the `set_file_mtime_utc` trick in the
unification tests, but both serve the same purpose: forcing the mtime guard to trigger reprocessing.

**Correct assertions?**  
**No** — incomplete. `concept_b = Some(1)` and `hash changed` are correct but insufficient. Missing:
`concept_a = None` (or `<= 0`). This is the only regression that matters for the update path.

**Good name?**  
The name says "adds mention for new concept and updates file content" — both halves are asserted. But the
name omits "removes mention for removed concept", which is the unasserted behavior. The name is accidentally
accurate about what is (and isn't) tested.

**Parameterizable?**  
Not a productive axis.

---

### `process_file_deleted_after_create_removes_file_content_and_mentions_from_db`

**What it does**: Creates and indexes a file with one concept, deletes the file, calls `process_path_for_test`
on the now-missing path, and asserts both `file_content` and `concept_mentions` rows are gone.

**Load-bearing?**  
Yes — the delete/tombstone path is the primary mechanism preventing stale concept accumulation. Without it,
ghost concepts never leave the graph. `incremental_sync_boundary_mutations.rs::deleted_files_remove_their_concepts_from_the_graph`
covers similar ground via `sync_one` and checks output text + `concept_names()` set. This test's direct DB
assertions via `get_file_row` and `get_mention_count` complement rather than duplicate those checks — they
verify the concrete rows are gone, not just that the output text mentions "stale index entry."

**Unique?**  
Yes — the entry-point distinction (`process_path_for_test` directly) and the specific `get_file_row = None`
assertion form a useful pair with the boundary mutation test.

**Correct assertions?**  
Yes — both the file row and the concept mention are checked. The implicit `Ok(())` assertion (via `.expect(…)`)
confirms no error is returned for a legitimately absent file.

**Good name?**  
Yes.

**Parameterizable?**  
Not a productive axis; the behavior is binary (deleted or not).

---

### `watcher_processing_only_indexes_nested_markdown_paths`

**What it does**: Creates an `ignored.txt` (with WikiLink content) and a `watched.md` in a nested directory,
passes both through `process_pending_paths_for_test`, and asserts the `.md` file was indexed and the `.txt`
file was not.

**Load-bearing?**  
Yes — the markdown-only filter in `process_pending_paths_for_test` (via `path_to_memory_uri` and
`is_watchable_markdown_path`) is a correctness boundary. Without it, arbitrary file types could pollute the
index. No other test in this file or the boundary files covers the non-markdown filter in the batch path.

**Unique?**  
Yes — this is the only test in the `incremental_sync_tools_*` group asserting non-markdown paths are silently
ignored during batch processing.

**Correct assertions?**  
Mostly. The test asserts `file_row = Some(watched_uri)` and `count_rows_like("file_content", "%ignored.txt%") = 0`.
It does NOT assert that `[[ignored-concept]]` (from the txt file) is absent from `concept_mentions`. This is
a minor gap: if some future code path inserted concept mentions without going through `file_content`, the test
would not catch it. Acceptable given the current architecture.

**Good name?**  
Misleading. The test name emphasises "nested" as if the subdirectory structure is the subject. The actual
subject is **file-type filtering** (`ignored.txt` vs `watched.md`). A name like
`watcher_processing_ignores_non_markdown_files` would be more precise.

**Parameterizable?**  
Yes — this could be table-driven over a set of non-markdown extensions (`txt`, `json`, `yaml`, etc.) to
prove the filter is exhaustive. Currently tests only one non-markdown type.

---

### `debounce_rapid_changes_to_same_path_coalesces_into_final_processing`

**What it does**: Creates a file (`concept_v1`), writes 5 versions over it (each with `concept_final`),
collects all 5 path entries into a `Vec`, calls `process_pending_paths_for_test` with the 5-element vec,
and asserts `concept_final = Some(1)` and `concept_v1 = None`.

**Load-bearing?**  
The *intent* is load-bearing: path deduplication in `process_pending_paths_for_test` prevents redundant
syncs when a file changes rapidly. But the test **does not actually prove deduplication is occurring**. It
proves only that after processing the deduped path set, the final state is correct — which would be equally
true even if the code processed the file 5 times instead of 1.

**Vacuous assertion — `concept_v1 = None`**: `concept_v1` was NEVER indexed. The initial `create_test_file`
just writes the file to disk; `process_path_for_test` is NOT called after creation. The loop immediately
overwrites the file 5 times with `concept_final` content. When `process_pending_paths_for_test` runs, the
file already contains only `concept_final`. Asserting `concept_v1 = None` cannot fail and provides zero
signal.

**What the test should assert instead**: To verify coalescing, `concept_v1` should be indexed FIRST, then
the rapid updates applied. The test should then assert `concept_v1 = None` (cleaned up by the update) AND
`concept_final = Some(1)`. As currently written, `concept_v1` was never in the database to begin with.

**Unique?**  
The only test for path-deduplication semantics. The concept is unique even if the execution is flawed.

**Correct assertions?**  
`concept_final = Some(1)` is correct. `concept_v1 = None` is vacuously true. The test does NOT verify that
the underlying sync function was called exactly once (which would be the real proof of coalescing).

**Good name?**  
The name claims to test "coalescing into final processing." The test only proves the final state is correct
for a never-previously-indexed file. Name overstates the guarantee.

**Parameterizable?**  
The "5 rapid changes" count is hardcoded. A parameterized version could test N=1, N=5, N=100 to confirm
the deduplicated count is always 1 call regardless of input size. More valuable than what exists.

---

## Verdicts

| # | Test Name | Load-Bearing | Unique | Assertions Correct | Good Name | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|---------|
| 1 | `process_file_created_with_wikilinks_inserts_concept_mentions_into_db` | ✅ (partial — overlaps unification test) | ⚠️ | ✅ | ✅ | **MERGE** |
| 2 | `process_file_changed_with_new_content_adds_mention_for_new_concept_and_updates_file_content` | ✅ | ✅ | ❌ (missing `concept_a = None`) | ⚠️ | **REWRITE** |
| 3 | `process_file_deleted_after_create_removes_file_content_and_mentions_from_db` | ✅ | ✅ | ✅ | ✅ | **KEEP** |
| 4 | `watcher_processing_only_indexes_nested_markdown_paths` | ✅ | ✅ | ✅ | ⚠️ | **KEEP** (rename) |
| 5 | `debounce_rapid_changes_to_same_path_coalesces_into_final_processing` | ⚠️ (vacuous assertion) | ✅ | ❌ (`concept_v1` was never indexed) | ❌ | **REWRITE** |

---

## Recommendations

### P0 — Immediate fixes (correctness gaps that could mask regressions)

**1. Rewrite Test 2 — add the missing old-concept removal assertion** (T-12.7 · NFR-1)

After the update, `concept_a` should be absent from `concept_mentions`. Add:

```rust
assert_eq!(
    get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_a)),
    None,
    "concept_a should be removed after content update that no longer references it"
);
```

Without this, any bug that causes concept mentions to accumulate rather than reconcile will not be caught
by this test.

**2. Rewrite Test 5 — fix the vacuous `concept_v1 = None` assertion** (T-12.7 · NFR-1)

Index `concept_v1` first, then apply rapid overwrites. The corrected structure:

```rust
// Index the initial state so concept_v1 is actually in the DB
process_path_for_test(&ctx, &file_path).expect("index initial v1");
assert_eq!(
    get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_v1)),
    Some(1),
    "precondition: concept_v1 must be indexed before dedup test"
);

// Force re-processing by backdating last_indexed
set_last_indexed(&cfg, &memory_uri, "2000-01-01T00:00:00+00:00");

// Now apply rapid overwrites — same as before
let mut pending_events = Vec::new();
for version in 0..5 {
    std::fs::write(&file_path, format!("# Final {version}\n\n[[{concept_final}]]\n")).unwrap();
    pending_events.push(file_path.clone());
}

process_pending_paths_for_test(&ctx, &pending_events).expect("process debounced paths");

// NOW concept_v1 = None is meaningful: it was in the DB and should have been reconciled
assert_eq!(
    get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_v1)),
    None,
    "concept_v1 should be reconciled away by the update"
);
assert_eq!(
    get_mention_count(&cfg, &memory_uri, &normalize_concept(&concept_final)),
    Some(1)
);
```

### P1 — Structural improvements (signal quality, not correctness)

**3. Merge Test 1 into the unification suite OR strengthen its entry-point justification**

`process_file_new_file_inserts_content_mentions_and_index_rows` in `sync_unification_single_file.rs`
already covers this scenario with identical DB assertions plus output-text verification. If this test is
kept, it should assert something the unification test does not — e.g., that `process_path_for_test`
(the direct test shim, bypassing leader election) behaves identically to the higher-level path. Add a
comment explaining this distinction. Otherwise, merge the two tests.

**4. Rename Test 4** to reflect its actual subject:

```
watcher_processing_ignores_non_markdown_files_in_batch
```

The word "nested" describes the fixture setup, not the behaviour under test.

**5. Extend Test 4** — add one non-markdown extension variant (`.json` or `.yaml`) to confirm the filter
is not accidentally hardcoded to only ignore `.txt`.

### P2 — Observability improvements

**6. Add a sync-call-count assertion to Test 5** after the rewrite, to prove coalescing is actually
happening and not just that the final state is correct:

Consider exposing a test-only `sync_call_counter` atomic or using a mock/spy at the `sync_single_file`
boundary, so the test can assert the underlying function was called exactly once for N duplicate path entries.

---

## Load-Bearing Score: 3/5

Tests with correct, non-vacuous assertions that would catch the regression they claim to cover:

| Test | Load-Bearing & Correct |
|------|:---:|
| 1 — file created, mentions inserted | ✅ (correct, but overlapped by stronger test) |
| 2 — file changed, new concept added | ❌ (missing removal assertion) |
| 3 — file deleted, content + mentions removed | ✅ |
| 4 — non-markdown files ignored | ✅ |
| 5 — debounce coalesces to final state | ❌ (vacuous `concept_v1 = None`) |

**3 of 5 tests would reliably catch the regression they describe. 2 of 5 would silently pass in the
presence of the specific bug they are meant to prevent.**
