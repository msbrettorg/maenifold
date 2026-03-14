# Test Evaluation: access_boosting_build_context.rs

> **Traceability**: T-12.0 (RTM NFR-1), T-GRAPH-DECAY-002.1-3 (RTM NFR-7.6.1/7.6.2/7.6.3)
> **Evaluator**: Gray (Skeptical Inquiry)
> **File**: `crates/maenifold-core/tests/access_boosting_build_context.rs`
> **Date**: 2025-07

---

## Summary

This file contains **4 tests** covering a single behavioral contract: `BuildContext` MUST NOT
update `last_accessed` on files it traverses. The contract is correct and the overall test
strategy is sound. However, the file suffers from **redundancy** between tests 1 and 2
(both use `includeContent=true`, test an overlapping code path, and differ only in depth and
file count), a **misleading name** on test 3 ("various depths" but only exercises depth=3),
and a **cross-cutting structural weakness**: every test calls `read_memory_output` before
`set_last_accessed` as a pre-condition verification, but this fact is unannounced and adds
visual noise without a comment explaining the intent.

The core behavioral property being tested (`build_context` never calls `update_last_accessed`,
which is confirmed by source inspection — `update_last_accessed` has exactly **one** call site,
in `memory/access.rs`, and is never invoked from `graph/build_context.rs`) is correct. Tests
assert the right thing. The problem is that 3 tests assert essentially the same thing, while
the one genuinely comprehensive test (test 4) already subsumes the BuildContext
assertion from all three others.

---

## Verdicts

| # | Test Name | Verdict | Load-Bearing | Unique | Assertions Correct | Name Clear | Parameterizable |
|---|-----------|---------|:---:|:---:|:---:|:---:|:---:|
| 1 | `build_context_does_not_update_last_accessed` | **KEEP** | ✅ | ✅ | ✅ | ✅ | Partial |
| 2 | `build_context_with_content_preview_does_not_update_last_accessed` | **REMOVE** | ❌ | ❌ | ✅ | ⚠️ | Yes |
| 3 | `build_context_at_various_depths_does_not_update_last_accessed` | **REWRITE** | ⚠️ | ⚠️ | ⚠️ | ❌ | Yes |
| 4 | `only_read_memory_updates_last_accessed_not_search_or_build_context` | **KEEP** | ✅ | ✅ | ✅ | ✅ | No |

---

## Per-Test Analysis

---

### Test 1 — `build_context_does_not_update_last_accessed`

**RTM**: NFR-7.6.3 | **Verdict**: KEEP

#### What it does
Creates **two** files both mentioning `[[build-context]]`, sets their `last_accessed` to
5 and 4 days ago, runs `BuildContext` on `"build-context"` with depth=2 and
`includeContent=true`, then asserts neither file's `last_accessed` changed.

#### Strengths

- **Two-file coverage**: The only test in this file that verifies the non-update property
  holds for *all* files referenced by a traversed concept simultaneously — not just one.
  This exercises the multi-relation path in `expand_related_concepts`.
- **Graph connectivity guard**: The `assert!(!direct_relations.is_empty(), ...)` assertion
  acts as a pre-condition sentinel. If the sync or concept extraction pipeline fails
  silently, the test will catch it here before the main assertion can pass vacuously.
- **Correct assertion shape**: `set_last_accessed` is called before `get_last_accessed`
  (establishing a non-NULL controlled baseline), and the after-value is compared to the
  same controlled baseline — not to "now". This is the right structure.

#### Issues / Gray Skepticism

- **Unannounced read pre-condition**: `read_memory_output` is called before
  `set_last_accessed` on both files. This silently updates `last_accessed` to "now"
  before it is overridden. The intent (validate files exist and are readable) is not
  commented. A future reader may not understand why a read happens before the
  controlled baseline is set.

  > *How do we know this read is necessary?* It is defensive — `create_and_sync_test_file`
  > doesn't guarantee `last_accessed` is set, but `set_last_accessed` is a direct `UPDATE`
  > so it will set any value regardless. The read could be replaced with a simple
  > `assert!` on the URI's existence. The current code is not wrong, just noisy.

- **`includeContent=true` overlap with test 2**: Both tests use `includeContent=true`.
  The `includeContent` flag only governs JSON rendering — it has no pathway to
  `update_last_accessed`. Testing it twice for this behavioral property adds no coverage.

---

### Test 2 — `build_context_with_content_preview_does_not_update_last_accessed`

**RTM**: NFR-7.6.3 | **Verdict**: REMOVE

#### What it does
Creates **one** file mentioning `[[content-preview]]`, sets `last_accessed` to 3 days ago,
runs `BuildContext` on `"content-preview"` with `includeContent=true`, depth=1, and
asserts `last_accessed` is unchanged.

#### Why REMOVE

**This test is a strict subset of test 1.**

The name implies it is specifically testing the `includeContent=true` code path. However:

1. Test 1 *already* uses `includeContent=true`.
2. `includeContent` only changes the JSON rendering path inside `formatter::render_build_context_json`.
   It has zero influence on whether `update_last_accessed` is called. The source code
   confirms `update_last_accessed` is never called anywhere in `graph/build_context.rs`.
3. The depth=1 variation (vs depth=2 in test 1) does not exercise a meaningfully different
   code path for this property.

> *What would this test catch that tests 1 and 4 would not?* Nothing. If a developer
> accidentally added `update_last_accessed(&uri, context)` to the `includeContent=true`
> branch of `build_context`, test 1 (which also uses `includeContent=true`) would catch
> it first. Test 4 would also catch it in the combined behavioral sequence.

**The only unique aspect** is the depth=1 shallow traversal. If depth-specific coverage is
desired, test 3 should cover a different depth instead of replicating the `includeContent`
variation. If depth=1 is considered worth testing, merge it into a parameterized version
of test 3.

---

### Test 3 — `build_context_at_various_depths_does_not_update_last_accessed`

**RTM**: NFR-7.6.3 | **Verdict**: REWRITE

#### What it does
Creates **one** file mentioning `[[depth-traversal]]`, sets `last_accessed` to 3 days ago,
runs `BuildContext` on `"depth-traversal"` at depth=3 with `includeContent=false`, and
asserts `last_accessed` is unchanged.

#### Issues

1. **Name is false advertising**: "various depths" implies multiple depth values are tested.
   Only depth=3 is tested. The name should be `build_context_at_depth_3_does_not_update_last_accessed`
   or the test should genuinely parameterize across depths [1, 2, 3].

2. **Fixture assertion is trivial**: `context_result["data"]["Depth"].as_u64() == Some(3)`
   checks that the depth echoed back in the JSON envelope matches what was sent. This is
   not a meaningful behavioral assertion — it just confirms input round-trip.

3. **`includeContent=false` coverage is marginal**: The `includeContent=false` path skips
   content attachment in the JSON formatter. It does not open any new last_accessed update
   path. However, it IS the only test in the file using `includeContent=false`, which means
   removing it would leave only `includeContent=true` test coverage. For completeness,
   keep one test with `includeContent=false` — but do it better.

4. **Depth=3 is the single most useful depth variant**: The `expand_related_concepts`
   function loops up to `depth` hops. Testing depth=3 is genuinely different from depth=1
   or depth=2 because it exercises more iterations of the BTreeMap dedup + frontier
   expansion logic. The test is asking the right question; it just names and asserts it
   poorly.

#### Suggested Rewrite

```rust
// T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.3 — multi-hop traversal must not update last_accessed
#[test]
fn build_context_deep_traversal_does_not_update_last_accessed() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = create_and_sync_test_file(
        &env,
        "Deep Traversal Test",
        "Testing [[deep-traversal]] with [[graph-depth]] concepts.",
        TEST_FOLDER,
    );

    let baseline = rfc3339_days_ago(3);
    set_last_accessed(&env.cfg, &uri, &baseline);
    let initial = get_last_accessed(&env.cfg, &uri);

    // Depth=3 exercises the full hop-expansion loop in expand_related_concepts
    build_context_json(&env, "deep-traversal", 3, 50, false);

    let after = get_last_accessed(&env.cfg, &uri);
    assert_eq!(after, initial,
        "BuildContext at depth=3 (multi-hop expansion) must not update last_accessed");
}
```

Key changes: accurate name, remove trivial fixture assertion, add intent comment on depth choice,
drop the defensive read (unnecessary given direct `set_last_accessed` call).

---

### Test 4 — `only_read_memory_updates_last_accessed_not_search_or_build_context`

**RTM**: NFR-7.6.1/7.6.2/7.6.3 | **Verdict**: KEEP

#### What it does
Exercises the **complete access-boosting behavioral contract** in a single sequence:

1. Seed `last_accessed` to a known past value
2. Call `ReadMemory` → assert `last_accessed` advances to "now"
3. Call `SearchMemories` → assert `last_accessed` **unchanged**
4. Call `BuildContext` → assert `last_accessed` **unchanged**
5. Sleep 1.1s to ensure clock advances
6. Call `ReadMemory` again → assert `last_accessed` advances *further*

#### Why this is the most load-bearing test in the file

- **Positive + negative assertions in one flow**: It's the only test that proves ReadMemory
  *does* update, not just that BuildContext doesn't. This distinction matters because a
  test suite that only asserts the negative (nothing updates) would pass even if the whole
  `update_last_accessed` mechanism was deleted.
- **Temporal ordering assertion**: `after_second_read > after_first_read` proves that
  repeat reads continue advancing the field — a required property for the decay model to
  function correctly.
- **Subsumes tests 1–3's BuildContext assertion**: The `after_build_context` check here
  covers the same behavioral property as the other three tests.
- **The sleep is justified**: `READ_SLEEP_MS = 1_100` gives > 1 second gap so that the
  RFC 3339 timestamp (second granularity) strictly advances between reads.

#### Minor issues

- **No guard against `after_first_read == baseline`**: If `ReadMemory` somehow returned
  the old `last_accessed` value (e.g., the update was silently swallowed via `.ok()`),
  `after_first_read` could equal the seeded value. The test would then still fail at the
  final `after_second_read > after_first_read` assertion — but only after sleeping 1.1s,
  making failure diagnosis slower. Adding `assert!(after_first_read > get_last_accessed(&env.cfg, &uri).unwrap_or_else(|| rfc3339_days_ago(3).parse()...))` before the sleep would catch it earlier. In practice, the existing `access_boosting_reads.rs` already covers this case directly, so this is low priority.

- **`assert_search_contains_title` is a mixed concern**: It validates that the search
  actually found the file (functional coverage) before asserting the last_accessed check.
  This is correct defensive practice but could be extracted into a named setup helper to
  separate concerns.

---

## Recommendations

### Priority 1 — Delete test 2 (REMOVE)
`build_context_with_content_preview_does_not_update_last_accessed` duplicates test 1 and
test 4 entirely. Its name implies unique coverage of `includeContent=true` but that flag
has no pathway to `last_accessed`. Deleting it reduces noise without any coverage loss.

### Priority 2 — Rewrite test 3 (REWRITE)
Rename to `build_context_deep_traversal_does_not_update_last_accessed`. Remove the trivial
depth-echo assertion. Add a comment explaining why depth=3 is worth testing (multi-hop
expansion loop). Drop the unnecessary pre-read before `set_last_accessed`.

### Priority 3 — Add comments to test 1 (KEEP + minor hardening)
Annotate the `read_memory_output` calls as "verify files exist and are searchable in graph"
so the intent is clear. The pre-read before `set_last_accessed` is otherwise invisible to
future readers and could be misidentified as a side-effect dependency.

### Priority 4 — Consider a parameterized sweep (FUTURE)
Tests 1, 2, 3 collectively test depth ∈ {1, 2, 3} and `includeContent` ∈ {true, false}.
If Rust gains stable `#[test_case]` or similar rstest-style parameterization, these could
be collapsed into a matrix test with explicit documentation of which axes are meaningful
(depth matters; includeContent does not).

### Priority 5 — Source-level comment on `update_last_accessed` (SOURCE)
`update_last_accessed` has exactly one call site (`memory/access.rs:199`). A single line
comment on the function like:

```rust
/// Access-boosting write — called ONLY by ReadMemory. Must not be called from
/// SearchMemories, BuildContext, FindSimilarConcepts, or other read-only tools.
```

…would make the behavioral contract self-documenting at the source level, reducing the
need to maintain multiple test variants to guard against accidental call-site additions.

---

## Load-Bearing Score: 2.5 / 4

| Test | Score | Rationale |
|------|-------|-----------|
| `build_context_does_not_update_last_accessed` | 1.0 | Unique multi-file case + graph connectivity guard |
| `build_context_with_content_preview_does_not_update_last_accessed` | 0.0 | Fully subsumed by tests 1 and 4 |
| `build_context_at_various_depths_does_not_update_last_accessed` | 0.5 | Depth=3 variation is meaningful; execution is poor |
| `only_read_memory_updates_last_accessed_not_search_or_build_context` | 1.0 | Full behavioral contract; positive + negative + temporal ordering |

**Total: 2.5 / 4**

> *Gray interpretation*: The suite is not broken — all assertions are correct and no test
> passes vacuously. But 1 of 4 tests adds zero coverage, 1 of 4 asserts poorly, and 3 of
> 4 tests share the same core assertion that test 4 already makes more completely.
> The minimum viable suite for this behavioral contract is: **test 1 + test 4**,
> plus a correctly-named depth=3 test. Test 2 should be deleted without hesitation.
