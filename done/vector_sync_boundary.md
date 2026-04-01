---
id: T-80.172
title: "Test Evaluation: vector_sync_boundary"
status: done
pipeline: tdd
---

# Test Evaluation: vector_sync_boundary

**File**: `crates/maenifold-core/tests/vector_sync_boundary.rs`
**Traceability**: T-12.7 | NFR-1 (C# Oracle Parity)
**Evaluator**: Gray 🔘 — Skeptical Inquiry
**Method**: Full read of all 3 `#[test]` functions; cross-referenced against
`tool_registry_tests.rs`, `search_graph_boundary.rs`, `similar_concepts_plateau_tests.rs`,
`markdown_rw_extract_tests.rs`, `markdown_rw_sync_tests.rs`, and
`crates/maenifold-cli/tests/vector_sync_cli_tests.rs`.

---

## Summary

The file carries its own header comment: _"acceptance hygiene trims smoke-only sync boundary
tests that under-assert runtime behavior."_ That annotation is self-incriminating. Gray inquiry
takes it at face value and asks: does the evidence support it?

**Yes.** Two of three tests are under-asserting. One is entirely redundant. The file adds almost
zero net coverage to the test suite — the registrations it checks are a strict subset of
`tool_registry_tests.rs`, the markdown-extraction path it exercises is duplicated word-for-word
in the CLI integration suite, and only the FindSimilarConcepts pipeline test has a genuinely
unique integration path — but that test overpromises in its name and under-delivers in its
assertions.

The `isolated_root` / `TEST_MUTEX` scaffolding is correct and reusable, but no test exploits it
fully. The infrastructure cost exceeds the assertion yield.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| 1 | `vector_sync_tools_are_registered_case_insensitively` | ❌ No | ❌ No | ❌ No | ⚠️ Misleading | ✅ Yes | **REMOVE** |
| 2 | `find_similar_concepts_after_sync_should_surface_ranked_related_concepts` | ✅ Yes | ✅ Yes | ❌ No | ❌ False | ✅ Yes | **REWRITE** |
| 3 | `markdown_reader_sync_should_match_single_pass_extraction_contract` | ⚠️ Partial | ❌ No | ⚠️ Partial | ⚠️ Misleading | ✅ Yes | **MERGE** |

---

## Detailed Findings

---

### Test 1 — `vector_sync_tools_are_registered_case_insensitively`

#### What it does
Calls `ToolRegistry::global()` and asserts `registry.contains(tool)` for 10 string literals:
five canonical names (CamelCase) and their all-lowercase counterparts for the five vector/sync
tools (`FindSimilarConcepts`, `StartWatcher`, `StopWatcher`, `RunFullBenchmark`, `Sync`).

#### Questions the Gray lens asks

**What is `registry.contains()` actually testing?**
The registry normalises keys to lowercase on insertion. `contains("FindSimilarConcepts")` and
`contains("findsimilarconcepts")` therefore both resolve to a single lowercase key lookup. The
test is not verifying case-folding logic — it is verifying that the same normalised key exists
twice. No arbitrary mixed-case form (e.g. `"FindSIMILARconcepts"`) is tried.

**Is this coverage that doesn't exist elsewhere?**
No.
- `tool_registry_tests.rs::new_registry_registers_all_expected_tool_names` registers a
  `ToolRegistry::new()` (not global), iterates all 28 canonical names in a `BTreeSet`, asserts
  both set equality and `contains()` per name. This test's 5-tool subset is a strict subset.
- `tool_registry_tests.rs::try_invoke_case_insensitive_lookup_always_resolves` invokes
  `GetConfig` with four case variants including `gEtCoNfIg` and asserts on the *result output*,
  not just presence — a materially stronger assertion.
- `search_graph_boundary.rs::search_graph_tools_are_registered_case_insensitively` does the
  same `registry.contains()` pattern for `SearchMemories`, `Sync`, `buildcontext`.

**Does it use `isolated_root`?**
No. It uses `ToolRegistry::global()` without any filesystem isolation. Coupling to global state
makes it order-dependent and harder to reason about, yet it asserts nothing that depends on
runtime state.

**Verdict: REMOVE.**
Zero unique assertions; fully subsumed by `tool_registry_tests.rs`. The name's claim of
"case_insensitively" is technically inaccurate — it checks two predetermined forms, not the
case-fold property. The test exists only to confirm registration smoke, which is already
exhaustively covered.

---

### Test 2 — `find_similar_concepts_after_sync_should_surface_ranked_related_concepts`

#### What it does
Seeds two memory files linking `[[authentication]]` to `[[oauth2]]` and `[[jwt]]` respectively,
runs `Sync`, then invokes `FindSimilarConcepts` on `"authentication"` with `maxResults:5`.
Asserts the output contains the strings `"oauth2"` and `"jwt"`.

#### Questions the Gray lens asks

**Is the name honest?**
No. The word _"ranked"_ appears in the function name. No rank ordering is asserted. The test
would pass even if `oauth2` appeared last, if a hundred unrelated concepts preceded it, or if
both results carried a similarity score of 0.001. The ranking claim is unfalsified.

**What's actually being tested?**
Concept reachability through the tool pipeline after sync: WriteMemory → Sync →
FindSimilarConcepts. This pipeline path is genuinely unique — `similar_concepts_plateau_tests.rs`
calls `search::find_similar_concepts()` directly at the internal API level (injecting pre-seeded
`vec_concepts` rows), bypassing the full tool-dispatch and sync flow. This test is therefore the
only place the full tool-layer integration is exercised for this feature.

**Is the uniqueness sufficient to justify the test?**
Yes — but only if the assertions are brought up to the name's promise. Currently:
- `output.contains("oauth2")` — passes even if "oauth2" appears in an error message.
- `output.contains("jwt")` — same weakness.
- No assertion on score values, rank order, result count, or response format.
- `maxResults:5` is passed but never validated (what if 6 results come back?).
- The seed data creates two perfectly symmetric co-occurrence edges; there is no way to
  distinguish the ranked output from an unranked one with this data.
- Sync's degraded-state warnings are not checked — the test will pass silently even if sync
  partially failed.

**Verdict: REWRITE.**
Rename to `find_similar_concepts_after_sync_surfaces_co_occurring_concepts`. Add:
- Assertion that the output format matches the expected markdown/JSON contract.
- Assertion on score bounds (scores ∈ (0, 1]).
- A third seed with a unique co-occurrence to test relative ranking between two candidates.
- An assertion that `maxResults` is respected when more candidates exist.
- Sync success pre-condition check that is more specific than `contains("Sync complete:")`.

---

### Test 3 — `markdown_reader_sync_should_match_single_pass_extraction_contract`

#### What it does
Seeds a file with a WikiLink in body text, in a fenced code block, and in inline code. Runs
`Sync`. Calls `ExtractConceptsFromFile`. Asserts that `"authentication"` is present, and that
`"ignored-in-code"` and `"ignored-inline"` are absent.

#### Questions the Gray lens asks

**Is this coverage unique?**
No — at three layers simultaneously:

1. **Unit layer**: `markdown_rw_extract_tests.rs` has `extract_wikilinks_skips_code_blocks`,
   `extract_wikilinks_skips_fenced_code_block_with_language`, and
   `extract_wikilinks_skips_indented_code_blocks` — all testing the same exclusion rule at the
   parser level.

2. **Sync layer**: `markdown_rw_sync_tests.rs::sync_code_block_exclusion_ignores_wikilinks_in_fenced_code_blocks`
   tests the Sync tool's code-block filtering directly.

3. **CLI integration layer**: `vector_sync_cli_tests.rs::sync_and_extract_concepts_cli_should_exclude_code_block_wikilinks`
   uses **the exact same seed content** (`"ignored-in-code"`, `"ignored-inline"`,
   `"authentication"`) and makes **the exact same three assertions** — at the binary CLI level,
   which is a higher-fidelity integration test than this one.

This test's assertions are a subset of existing coverage at every level.

**Are the assertions correct?**
Partially. The negative assertions (`!contains("ignored-in-code")`) are well-formed. The
positive assertion is weak: `extract.contains("authentication")` passes even if authentication
appears in an error message or as a substring of another concept name. A stronger assertion
would check that `"authentication"` appears as a standalone concept entry in the structured
output.

**What does "single_pass" mean?**
Nothing testable. The test would pass regardless of whether extraction performs one pass or ten.
The name documents an architectural aspiration, not a test contract.

**Verdict: MERGE.**
Consolidate into `markdown_rw_sync_tests.rs` alongside its sibling
`sync_code_block_exclusion_ignores_wikilinks_in_fenced_code_blocks`. Rename to something that
describes the combined Sync+Extract pipeline contract without the unverifiable "single_pass"
claim. Tighten the positive assertion to verify concept-entry format rather than substring
presence. Remove the standalone file.

---

## Recommendations

| Priority | Action | Rationale |
|:---:|---|---|
| 1 | **Delete** `vector_sync_tools_are_registered_case_insensitively` | Zero unique coverage; `tool_registry_tests.rs` is exhaustive and tests actual invocation. |
| 2 | **Rewrite** `find_similar_concepts_after_sync_…` in-place | Only test of the full WriteMemory→Sync→FindSimilarConcepts tool pipeline. Worth keeping with honest name + ranking assertions. |
| 3 | **Merge** `markdown_reader_sync_…` into `markdown_rw_sync_tests.rs` | Eliminates exact duplication with CLI test and unit-level markdown tests. |
| 4 | If all three are removed/merged, **delete** `vector_sync_boundary.rs` | Empty file left behind is a maintenance hazard. |
| 5 | **Add** a `find_similar_concepts_rank_order_reflects_co_occurrence_weight` test | The file's stated purpose (sync boundary with semantic lookup) is only served if ranking is actually asserted. No test in the suite currently validates this. |

### For the rewritten Test 2 — suggested skeleton

```rust
#[test]
fn find_similar_concepts_after_sync_surfaces_co_occurring_concepts() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, _cfg) = isolated_root();
    let registry = ToolRegistry::global();

    // Seed: strong double co-occurrence for oauth2, single for jwt
    for payload in [
        json!({"title":"OAuth A","content":"Links [[authentication]] to [[oauth2]]."}),
        json!({"title":"OAuth B","content":"Links [[authentication]] to [[oauth2]]."}),
        json!({"title":"JWT A","content":"Links [[authentication]] to [[jwt]]."}),
    ] {
        let w = registry.try_invoke("WriteMemory", &payload, OutputFormat::Json).expect("write");
        let p: serde_json::Value = serde_json::from_str(&w).unwrap();
        assert_eq!(p["success"], true, "write: {w}");
    }

    let sync = registry.try_invoke("Sync", &json!({}), OutputFormat::Json).expect("sync");
    let sp: serde_json::Value = serde_json::from_str(&sync).unwrap();
    assert_eq!(sp["success"], true, "sync: {sync}");

    let output = registry.try_invoke(
        "FindSimilarConcepts",
        &json!({"conceptName":"authentication","maxResults":3}),
        OutputFormat::Json,
    ).expect("find similar");
    let op: serde_json::Value = serde_json::from_str(&output).unwrap();

    // Results list is present and bounded by maxResults
    let results = op["data"]["results"].as_array().expect("results array");
    assert!(results.len() <= 3, "maxResults not respected: {output}");

    // Both co-occurring concepts are surfaced
    let names: Vec<&str> = results.iter()
        .filter_map(|r| r["conceptName"].as_str())
        .collect();
    assert!(names.contains(&"oauth2"), "expected oauth2 in results: {output}");
    assert!(names.contains(&"jwt"), "expected jwt in results: {output}");

    // Stronger co-occurrence (oauth2 × 2) ranks above weaker (jwt × 1)
    let oauth2_pos = names.iter().position(|n| *n == "oauth2").unwrap();
    let jwt_pos = names.iter().position(|n| *n == "jwt").unwrap();
    assert!(oauth2_pos < jwt_pos, "oauth2 should rank above jwt: {names:?}");

    Config::reset_overrides();
}
```

---

## Load-Bearing Score: 1/3

| Test | Load-Bearing Verdict | Reason |
|---|:---:|---|
| `vector_sync_tools_are_registered_case_insensitively` | ❌ 0 | Fully covered by `tool_registry_tests.rs`; no unique assertions |
| `find_similar_concepts_after_sync_…` | ✅ 1 | Only test exercising full WriteMemory→Sync→FindSimilarConcepts tool pipeline |
| `markdown_reader_sync_…` | ❌ 0 | Near-identical coverage exists in CLI tests and unit markdown tests |

**1 of 3 tests is genuinely load-bearing.** The file's infrastructure (mutex, isolated root,
tool-pipeline invocation pattern) is sound. The assertions it wraps are not.

The file's own header comment — _"acceptance hygiene trims smoke-only sync boundary tests that
under-assert runtime behavior"_ — identifies the correct remediation. The evidence confirms it.
Act on it.

---

*Related concepts: [[test-quality]] [[maenifold-rs]] [[vector sync diagnostics]] [[NFR-1]] [[T-12.7]]*

---
## SWE Remediation
- REMOVE `vector_sync_tools_are_registered_case_insensitively`: confirmed fully subsumed by `tool_registry_tests.rs`; zero unique assertions
- REWRITE `find_similar_concepts_after_sync_should_surface_ranked_related_concepts` → `find_similar_concepts_after_sync_surfaces_co_occurring_concepts`: added asymmetric seed data (oauth2×2, jwt×1), structured JSON assertions on `data.results` array, score bounds ∈ (0,1], rank ordering verification, maxResults enforcement, sync error pre-condition check
- MERGE `markdown_reader_sync_should_match_single_pass_extraction_contract` → `markdown_rw_sync_tests.rs::sync_extract_excludes_wikilinks_in_all_code_contexts`: structural `BTreeSet` concept-set equality assertion instead of substring matching, combined fenced+inline code exclusion, mention count verification
- Commit: 094bbf1

---
## Red-team Audit

**Traceability**: T-12.7 | NFR-1

### REMOVE — `vector_sync_tools_are_registered_case_insensitively`
- ✅ Function definition deleted from `vector_sync_boundary.rs`
- ✅ Tombstone comment references T-12.7 and documents rationale (subsumed by `tool_registry_tests.rs`)
- ✅ No residual `#[test] fn` definition found; only comment mentions remain
- ✅ No unique coverage lost — eval confirmed zero unique assertions

### REWRITE — `find_similar_concepts_after_sync_surfaces_co_occurring_concepts`
- ✅ Renamed from `find_similar_concepts_after_sync_should_surface_ranked_related_concepts` per eval spec
- ✅ Asymmetric seed data: oauth2 × 2 co-occurrences, jwt × 1 — enables rank-order verification
- ✅ Structured JSON assertions: `OutputFormat::Json`, parsed response, `parsed["success"]`, `parsed["data"]["results"].as_array()`
- ✅ Score bounds: `similarity > 0.0 && similarity <= 1.0` for every result
- ✅ Rank ordering: `oauth2_pos < jwt_pos` asserts stronger co-occurrence ranks higher
- ✅ maxResults enforcement: `results.len() <= 3`
- ✅ Sync pre-condition strengthened: added `!sync.contains("ERROR")` alongside existing completion marker check
- ✅ Result field names (`r["name"]`, `r["similarity"]`) match actual `format_ranked_results_json()` output in `similar/mod.rs:262-268` — eval skeleton's `r["conceptName"]` was incorrect; SWE adapted correctly
- ✅ Test passes green: `cargo test -- vector_sync_boundary` → 1 passed

**Note**: Sync invocation kept as `OutputFormat::Markdown` with string-based check rather than JSON parse (eval skeleton used JSON). SWE comment explains: "Sync returns markdown regardless of output format." This is a pragmatic adaptation, not a deviation — the combined pair (completion marker + no ERROR) is more specific than the original single check.

### MERGE — `sync_extract_excludes_wikilinks_in_all_code_contexts`
- ✅ Moved to `markdown_rw_sync_tests.rs` alongside sibling `sync_code_block_exclusion_ignores_wikilinks_in_fenced_code_blocks`
- ✅ Renamed: no unverifiable "single_pass" claim; new name describes combined fenced+inline code-context exclusion
- ✅ Positive assertion tightened: `BTreeSet::from(["authentication".to_string()])` structural equality replaces `extract.contains("authentication")` substring check
- ✅ Negative assertions preserved: `!concepts.contains("ignored-in-code")`, `!concepts.contains("ignored-inline")`
- ✅ Mention count verification added: `mention_count(&env.cfg, &uri), 1`
- ✅ Uses existing `markdown_rw_sync_tests.rs` infrastructure (`isolated_env`, `sync_one`, `file_concepts`, `write_fixture`) — consistent with sibling tests in the same file
- ✅ Test passes green: all 7 `markdown_rw_sync` tests pass including the new merged test

**Fidelity note**: Original test used `ToolRegistry::try_invoke("ExtractConceptsFromFile", ...)` (tool dispatch). Merged version uses `file_concepts()` (direct DB query). This is an acceptable downgrade because (a) the eval itself concluded the CLI test `sync_and_extract_concepts_cli_should_exclude_code_block_wikilinks` covers the full tool-dispatch path at higher fidelity, and (b) the eval directed merging into this file which uses internal helpers throughout.

### Unrelated Changes
- ✅ None detected. Commit touches exactly 2 files: `vector_sync_boundary.rs` and `markdown_rw_sync_tests.rs`

### Priority Coverage
- P1 (Delete redundant registration test): ✅ Addressed
- P2 (Rewrite FindSimilarConcepts with honest assertions): ✅ Addressed
- P3 (Merge markdown extraction test): ✅ Addressed
- P4 (Delete file if empty): N/A — file retained with 1 remaining test (correct)
- P5 (Add rank-order-reflects-co-occurrence-weight test): Not in scope — eval recommendation 5 was aspirational, not a verdict action item

### Verdict: PASS

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- vector_sync_boundary`: 1 passed, 0 failed
- `cargo test --test integration_tests -p maenifold-core -- markdown_rw_sync_tests::sync_extract_excludes_wikilinks_in_all_code_contexts`: 1 passed, 0 failed
- Notes: `vector_sync_boundary` is not a standalone test binary; all `maenifold-core` integration tests compile into the `integration_tests` binary. Both the rewritten test (`find_similar_concepts_after_sync_surfaces_co_occurring_concepts`) and the merged test (`sync_extract_excludes_wikilinks_in_all_code_contexts`) pass green.
- Verdict: PASS
