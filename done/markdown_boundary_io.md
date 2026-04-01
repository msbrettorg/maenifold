---
id: T-80.91
title: "Test Evaluation: markdown_boundary_io"
status: done
pipeline: tdd
---

# Test Evaluation: markdown_boundary_io

**File:** `crates/maenifold-core/tests/markdown_boundary_io.rs`
**Test count:** 10
**Evaluated:** 2025-07-15
**Traceability:** T-12.7, T-W1-2, NFR-1, NFR-2, FR-28.1–FR-28.4

## Summary

The best-written of the three files: all 10 tests exercise the production API against real filesystem I/O in isolated environments. Coverage spans four distinct operations (read, write, update, append). Two tests assert only filesystem *existence* without checking *content*, which is a meaningful but incomplete assertion. The checksum test is the weakest fixture in the file — it verifies a checksum is "non-empty" but does not prove it is content-derived or deterministic.

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `fr28_1_ac1_read_markdown_valid_uri_returns_content` | **REWRITE** | Load-bearing, but the checksum assertion is `assert!(!checksum.is_empty())`. This only proves the function returns *a* string, not that the checksum is content-derived. A function that returns `"abc"` unconditionally would pass. Strengthen: write two files with different content, read both, assert checksums differ. Also: `assert!(read_content.contains("World"))` — the seeded content is `"# Hello\n\nWorld"`. "World" is inside the content body so this passes, but `contains` is a weak assertion. Assert the full body or at minimum a more unique sentinel. |
| `fr28_1_ac2_read_markdown_nested_uri_returns_content` | **KEEP** | Load-bearing and unique: verifies that `read_markdown` correctly resolves multi-segment URI paths (`projects/alpha/docs/readme`) to a physical nested directory structure. This is a distinct code path from flat URIs (path construction logic). The `contains("Deeply nested content.")` assertion is sufficient here because the only thing under test is path resolution, not content fidelity. |
| `fr28_1_ac3_read_markdown_non_existent_uri_returns_not_found_error` | **KEEP** | Load-bearing: verifies the error variant (`MaenifoldError::Message`) and message fragment (`"Memory file not found"`). Correct assertion target. Would catch a regression that returns an empty result instead of an error, or panics. |
| `fr28_2_ac1_write_markdown_valid_uri_creates_file` | **REWRITE** | Asserts only `env.cfg.memory_path.join("simple-note.md").exists()`. This passes even if `write_markdown` creates an *empty* file or a file with garbage content. Strengthen: read back the file with `read_markdown` and assert the content matches. A 3-line addition eliminates the entire class of "writes a file but corrupts content" regressions. |
| `fr28_2_ac2_write_markdown_nested_uri_creates_intermediate_dirs` | **REWRITE** | Same issue as ac1: asserts file existence only. The meaningful behavior here is (a) that intermediate directories are created and (b) that the file is readable. The current test confirms (a) implicitly via path existence, but add a `read_markdown` call to confirm (b). Without it, a bug where `write_markdown` creates the directories but fails to write content (returning `Ok(())` anyway) would pass this test. |
| `fr28_2_ac3_write_markdown_frontmatter_round_trips` | **KEEP** | The strongest test in this file. Asserts exact frontmatter values after a round-trip: title string, tag sequence values, content body. Catches YAML serialization bugs, key casing changes, and type coercion errors. Load-bearing and unique. |
| `fr28_3_ac1_update_markdown_valid_checksum_succeeds` | **KEEP** | Load-bearing: verifies the happy-path update flow — read checksum, use it in update, verify new content is written. Correctly chains `read_markdown` → `update_markdown` → `read_markdown`. The assertion `contains("New content")` is adequate because the entire updated body is under test. |
| `fr28_3_ac2_update_markdown_stale_checksum_returns_error` | **KEEP** | Load-bearing and critical. Tests the OCC (Optimistic Concurrency Control) guard. If this test is removed, a regression that accepts any checksum (or ignores it) would ship silently. The string `"bogus-checksum-that-does-not-match"` is sufficiently distinct from any real checksum. Assertion checks `MaenifoldError::Message("Checksum mismatch")` — correct target. |
| `fr28_4_ac1_append_h2_section_valid_uri_appends_section` | **KEEP** | Load-bearing: verifies three things — (1) existing content is preserved, (2) the appended section uses correct H2 heading syntax (`## New Section`), (3) the appended body content is present. All three assertions are meaningful. If the append function truncates the file instead, (1) catches it. If it uses H3 instead of H2, (2) catches it. |
| `fr28_4_ac2_append_h2_section_non_existent_uri_creates_file` | **KEEP** | Load-bearing: tests create-on-append semantics, which is a documented behavior. Combines `file_exists` + `read_markdown` to verify both existence and content. This is the correct assertion pattern — wish ac1 and ac2 followed this example. |

## Recommendations

1. **Fix the two existence-only write tests (ac1, ac2).** Add a `read_markdown` + content assertion after each write. This is the highest-value fix in this file: it turns two shallow smoke tests into real regression guards at the cost of 3 lines each.

2. **Strengthen the checksum test in ac1.** Replace `assert!(!checksum.is_empty())` with a proof that the checksum is content-derived:
   ```rust
   // Write a second file with different content, read it, compare checksums
   seed_file(&env.cfg, "memory://notes/other-note", None, "# Different\n\nOther");
   let (_, _, checksum2) = markdown::read_markdown("memory://notes/other-note", &env.cfg).unwrap();
   assert_ne!(checksum, checksum2, "checksums must differ for different content");
   ```
   Without this, the checksum feature has no load-bearing test.

3. **Add a test for `update_markdown` with no checksum (None).** The current tests cover (a) correct checksum succeeds and (b) wrong checksum fails. What about `checksum: None`? Is that allowed (unconditional overwrite) or rejected? This behavior is undocumented by the test suite. The answer matters for callers who don't do optimistic locking.

4. **Consider a rename for ac1.** `fr28_1_ac1_read_markdown_valid_uri_returns_content` is accurate but the name suggests "returns any content" rather than "round-trips frontmatter and body." The test actually covers more than its name implies. Better: `fr28_1_ac1_read_markdown_returns_frontmatter_content_and_checksum`.

5. **The `_guard = TEST_MUTEX.lock()` pattern.** All 10 tests acquire a global mutex, suggesting they share global state despite using `isolated_env()` + `TempDir`. This is suspicious: if each test uses a truly isolated temp directory and calls `Config::reset_overrides()` on drop, why is a global mutex needed? If it's needed because `Config::override_root` mutates global state, then the mutex is mandatory — but this should be explicitly documented. If it's defensive boilerplate copied from a time when tests shared state, it may be hiding a design issue. Either add a comment explaining why or remove it.

## Load-Bearing Score: 8/10

Tests ac1, ac2, ac3 (read), ac3, ac2 (write/frontmatter), ac1, ac2 (update), ac1, ac2 (append) — 8 are genuinely load-bearing for distinct behaviors. The write-existence-only tests (fr28_2_ac1 and fr28_2_ac2) are load-bearing at the weakest possible assertion level.

---
## SWE Remediation

- REWRITE fr28_1_ac1: renamed to `fr28_1_ac1_read_markdown_returns_frontmatter_content_and_checksum` (Rec 4); strengthened body assertion from `contains("World")` to `contains("# Hello\n\nWorld")`; replaced `assert!(!checksum.is_empty())` with checksum-differs proof — writes second file with different content and asserts checksums are not equal (Rec 2, P0)
- REWRITE fr28_2_ac1: added `read_markdown` round-trip after `write_markdown` to assert body content, not just file existence (Rec 1, P0)
- REWRITE fr28_2_ac2: added `read_markdown` round-trip after nested write to verify content is readable through directory hierarchy (Rec 1, P0)
- ADD fr28_3_ac3_update_markdown_none_checksum_overwrites_unconditionally: new test documenting that `update_markdown` with `None` checksum skips OCC and overwrites unconditionally (Rec 3, P1)
- DOC: added comment explaining TEST_MUTEX is mandatory because `Config::override_root` mutates process-global state (Rec 5, P2)
- KEEP: 7 tests unchanged (fr28_1_ac2, fr28_1_ac3, fr28_2_ac3, fr28_3_ac1, fr28_3_ac2, fr28_4_ac1, fr28_4_ac2)
- Test count: 10 → 11, all passing
- Commit: e76844e

---
## Red-team Audit

**Traceability:** T-12.7, T-W1-2 | FR-28.1–FR-28.4 | NFR-1, NFR-2

### Line-item verification

- ✅ REWRITE fr28_1_ac1: Renamed per Rec 4. Body assertion strengthened from `contains("World")` → `contains("# Hello\n\nWorld")` per eval spec. Checksum proof matches Rec 2 exactly: second file seeded with different content, `assert_ne!(checksum, checksum2)`. P0 checksum gap closed.
- ✅ REWRITE fr28_2_ac1: `read_markdown` round-trip added after `write_markdown` with content assertion per Rec 1. Existence assertion preserved (harmless). P0 write-existence gap closed.
- ✅ REWRITE fr28_2_ac2: `read_markdown` round-trip added after nested write per Rec 1. Verifies content readable through directory hierarchy. P0 write-existence gap closed.
- ✅ ADD fr28_3_ac3: New test documents that `update_markdown` with `None` checksum skips OCC, matching Rec 3 (P1). Seeds file, updates with `None`, reads back, asserts new content present.
- ✅ DOC: TEST_MUTEX comment added at lines 9-11 explaining `Config::override_root` global mutation, matching Rec 5 (P2).
- ✅ KEEP × 7: fr28_1_ac2, fr28_1_ac3, fr28_2_ac3, fr28_3_ac1, fr28_3_ac2, fr28_4_ac1, fr28_4_ac2 — all unmodified in diff, matching eval KEEP verdicts.
- ✅ No unrelated changes introduced. Diff is scoped exactly to eval recommendations.
- ✅ 11/11 tests passing (`cargo test -p maenifold-core --test integration_tests -- markdown_boundary_io`).

### Findings

- ⚠️ **Wrong commit hash in SWE remediation section.** Claims `e76844e`, which is the `markdown_adversarial_structure` remediation commit. The actual code change for this file is in `549f292`. The eval spec append (remediation notes only) is `33f45a1`. Non-blocking but traceability should be corrected.

### Verdict: **PASS**

All P0, P1, and P2 recommendations addressed. Code changes match eval spec precisely. Test count correct (10 → 11). No regressions, no unrelated changes.

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- markdown_boundary_io`: 11 passed, 0 failed
- Note: consolidated binary had a pre-existing compilation error in unrelated `markdown_boundary_redteam.rs` (uncommitted WIP adding `ToolRegistry::global().try_invoke` usage). Stashed the WIP, ran tests, restored. The error is not in this eval's scope.
- Verdict: PASS
