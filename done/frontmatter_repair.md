---
id: frontmatter_repair
status: done
---

# Test Evaluation: frontmatter_repair
**File:** `crates/maenifold-core/tests/frontmatter_repair.rs`
**Test count:** 3
**Evaluated:** 2026-07-13
**Traceability:** T-META-001.6 (RTM FR-24.4) · T-12.6 (RTM NFR-1)

> ⚠️ **Traceability gap:** `T-META-001.6` does not appear in `TODO.md` or `RTM.md`. It is cited only
> in code comments across the `frontmatter_*` test files. Verify whether this task ID was retired
> to `CHANGELOG.md` or was never formally registered. `T-12.6` appears in `CHANGELOG.md` (completed
> sprint infrastructure split work) and is the correct attribution for the file-level split rationale.

---

## Summary

This file contains exactly three tests, all targeting the intersection of `repair_concepts` and
frontmatter IO. The intent is sound: repair is a write path that bypasses `write_markdown`, so
its interaction with frontmatter fields deserves focused regression coverage. In practice, two of
the three tests fail to prove their stated intent.

**Test 1 (`fr24_4_ac1_concept_repair_modified_updated`)** is the most damaging case. The test
name declares that `modified` is *updated* by repair. The setup includes a deliberate 20 ms sleep
— a clear signal the author intended to verify timestamp advance. But neither `assert_ne!` nor any
comparison between before/after timestamps is present. The actual assertions are
`!modified_after.is_empty()` and `!modified_before.is_empty()`, which are trivially true from
`write_markdown`'s own invariants. A `grep` of `repair/api.rs` confirms the repair path uses
`fs::write()` directly — it never calls `write_markdown`, never touches frontmatter, and never
updates `modified`. The test therefore cannot catch the regression it names. It is also unable to
catch the inverse regression: if repair were changed to incorrectly overwrite `modified`, this test
would not notice.

**Test 2 (`fr24_4_ac2_concept_repair_frontmatter_preserved`)** has real value in its intent —
verifying that repair's raw `fs::write` path does not destroy existing frontmatter fields — but
misses its own setup. Four fields are written (`title`, `type`, `status`, `tags`). Three are
checked post-repair. `tags` is silently omitted from assertions. A regression that specifically
corrupted sequence-type frontmatter fields (YAML sequences, not scalars) would pass this test
undetected. The test also contains no assertion that the WikiLink replacement actually ran;
a no-op repair would produce identical post-repair frontmatter, making it impossible to distinguish
"repair ran cleanly" from "repair found nothing to do."

**Test 3 (`fr24_4_ac3_concept_repair_replacements_correct`)** is the only test in this file with
a genuine regression signal. It checks that both `[[tools]]` and `[[Tools]]` are replaced,
exercising the case-insensitive regex path in `replace_wikilink_variants`. This is real behavior
that could regress. However, the test is **structurally misplaced**: it does not test frontmatter
at all (the file has no frontmatter), making it a content-replacement test filed in a frontmatter
test suite. A semantically equivalent, more rigorous version exists in `repair_concepts_batch.rs`
(`repair_concepts_case_insensitive_matching`), which tests three case variants and asserts exact
replacement count. Test 3 tests two case variants and does not assert count — it is a weaker,
duplicate test in the wrong file.

---

## Verdicts

| # | Test Function | Verdict | Severity |
|---|---|---|---|
| 1 | `fr24_4_ac1_concept_repair_modified_updated` | **REWRITE** | 🔴 Critical |
| 2 | `fr24_4_ac2_concept_repair_frontmatter_preserved` | **REWRITE** | 🟡 Medium |
| 3 | `fr24_4_ac3_concept_repair_replacements_correct` | **MERGE** | 🟡 Medium |

---

## Test-by-Test Analysis

### Test 1 — `fr24_4_ac1_concept_repair_modified_updated`

**Verdict: REWRITE**

**What the test name claims:** `repair_concepts` updates the `modified` frontmatter timestamp.

**What the test asserts:**
```rust
assert!(!modified_after.is_empty());
assert!(!modified_before.is_empty());
```

Both assertions are trivially satisfied by `write_markdown`'s own behavior (it always writes
`modified = now_iso()`). The test never asserts `modified_after != modified_before`.

**Confirmed implementation behavior:** `repair/api.rs` line 144 is `fs::write(&path, updated_content)`.
There is no call to `write_markdown`, no frontmatter manipulation, no `modified` field update
anywhere in the repair pipeline. After repair, `modified` will be byte-for-byte identical to
before repair.

**Consequence:** The test has two mutually exclusive interpretations, and neither is tested:
- If intent is **"repair updates `modified`"** — the assertion is missing `assert_ne!` and
  the implementation does not support the behavior. The 20 ms sleep is evidence of this intent.
  Either the behavior needs implementing and the assertion needs completing, or the test documents
  a design decision that was never built.
- If intent is **"repair preserves `modified`"** — the test name is wrong, the sleep is
  pointless, and the assertion should be `assert_eq!(modified_after, modified_before)` to
  actually verify preservation.

**Recommendation:** Determine the correct design intent, then either:
- **If repair should update `modified`**: implement it via `write_markdown` (or equivalent
  frontmatter patch path), then add `assert_ne!(modified_after, modified_before, "repair should advance modified timestamp")`.
- **If repair should preserve `modified`**: rename to `fr24_4_ac1_concept_repair_modified_preserved`,
  remove the sleep, and assert `assert_eq!(modified_after, modified_before, "repair must not alter modified timestamp")`.

---

### Test 2 — `fr24_4_ac2_concept_repair_frontmatter_preserved`

**Verdict: REWRITE**

**What the test covers (correctly):** Title, type, and status survive a repair that uses raw
`fs::write`. This would catch a regression that completely zeroed or overwrote frontmatter.

**What the test misses:**

1. **`tags` field is silently unchecked.** Four fields are written; only three are read back.
   `tags` is a YAML sequence (`YamlValue::Sequence`), distinct from the string scalars. A bug
   in the frontmatter serialization path that corrupts sequences (while preserving scalars) would
   pass this test. Add:
   ```rust
   let tags = frontmatter.get("tags").expect("tags should be present");
   assert!(matches!(tags, YamlValue::Sequence(_)), "tags should remain a sequence: {tags:?}");
   ```

2. **No confirmation that repair actually ran.** The test calls `repair_concepts` but never
   checks its output or verifies the content changed. If no file matched the repair criteria
   (e.g., a bug in folder scoping), the test would silently pass with original frontmatter
   intact. Add an assertion against the output string OR assert the post-repair content
   contains `[[tool]]` and not `[[tools]]`.

---

### Test 3 — `fr24_4_ac3_concept_repair_replacements_correct`

**Verdict: MERGE into `repair_concepts_batch.rs`**

**What the test covers:** Case-insensitive WikiLink replacement (`[[tools]]` and `[[Tools]]`
both replaced by `[[tool]]`). This is real behavior worth testing.

**Why it belongs elsewhere:** The file is `frontmatter_repair.rs` but this test creates a file
with *no frontmatter* and asserts nothing about frontmatter. It is a content-replacement test.
The `repair_concepts_batch.rs` file already contains
`repair_concepts_case_insensitive_matching` which exercises the same case-insensitive regex
path against three variants (`[[Tool]]`, `[[TOOL]]`, `[[tool]]`) and verifies exact replacement
count. Test 3 uses two variants and no count assertion — it is a strict subset.

**Assessment of overlap:**
- `repair_concepts_batch.rs` canonical: three case variants + `count() == 3` assertion
- This test: two case variants + no count assertion

Test 3 adds no scenario that the batch test does not already cover. Merging means deleting
this test and verifying `repair_concepts_batch.rs` already provides coverage (it does).

**Minor issue with current assertions (if kept):** The input has two distinct variants;
the output should have exactly two `[[tool]]` instances. `content.contains("[[tool]]")`
passes even if only one replacement succeeded. Add:
```rust
assert_eq!(
    content.matches("[[tool]]").count(), 2,
    "both case variants should produce canonical wikilinks: {content}"
);
```

---

## Recommendations

**R-1 (Critical):** Resolve the design ambiguity in `fr24_4_ac1`. Audit whether `repair_concepts`
is *intended* to update the `modified` frontmatter timestamp. If yes, implement it and add
`assert_ne!(modified_after, modified_before)`. If no, rename, remove the sleep, and assert
equality. The current test provides zero regression signal for either interpretation.

**R-2 (Medium):** In `fr24_4_ac2`, add a `tags` assertion (sequence type + content) and add a
content-level assertion that the replacement actually ran. Without both, the test cannot
distinguish a successful repair from a no-op.

**R-3 (Medium):** Delete `fr24_4_ac3` and confirm `repair_concepts_batch.rs`'s case-sensitivity
test provides equivalent or stronger coverage (it does). Content-replacement tests do not belong
in `frontmatter_repair.rs`.

**R-4 (Low):** Once R-3 is applied, consider whether `fr24_4_ac2` also deserves to migrate to
`repair_concepts_replacement.rs`, since frontmatter preservation during repair is a repair
behavior, not a frontmatter behavior. `frontmatter_repair.rs` would then contain only Test 1,
which may not justify its own file.

**R-5 (Informational):** Register `T-META-001.6` in `TODO.md` or `RTM.md`, or document in
`CHANGELOG.md` that it was completed. It is currently a dangling traceability reference present
in five test files with no canonical definition.

---

## Load-Bearing Score: 1/3

| Test | Load-Bearing? | Reasoning |
|---|---|---|
| `fr24_4_ac1_concept_repair_modified_updated` | ❌ No | Asserts only non-emptiness of timestamps that are always non-empty. `assert_ne!` is absent. No regression can be caught. |
| `fr24_4_ac2_concept_repair_frontmatter_preserved` | ⚠️ Partial | Catches wholesale frontmatter destruction but misses `tags` (sequence type) and cannot distinguish a successful repair from a no-op. |
| `fr24_4_ac3_concept_repair_replacements_correct` | ✅ Yes | Would catch a regression in the case-insensitive regex path. Weaker than the batch test analog but functional. Misplaced file. |

One of three tests (33%) reliably catches a regression. One is conditionally useful with gaps.
One proves nothing its name claims.
