# Test Evaluation: repair_concepts_communities

**File**: `crates/maenifold-core/tests/repair_concepts_communities.rs`  
**Evaluator**: Red Team — Gray (Skeptical Inquiry)  
**Traceability**: T-12.0 · NFR-1 (C# Oracle Parity)  
**Date**: 2025-01-31  
**Suite status at evaluation time**: 3 pass · **1 FAIL** · 0 ignored

---

## Summary

This file contains 4 integration tests that exercise community detection plumbing through the
full WriteMemory → Sync → BuildContext tool stack. All 4 tests are tagged `// T-12.0: RTM NFR-1`
and live in a file named `repair_concepts_communities.rs`.

**Critical finding**: **Test 3 is currently broken** and has been silently failing. The assertion
`contains("Community Siblings")` uses a capital `S`, but the production formatter in
`graph/formatter.rs:129` emits `"Community siblings"` (lowercase `s`). The test fails at the first
`assert!` before it ever reaches the `contains("lambda")` check — meaning the positive sibling
content check has never run in CI. `cargo test` confirms: `build_context_with_community_data_shows_community_siblings_section` **PANICS**.

**Structural concern**: Not one of the 4 tests invokes `RepairConcepts`. They all sit in a file
prefixed `repair_concepts_*` because they reuse `repair_concepts/support.rs` for its fixture
helpers (`seed_community_graph`, `sync_graph`, `isolated_config`). The community behavior being
tested is BuildContext + Sync, not RepairConcepts. The file is therefore misnamed and
misfiled — a navigability problem for anyone maintaining the community test corpus, which already
spans 10+ dedicated `community_*.rs` files.

**Overlap**: The dedicated community test corpus (`community_sync.rs`, `community_build_context_siblings.rs`,
`community_build_context_degradation.rs`) covers structurally similar ground via `detect_and_persist`
(direct DB manipulation) and JSON output format. These 4 tests add distinct value by exercising the
**tool-layer integration path** (WriteMemory → Sync → BuildContext) and checking **Markdown text
rendering** rather than JSON schema. That distinction is real and worth preserving.

---

## Verdicts

| # | Test Name | Load-Bearing | Unique | Assertions Correct | Good Name | Status | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| 1 | `community_detection_populates_non_zero_community_count_after_sync` | ✅ | ✅ | ⚠️ partial | ✅ | PASS | **KEEP** |
| 2 | `build_context_shows_real_community_ids_not_hardcoded_zero` | ✅ | ✅ | ✅ | ✅ | PASS | **KEEP** |
| 3 | `build_context_with_community_data_shows_community_siblings_section` | ✅ | ✅ | ❌ broken | ✅ | **FAIL** | **REWRITE** |
| 4 | `build_context_gracefully_degrades_with_empty_concept_communities_table` | ✅ | ✅ | ✅ | ✅ | PASS | **KEEP** |

---

## Per-Test Analysis

### Test 1 — `community_detection_populates_non_zero_community_count_after_sync`

**What it tests**: After calling `seed_community_graph()` (5 WriteMemory calls) and running Sync,
the test asserts (a) that the Sync Markdown output contains the literal string
`"COMMUNITIES detected"` and (b) that `COUNT(DISTINCT community_id)` in the DB is `> 0`.

**Load-bearing?** Yes. This is the only test in this file that exercises the Sync output text
for the community count line. If community detection stops running during Sync, both assertions
catch it. The DB assertion (`community_count > 0`) is robust; the string assertion is a useful
parity guard but fragile.

**Unique?** Partially. `community_sync.rs :: sync_produces_community_assignments` covers Sync
community population with named-concept assertions at the DB level. This test adds Markdown
output text verification (`"COMMUNITIES detected"`) that the dedicated community suite doesn't
have. The fixture also differs: `seed_community_graph` uses WriteMemory tool calls rather than
direct DB manipulation, making this test a higher-fidelity integration check.

**Correct assertions?** The `community_count > 0` assertion is unambiguous. The
`contains("COMMUNITIES detected")` assertion is coupled to the exact wording in
`sync_state.rs:145`: `"- {} COMMUNITIES detected (modularity 0.000)"`. This is user-facing
Markdown output (NFR-1 parity target), so the coupling is intentional — but it will create a
false failure if the log line is reworded for cosmetic reasons, even if behavior is unchanged.
The assertion provides no tolerance for variations in capitalization or punctuation.

**Parameterizable?** The fixture is hardcoded to 5 WriteMemory calls producing a specific
topology. The test could be parameterized over different graph sizes/topologies, but as a smoke
test of the Sync → community detection pipeline, a single representative fixture is appropriate.

**Verdict**: **KEEP**. The `contains("COMMUNITIES detected")` assertion is noted as fragile but
acceptable for its NFR-1 parity purpose. No behavioral changes recommended.

---

### Test 2 — `build_context_shows_real_community_ids_not_hardcoded_zero`

**What it tests**: After seeding and syncing, loads omega's `community_id` directly from
`concept_communities` and then invokes BuildContext on omega. Asserts:
1. `omega_community_id > 0` (fixture pre-condition: omega is not in the first community)
2. BuildContext Markdown contains `[community {omega_community_id}]` (real ID rendered)
3. BuildContext Markdown does NOT contain `[community 0]` (not hardcoded)

**Load-bearing?** Yes. This is a targeted regression guard against a specific historical bug
(community IDs hardcoded to 0 in Markdown rendering). The test structure is correct: the
pre-condition assertion on `omega_community_id > 0` ensures the negative assertion
`!contains("[community 0]")` is non-vacuous. Without the pre-condition, if the algorithm
happened to assign omega to community 0, the negative assertion would pass while the bug exists.

**Unique?** Yes. The dedicated community test corpus verifies community IDs via JSON output
(`community_sync.rs :: build_context_includes_community_id_on_related_concepts` checks
`CommunityId` in JSON). No other test verifies the Markdown text format
`[community {id}]` from `formatter.rs:103`. This test closes a real parity gap.

**Correct assertions?** The three assertions are well-layered and logically sound. The pre-condition
guards the main assertions. No vacuous paths.

**Parameterizable?** The concept name "omega" is hardcoded and its expected community placement
is an implicit assumption of the fixture topology. The fixture is designed so omega ends up in a
different community from alpha/beta/gamma — this is documented by the assertion. Low priority
parameterization candidate.

**Verdict**: **KEEP**. Well-designed regression test. No changes recommended.

---

### Test 3 — `build_context_with_community_data_shows_community_siblings_section`

**What it tests**: After seeding and syncing, invokes BuildContext on omega and asserts that the
Markdown output contains `"Community Siblings"` (section header) and `"lambda"` (a sibling
concept expected in the same community).

**Load-bearing?** Yes — in principle. This is the only test checking that the Community siblings
section appears in Markdown output for a concept that has siblings. However, the test is
**currently broken** and has never exercised the `contains("lambda")` assertion.

**The bug**: `formatter.rs:129` emits `"Community siblings"` (lowercase `s`). The test asserts
`contains("Community Siblings")` (uppercase `S`). Rust string matching is case-sensitive. The
assertion fails unconditionally whenever community siblings are present in the output. The test
is not protecting the feature it claims to protect.

**Secondary assertion problem**: `contains("lambda")` is too coarse. It asserts that the string
"lambda" appears somewhere in the BuildContext output — but "lambda" could appear in relation
text, file references, or other sections. The assertion does not verify that lambda appears
specifically within the Community siblings section. A concept named "lambda" appearing as a
direct relation (which it doesn't in this fixture, but could in a different scenario) would cause
the assertion to pass even if the siblings section is absent.

**Correct assertions?** No. Two problems: (1) broken case mismatch on the section header
assertion; (2) `contains("lambda")` is insufficiently scoped to verify the siblings section
specifically.

**Relationship to dedicated sibling tests**: `community_build_context_siblings.rs ::
build_context_returns_community_siblings` tests the same feature via JSON output format with
precise field-level assertions (`CommunitySiblings` array, `NormalizedOverlap` values). This
test adds Markdown rendering verification. That's a valid and distinct concern — but only if
the assertions are correct.

**Parameterizable?** The expected sibling "lambda" is fixture-specific. Low priority.

**Verdict**: **REWRITE**. Fix the case mismatch. Tighten the `contains("lambda")` assertion to
verify lambda appears after the section heading (e.g., assert the substring
`"Community siblings"` appears before `"lambda"` in the output, or assert that the output
contains `"lambda (overlap:"` which is the sibling entry format from `formatter.rs:134`).

```rust
// Proposed rewrite of the two assertions:
assert!(
    context.contains("Community siblings"),   // lowercase 's' matches formatter.rs:129
    "expected sibling section in BuildContext output: {context}"
);
assert!(
    context.contains("lambda (overlap:"),     // scoped to sibling entry format (formatter.rs:134)
    "expected lambda as community sibling in BuildContext output: {context}"
);
```

---

### Test 4 — `build_context_gracefully_degrades_with_empty_concept_communities_table`

**What it tests**: After seeding and syncing, hard-deletes all rows from `concept_communities`,
then invokes BuildContext on omega. Asserts:
1. Output still contains `"rho"` (direct relations unaffected)
2. Output does NOT contain `"[community "` (no community IDs rendered)
3. Output does NOT contain `"Community Siblings"` (sibling section suppressed)

**Load-bearing?** Yes. This tests graceful degradation of the Markdown rendering path when
community data is absent. Critically: it also provides implicit evidence that Test 3's
`contains("Community Siblings")` assertion is targeting the wrong string. If the negative
assertion in Test 4 (`!contains("Community Siblings")`) were using the same string as the
positive assertion in Test 3 (`contains("Community Siblings")`), both would always pass —
for different reasons (Test 4 because the section is absent; Test 3 because the string never
matches). The two tests would be uncorrelated noise. Fortunately, Test 4 passes correctly
because the section header is absent in the degraded case regardless of capitalization.

**Wait — does Test 4's negative assertion catch the bug?** No. Test 4 asserts
`!contains("Community Siblings")` in a state where there are no siblings. Since the formatter
only emits `"Community siblings"` (lowercase), `contains("Community Siblings")` is always
false — the negative assertion always passes vacuously, whether or not the degradation logic
is working. **Test 4's third assertion provides false confidence.** It would pass even if the
degradation code were completely removed, because the string it checks for was never emitted.

**Correct assertions?** Mostly. The `contains("rho")` assertion is solid. The
`!contains("[community ")` assertion is solid (lowercase, matches formatter format). The
`!contains("Community Siblings")` assertion is vacuously true due to the case mismatch bug —
but it becomes meaningful once Test 3 is fixed, because it uses the same wrong string. The
fix: change it to `!contains("Community siblings")` (lowercase) to be semantically accurate.

**Verdict**: **KEEP** with minor correction to the third negative assertion: change
`"Community Siblings"` → `"Community siblings"` for consistency with the production format.

---

## Cross-Cutting Observations

### File Misplacement (Non-Blocking)
None of the 4 tests invoke `RepairConcepts`. The file name `repair_concepts_communities.rs`
and its use of `repair_concepts/support.rs` implies these tests are part of the RepairConcepts
test domain. They are not. The tests belong conceptually alongside `community_build_context_*.rs`
and `community_sync.rs`. This creates a navigability hazard: someone auditing community feature
coverage will not find these tests by scanning `community_*.rs`.

**Recommendation**: Move to `community_via_tool_pipeline.rs` or similar, and create a thin
re-export from a new `community_integration/support.rs` rather than re-using
`repair_concepts/support.rs`. Not blocking; traceability and the mutex work correctly as-is.

### Duplicate Setup (Tests 2 and 3)
Tests 2 and 3 run **identical setup code**: `seed_community_graph()` + `sync_graph()` +
`BuildContext on "omega"`. The setup calls are copy-pasted. If the fixture changes (e.g., omega
is renamed, the graph topology is modified, or `isolated_config` semantics change), two test
bodies must be updated in lock-step. Consider a shared helper `seeded_omega_context() -> String`
that returns the BuildContext output for the standard community fixture.

### Assertion Coupling to Internal Format Strings
Three tests check for exact substrings from production log/format strings:
- `"COMMUNITIES detected"` → `sync_state.rs:145`
- `"[community {id}]"` → `formatter.rs:103`  
- `"Community siblings"` → `formatter.rs:129`

These are appropriate for NFR-1 parity checks (C# oracle string matching). However, they create
implicit coupling: any cosmetic rewording of these strings requires corresponding test updates.
This is an acceptable trade-off for NFR-1 compliance, but the coupling should be made explicit
with code comments that reference the source lines being asserted against.

---

## Recommendations

| Priority | Action | Blocking |
|----------|--------|:---:|
| 🔴 **Critical** | Fix `build_context_with_community_data_shows_community_siblings_section`: change `"Community Siblings"` → `"Community siblings"` (lowercase `s`) in the first assertion. | Yes — test currently FAILS |
| 🔴 **Critical** | Tighten the `contains("lambda")` assertion to be section-scoped: use `contains("lambda (overlap:")` to verify lambda appears as a sibling entry, not just anywhere in the output. | Yes — current form provides weak coverage |
| 🟡 **Important** | Fix vacuous negative assertion in Test 4: change `!contains("Community Siblings")` → `!contains("Community siblings")` (lowercase). Currently passes but for the wrong reason. | No — test passes; fix is semantic correctness |
| 🟡 **Important** | Extract shared `seeded_omega_context()` helper to eliminate copy-paste setup in Tests 2 and 3. | No — cosmetic, reduces maintenance burden |
| 🟢 **Minor** | Move file to `community_via_tool_pipeline.rs` or rename to reflect BuildContext/Sync scope. Add comments pointing to `formatter.rs` line numbers for string assertions. | No — navigability improvement |

---

## Recommendations: Rewrite of Test 3

```rust
// T-12.0: RTM NFR-1
// Verifies: Markdown output includes "Community siblings" section header and
// lambda appears as a scored sibling entry (formatter.rs:129, formatter.rs:134).
#[test]
fn build_context_with_community_data_shows_community_siblings_section() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, _cfg) = isolated_config();
    seed_community_graph();
    let _sync = sync_graph();

    let context = ToolRegistry::global()
        .try_invoke(
            "BuildContext",
            &json!({"conceptName":"omega","depth":1}),
            OutputFormat::Markdown,
        )
        .expect("invoke BuildContext");

    // formatter.rs:129 — section header uses lowercase 's'
    assert!(
        context.contains("Community siblings"),
        "expected sibling section in BuildContext output: {context}"
    );
    // formatter.rs:134 — sibling entry format includes '(overlap:'; scoped check prevents
    // false positives from lambda appearing elsewhere in the output.
    assert!(
        context.contains("lambda (overlap:"),
        "expected lambda as a scored community sibling entry: {context}"
    );
}
```

---

## Load-Bearing Score: 4/4

All 4 tests guard distinct behavioral properties:

| Test | Property Protected |
|------|--------------------|
| 1 | Sync populates `concept_communities` and emits community count in Markdown output |
| 2 | BuildContext Markdown renders real `community_id` values, not hardcoded `0` |
| 3 | BuildContext Markdown includes `Community siblings` section when siblings exist *(currently broken)* |
| 4 | BuildContext Markdown degrades gracefully when `concept_communities` is empty |

Removing any one of these tests deletes coverage that is not replicated by the dedicated
`community_*.rs` suite (which operates at the JSON layer and DB layer, not the Markdown
rendering layer). Despite the file misplacement and the broken test, the suite justifies its
existence. The fix cost is one line change.

---

*Evaluated under [[test-quality]] standards for [[maenifold-rs]] · [[NFR-1 C# Oracle Parity]] · [[community detection]]*
