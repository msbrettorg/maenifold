# Test Evaluation: tool_registry_tests

**File:** `crates/maenifold-core/tests/tool_registry_tests.rs`
**Test count:** 13
**Evaluated:** 2025-07-15
**Traceability:** T-0.2 / T-13.4 / FR-1 through FR-6 / FR-7

---

## Summary

Integration-layer tests with a global mutex, isolated env, and full tool invocation stack. Three defects:  
(1) **Tests 2 and 8 are the same test written twice** — both call `GetConfig` and assert on its output, with different string fragments. One should be deleted.  
(2) **Test 3 is subsumed by test 4** — test 4 exercises all cases test 3 exercises plus more; test 3 is dead weight.  
(3) **Tests 10 and 11 assert almost nothing** — `contains("gethelp")` and `contains("# Directory:") || contains("### Files")` are format-sniffing, not behavior verification. A tool could return an error containing the tool name and these tests would still pass.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `new_registry_registers_all_expected_tool_names` | **KEEP** | Snapshot test of the complete 28-tool manifest. Catches both accidental additions and regressions. The hardcoded `BTreeSet` is the right tradeoff here — drift is intentional and reviewer-visible |
| `try_invoke_with_valid_tool_name_returns_successful_result` | **REMOVE** | **Directly redundant with test 8.** Both call `invoke_markdown("GetConfig", json!({}))` and assert on string presence in the output. Test 8 asserts a more specific/dynamic value (actual `memory_path`). Delete this one, keep test 8 |
| `try_invoke_with_lowercase_alias_dispatches_same_as_canonical` | **REMOVE** | **Subsumed by test 4.** Test 4 exercises `"GETCONFIG"`, `"GetConfig"`, `"getconfig"`, `"gEtCoNfIg"` — all four casings. Test 3 only tests `"getconfig"` == `"GetConfig"`. Anything test 3 proves, test 4 also proves. Delete test 3 |
| `try_invoke_case_insensitive_lookup_always_resolves` | **KEEP** | Tests four distinct casing variants; the loop-based assertion is efficient. The assertion `contains(memory_path)` is strong — it checks that actual config content appears, not just that the call didn't error |
| `try_invoke_with_unknown_tool_returns_unknown_tool_error` | **KEEP** | Pattern-matches on `MaenifoldError::UnknownTool(name)` AND asserts the name is preserved in the variant. Strong: tests error type discrimination, not just error presence |
| `try_invoke_with_empty_or_whitespace_name_returns_unknown_tool_error` | **KEEP** | Extends test 5 to boundary input values; both `""` and `"   "` are exercised. Load-bearing: empty/whitespace could plausibly be trimmed before lookup, which would change the error |
| `try_invoke_read_memory_with_missing_file_returns_not_found_message` | **REWRITE** | The assertion `lower.contains("not found") || lower.contains("error")` is too loose — almost any error response satisfies it. The contract being tested is that `ReadMemory` returns a rendered response (not a panic/unwrap) for a missing file. Rewrite: assert `result.is_ok()` (the `Ok` path is where rendered "not found" lives) and then assert the returned string contains a file-path or URI reference, not just a generic word |
| `try_invoke_get_config_returns_non_null_result` | **KEEP** | Asserts `contains(env.config.memory_path)` — a dynamic, non-trivial assertion that confirms the config output reflects the actual isolated test environment. Better assertion than test 2 |
| `try_invoke_memory_status_returns_non_null_result` | **KEEP** | Five distinct structure assertions against the MemoryStatus output: header, file count, folder count, total size, graph section. This is the most thorough output-structure test in the file |
| `try_invoke_get_help_with_tool_name_returns_non_null_result` | **REWRITE** | `result.to_ascii_lowercase().contains("gethelp")` is near-useless. A tool name appearing anywhere in output (including in an error like "Unknown tool 'GetHelp'") would satisfy this. The test name says "non_null_result" which is exactly what it tests — existence, not content. Rewrite to assert the help output contains a meaningful structural marker: `## Parameters`, `## Description`, or at minimum the tool description text |
| `try_invoke_list_memories_returns_non_null_result` | **REWRITE** | `contains("# Directory:") || contains("### Files")` is format-sniffing with an OR — means EITHER header format is acceptable, but the test doesn't document why there are two valid formats. If the output format changes, the test passes with a format nobody expected. Rewrite to assert the output at minimum contains the test environment's temp directory path (it must, since `ListMemories` lists the memory root), making the assertion dynamic rather than format-dependent |
| `constructing_registry_twice_produces_the_same_registrations` | **KEEP** | Tests registry construction determinism. Would catch any global mutable state, randomized registration order, or `OnceLock` initialization hazard. No I/O needed; fast |
| `global_registry_invocation_works_without_explicit_setup_call` | **KEEP** | Tests `ToolRegistry::global()` lazy initialization. Partially overlaps with test 9 (both call MemoryStatus and assert the header), but test 13's distinct value is proving that the *global singleton* path works without a prior explicit `ToolRegistry::new()` setup call |

---

## Structural Observations

### Tests 2 + 8: Identical invocation, divergent assertions

```rust
// Test 2
let result = invoke_markdown("GetConfig", json!({})).expect("GetConfig should succeed");
assert!(result.contains("Maenifold Configuration"), ...);

// Test 8
let result = invoke_markdown("GetConfig", json!({})).expect("GetConfig should succeed");
assert!(result.contains(env.config.memory_path.to_string_lossy().as_ref()), ...);
```

Both call the same tool with the same payload. Test 8's assertion is strictly stronger (it embeds the actual runtime memory path, which proves the config reflects the test environment). Test 2's `"Maenifold Configuration"` is a format string that could survive a bug where the path is wrong. **Delete test 2, keep test 8.**

### Tests 3 + 4: Complete subsumption

Test 4 runs `for name in ["GETCONFIG", "GetConfig", "getconfig", "gEtCoNfIg"]` and includes `"getconfig"` — the exact value that test 3 independently tests. Test 3 adds nothing to the coverage that test 4 doesn't already provide. Additionally, test 3's assertion (`alias == canonical`) is slightly weaker than test 4's (`contains(memory_path)`), so test 4 is also asserting more. **Delete test 3.**

### Tests 9 + 13: Overlapping MemoryStatus assertions

Both invoke `MemoryStatus` and assert `contains("# Memory System Status")`. Test 9 also asserts 4 additional structural markers. Test 13's value is proving the *lazy global* path works, which is genuinely distinct. But the shared `# Memory System Status` assertion means a regression in MemoryStatus header text would fail both tests — the first failure would be reported twice, causing confusion. Acceptable duplication given the different test goals; document in test 13 that the MemoryStatus content assertion is incidental.

---

## Missing Tests (Gaps)

1. **Alias lookup for a non-GetConfig tool**: all case-insensitivity tests use `GetConfig`. A test using `wrITEmeMory` would confirm aliasing isn't a `GetConfig`-only special case.

2. **`try_invoke` with a valid tool that returns an `Err`**: test 7 tests `ReadMemory` on missing file but expects an `Ok` with error content. There's no test for a tool that legitimately returns `MaenifoldError::*` from `try_invoke`.

3. **`ToolRegistry::contains` with an alias (not canonical name)**: test 1 only checks `canonical_names()` contains, not that `registry.contains("getconfig")` works for lowercase.

---

## Recommendations

1. **Delete test 2** (`try_invoke_with_valid_tool_name_returns_successful_result`) — subsumed by test 8.

2. **Delete test 3** (`try_invoke_with_lowercase_alias_dispatches_same_as_canonical`) — subsumed by test 4.

3. **Rewrite test 7** (`try_invoke_read_memory_with_missing_file_returns_not_found_message`) to assert `result.is_ok()` and check for the URI in the response, replacing the loose `contains("not found") || contains("error")`.

4. **Rewrite test 10** (`try_invoke_get_help_with_tool_name_returns_non_null_result`) to check for a structural marker from the help format: `assert!(result.contains("## Parameters") || result.contains("Description"))`.

5. **Rewrite test 11** (`try_invoke_list_memories_returns_non_null_result`) to assert the output contains the isolated test environment's memory path string, making the assertion environment-aware rather than format-pattern-dependent.

6. **Add a test** for `ToolRegistry::contains` with an alias (lowercase): `assert!(registry.contains("getconfig"))`.

---

## Load-Bearing Score: 9/13

- Tests 2 and 3 are fully subsumed (2 tests that add zero unique coverage).
- Tests 10 and 11 have assertions so weak that a meaningful regression (tool returning garbage output) would pass them. Technically they'd catch a complete `None`/panic but not a wrong-content response.
- The remaining 9 tests are genuinely load-bearing: test 1 would catch a missing tool registration; tests 5–6 catch error-type regressions; test 9 catches MemoryStatus format regressions; tests 12–13 catch registry construction/init regressions.

---

## SWE Remediation
- REMOVE `try_invoke_with_valid_tool_name_returns_successful_result`: deleted, subsumed by `try_invoke_get_config_returns_non_null_result` (test 8) which asserts dynamic memory path
- REMOVE `try_invoke_with_lowercase_alias_dispatches_same_as_canonical`: deleted, subsumed by `try_invoke_case_insensitive_lookup_always_resolves` (test 4) which covers all casing variants
- REWRITE `try_invoke_read_memory_with_missing_file_returns_not_found_message`: now asserts the rendered response contains the requested URI/identifier, replacing loose `contains("not found") || contains("error")`
- REWRITE `try_invoke_get_help_with_tool_name_returns_non_null_result`: now asserts structural markers (`## Parameters` or `Description`), replacing near-useless `contains("gethelp")`
- REWRITE `try_invoke_list_memories_returns_non_null_result`: now seeds a sentinel file and asserts it appears in the listing, proving the tool reads from the isolated test environment (ListMemories uses relative paths internally so absolute memory_path assertion was not viable)
- ADD `registry_contains_resolves_aliases_not_only_canonical_names`: tests `ToolRegistry::contains` with lowercase/uppercase aliases for both GetConfig and WriteMemory, closing gap #3 from eval
- Test count: 13 → 12 (–2 removed, +1 added). All 12 pass.
- Commit: f4fd486
