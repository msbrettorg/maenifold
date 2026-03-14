# Test Evaluation: safe_json_tests

**File**: `crates/maenifold-core/tests/safe_json_tests.rs`  
**Source Under Test**: `crates/maenifold-core/src/util/json.rs` — `from_str_safe<T>()`, `ensure_max_depth()`, `SAFE_JSON_MAX_DEPTH`  
**Evaluator**: 🔘 Gray — Skeptical Inquiry  
**Traceability**: NFR-2 (test coverage per tool), NFR-3 (no regressions), NFR-1 (C# oracle parity)  
**Related RTM**: `RTM NFR-3 / SEC-001` (see §Traceability Defects below)

---

## Summary

The file contains **4 tests** targeting a depth-limiting JSON parser guard. The production function (`from_str_safe`) is a thin two-pass wrapper: first `ensure_max_depth` (hand-rolled char scanner) validates nesting depth ≤ 32, then `serde_json::from_str` parses. The guard exists to prevent deeply nested JSON payloads from causing stack overflows or DoS in the `serde_json` recursive deserializer.

**Skeptical baseline questions answered before verdicts were issued:**

- *What does depth 10 actually prove that depth 32 does not?* Nothing structurally. It confirms the happy path but tests no unique logic branch.
- *What does depth 64 prove that depth 33 does not?* Nothing. `ensure_max_depth` is a linear char scan with no recursion and no branching after `depth > max_depth`. If 33 fails, 64 is trivially caught by the same `if depth > max_depth` arm.
- *Are both pass cases needed?* Only one is load-bearing (the exact boundary). The other is comfort.
- *Does the helper `create_nested_json` exercise the full scanner?* No. It only emits `{` / `}` characters. The scanner also tracks `[` / `]` and has escape-aware in-string exclusion logic — none of which is exercised by any test in this file.
- *Is `SEC-001` a real RTM entry?* No. It appears in 4 test comments but resolves to nothing in `RTM.md`, `TODO.md`, or `CHANGELOG.md`. Dangling.

**Headline verdict**: 2 of 4 tests are genuinely load-bearing. 1 is marginal. 1 is redundant. The suite also has meaningful **coverage gaps** against the function's own complexity.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Assertions Correct? | Name Idiomatic? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|---------|
| 1 | `safe_json_with_normal_depth_deserializes` | ⚠️ Marginal | ❌ No | ✅ Yes | ✅ Adequate | ✅ Yes | **MERGE** |
| 2 | `safe_json_with_max_depth_32_deserializes` | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Optional | **KEEP** |
| 3 | `safe_json_with_depth_over_32_throws_exception` | ✅ Yes | ✅ Yes | ✅ Yes | ❌ Java idiom | ✅ Optional | **KEEP** (name: REWRITE) |
| 4 | `safe_json_with_depth_64_throws_exception` | ❌ No | ❌ No | ✅ Yes | ❌ Java idiom | ✅ Yes | **REMOVE** |

---

## Per-Test Analysis

### Test 1 — `safe_json_with_normal_depth_deserializes`

```rust
fn safe_json_with_normal_depth_deserializes()
```

**What it claims to test**: That a depth-10 payload parses successfully.  
**What it actually proves**: That the happy path doesn't crash for one arbitrary depth below the limit.

**Load-bearing?** No unique code path. Depth 10 traverses the exact same `if depth > max_depth` check as depth 32 — it just never gets close to triggering it. If this test failed, test 2 would also fail. The converse is not true: test 2 could fail while this one passes if someone introduced an off-by-one at the boundary (e.g., changed `>` to `>=`). So this test provides *zero* discriminating power at the boundary.

**Unique?** No. Replaced entirely by test 2 for the "accepts valid depth" role. The only distinction is that depth 10 is "comfortably within" whereas depth 32 is "at the limit." Comfortably-within coverage is only valuable if there is a risk the implementation might accidentally reject low depths — which there isn't, because the counter only fires at `depth > max_depth`.

**Name quality**: "normal_depth" is undefined language. What makes 10 "normal"? The name doesn't communicate what boundary is being checked.

**Verdict: MERGE** — Eliminate this test and replace with a parameterized accept-case that covers `[1, 10, 31, 32]` using `rstest::rstest` or a `for` loop. Retaining depth-1 and depth-31 alongside depth-32 is far more diagnostic than the arbitrary depth-10 choice.

---

### Test 2 — `safe_json_with_max_depth_32_deserializes`

```rust
fn safe_json_with_max_depth_32_deserializes()
```

**What it claims to test**: That a payload at exactly `SAFE_JSON_MAX_DEPTH` parses.  
**What it actually proves**: The boundary is *inclusive* — `depth > max_depth` not `depth >= max_depth`.

**Load-bearing?** Yes. This is the critical off-by-one guard. If a maintainer changes `>` to `>=` in `ensure_max_depth`, this test is the only one that catches it. The assertion `is_ok()` confirms the limit is inclusive.

**Unique?** Yes. Test 1 cannot replace it because test 1's depth (10) never reaches the comparison point.

**Assertions correct?** Yes. `assert!(parsed.is_ok(), ...)` is the right check. The diagnostic message includes `{parsed:?}` which surfaces the actual error on failure — well done.

**Name quality**: Adequate. `safe_json_with_max_depth_32_deserializes` correctly names the constant value and the outcome.

**Verdict: KEEP**

---

### Test 3 — `safe_json_with_depth_over_32_throws_exception`

```rust
fn safe_json_with_depth_over_32_throws_exception()
```

**What it claims to test**: That depth `SAFE_JSON_MAX_DEPTH + 1` (33) is rejected.  
**What it actually proves**: The rejection arm fires at exactly one over the limit.

**Load-bearing?** Yes. The security invariant. If `ensure_max_depth` is removed or its check neutralised, this test fails. It is the primary security regression guard for this function.

**Unique?** Yes. Test 4 cannot be merged into this because (see test 4 analysis) test 4 is the redundant one, not this one.

**Assertions correct?** Yes, with a qualification. The test uses:
```rust
assert!(
    error.to_string().to_ascii_lowercase().contains("depth"),
    ...
);
```
This checks the *message content* of the error, not an error type. This is the correct approach given `serde_json::Error::io(...)` wraps a plain `std::io::Error` with no typed variant to match. The `to_ascii_lowercase()` defensive normalisation is thoughtful. However, the assertion would also pass if any other depth-related error occurred for an unrelated reason — this is an acceptable tradeoff given the `serde_json` API constraints.

**Name quality**: `throws_exception` is Java/C# idiom. Rust convention is `returns_err` or `fails_with_depth_error`. This is a cosmetic issue but creates friction for Rust-native readers of the test suite.

**Verdict: KEEP** — Optionally REWRITE name to `depth_33_exceeds_limit_returns_err` or similar Rust-idiomatic form.

---

### Test 4 — `safe_json_with_depth_64_throws_exception`

```rust
fn safe_json_with_depth_64_throws_exception()
```

**What it claims to test**: That extreme nesting (depth 64) is rejected.  
**What it actually proves**: Nothing test 3 does not already prove.

**Load-bearing?** No. The `ensure_max_depth` scanner is a simple O(N) linear char walk with no recursion and no branching after the `depth > max_depth` check:

```rust
'{' | '[' => {
    depth += 1;
    if depth > max_depth {
        return Err(...);  // ← same arm fires for 33 and 64
    }
}
```

Once depth 33 is confirmed to fail, depth 64 is mechanically guaranteed to fail — it hits the same `return Err(...)` at the 33rd nesting level. There is no code path that distinguishes the two.

**Possible rationale** (steel-manned): Testing extreme depth validates that the function doesn't *hang*, recurse deeply, or allocate unboundedly before rejecting. However, `ensure_max_depth` is a simple `for ch in input.chars()` loop — there is no recursion, no heap growth, and early return on first violation. The "doesn't hang at depth 64" scenario is not a real risk.

**Name quality**: Same Java idiom issue as test 3.

**Verdict: REMOVE** — Delete this test. If depth-parameterised failure coverage is desired for documentation purposes, merge it into a `#[case(33)]` / `#[case(64)]` / `#[case(1000)]` parameterised block alongside test 3.

---

## Coverage Gaps — What the Suite Does Not Test

The following scenarios are present in the production implementation but absent from the test suite. These represent real risks, not theoretical ones.

### Gap 1 — Array Nesting (`[`) is Never Exercised

`create_nested_json` produces only `{}` objects. The production scanner increments `depth` on both `{` and `[`. No test verifies:
- `[[[[...]]]]` at depth 32 is accepted
- `[[[[...]]]]` at depth 33 is rejected
- Mixed `[` and `{` nesting (e.g., `[{"a": [{"b": ...}]}]`) at boundary

If a maintainer inadvertently removed the `'[' =>` arm from the scanner, all 4 existing tests would continue passing.

**Recommended addition**: Mirror tests 2 and 3 using an array-nesting helper.

### Gap 2 — In-String `{` and `[` Are Never Exercised

The scanner tracks `in_string` state and escape sequences to avoid counting brackets inside JSON strings. This logic is not tested at all:

```rust
fn ensure_max_depth(input: &str, max_depth: usize) -> serde_json::Result<()> {
    let mut in_string = false;
    let mut escaped = false;
    // ...
    '"' => in_string = true,
    '{' | '[' => { depth += 1; ... }   // only reached when !in_string
```

A broken `in_string` toggle would cause `{"key": "value {with} [brackets]"}` to report depth 3 instead of depth 1. No test would catch this regression.

**Recommended addition**:
```rust
#[test]
fn brackets_inside_strings_are_not_counted_as_depth() {
    let json = r#"{"key": "value {with} [brackets] \" escaped"}"#;
    let parsed = from_str_safe::<Value>(json);
    assert!(parsed.is_ok(), "depth-1 with string brackets should succeed");
}
```

### Gap 3 — Escape Sequence Handling Is Untested

The scanner handles `\\` and `\"` inside strings. Edge cases not tested:
- `"\\"` (string containing a single backslash — escape followed by `"` closes the string correctly)
- `"\\{"` (backslash-escaped backslash, then `{` after close — should be counted)
- A string ending with `\` (malformed, but scanner should not get confused)

### Gap 4 — `get_string_array` Has No Tests in This File

`get_string_array` in `util/json.rs` also calls `from_str_safe` (when the value is a `Value::String` that should deserialize as a `Vec<String>`). This path:
1. Uses `from_str_safe` on attacker-controlled string values
2. Emits a degraded diagnostic on failure via unsafe `write(fd=2, ...)`
3. Returns `None` silently on depth violation (the depth error is swallowed)

This silent-swallow behavior means a deeply-nested string payload passed as a `get_string_array` argument would be silently discarded rather than surfacing an error. No test verifies this behavior is intentional.

### Gap 5 — Traceability Defect

All 4 tests carry the comment `// T-0.1: RTM NFR-3 / SEC-001`. The reference `SEC-001` does not appear in `RTM.md`, `TODO.md`, or `CHANGELOG.md`. It is a dangling identifier. There is no way to trace what security requirement this tag refers to, whether it has been accepted/closed, or whether the tests actually satisfy it. `T-0.1` is used as a generic "ported from C# oracle" tag throughout the codebase, not as a specific task — which dilutes its traceability value for this file specifically.

---

## Recommendations

Ordered by impact:

1. **Add array-nesting boundary tests** (Gap 1 — highest value). Mirror tests 2 and 3 for `[` nesting. A two-liner helper `create_nested_array(depth)` would suffice.

2. **Add in-string bracket exclusion test** (Gap 2 — correctness guard for the escape-tracking logic). Catches a full class of scanner regressions invisible to the current suite.

3. **Remove test 4** (`safe_json_with_depth_64_throws_exception`). Pure redundancy. Adds noise without coverage value.

4. **Merge test 1 into a parameterized accept-case** alongside depth 1, 10, 31, 32 using `rstest`. This replaces the arbitrary depth-10 choice with a principled set including the edge (1) and the pre-boundary (31).

5. **Rename "throws_exception" tests** to Rust-idiomatic names (`returns_err`, `is_rejected`, `fails_with`). Minor but consistent with codebase conventions.

6. **Resolve or remove `SEC-001` reference**. Either add `SEC-001` to `RTM.md` with an explicit requirement statement, or replace the comment with the actual NFR entry it maps to (likely `NFR-3`).

7. **Add `get_string_array` depth test** (Gap 4 — documents silent-swallow behavior as intentional). At minimum, a comment in the test file noting the omission is intentional would reduce ambiguity.

---

## Load-Bearing Score: 2/4

| Test | Load-Bearing |
|------|:---:|
| `safe_json_with_normal_depth_deserializes` | ⚠️ Marginal (no unique branch exercised) |
| `safe_json_with_max_depth_32_deserializes` | ✅ Yes — off-by-one boundary guard |
| `safe_json_with_depth_over_32_throws_exception` | ✅ Yes — primary security rejection test |
| `safe_json_with_depth_64_throws_exception` | ❌ No — redundant code path |

**2 of 4 tests are genuinely load-bearing.** The suite guards the critical boundary correctly but leaves the `[`-nesting path and in-string exclusion logic completely unverified. A targeted scanner regression (breaking only array depth counting or string escape handling) would pass all 4 existing tests.

---

## SWE Remediation

- MERGE `safe_json_with_normal_depth_deserializes` → `safe_json_sub_boundary_depths_deserialize`: parameterized over depths [1, 10, 31] replacing arbitrary depth-10
- KEEP `safe_json_with_max_depth_32_deserializes`: unchanged (off-by-one boundary guard)
- KEEP+REWRITE `safe_json_with_depth_over_32_throws_exception` → `safe_json_depth_33_exceeds_limit_returns_err`: Rust-idiomatic name
- REMOVE `safe_json_with_depth_64_throws_exception`: redundant with depth-33 test
- ADD `safe_json_array_nesting_at_max_depth_deserializes`: Gap 1 — array `[` nesting at boundary 32 (Rec #1)
- ADD `safe_json_array_nesting_at_33_returns_err`: Gap 1 — array nesting rejection at 33 (Rec #1)
- ADD `safe_json_brackets_inside_strings_are_not_counted`: Gap 2 — in-string bracket exclusion guard (Rec #2)
- ADD `safe_json_mixed_nesting_at_max_depth_deserializes`: mixed `{`/`[` nesting at boundary
- FIX dangling `SEC-001` references in `json.rs` and test comments → `NFR-3` only (Rec #6)
- Final test count: 7 (was 4). Load-bearing score: 7/7.
- Commit: 2286708

---

## Red-team Audit

**Traceability**: NFR-2, NFR-3 — eval references are valid. `T-0.1` traceability tags in source consistent across all 7 tests.

### Verdict Line Items

- ✅ MERGE `safe_json_with_normal_depth_deserializes` → `safe_json_sub_boundary_depths_deserialize`: depths `[1, 10, 31]` match eval recommendation. Depth 32 correctly excluded (already covered by KEEP test 2, no redundancy). Parameterized via `for` loop as spec allowed.
- ✅ KEEP `safe_json_with_max_depth_32_deserializes`: code unchanged, confirmed in diff `ee459bd..e1b58c8`. Off-by-one boundary guard preserved.
- ✅ REWRITE `safe_json_with_depth_over_32_throws_exception` → `safe_json_depth_33_exceeds_limit_returns_err`: name now Rust-idiomatic (`returns_err`). Assertion body unchanged — same `to_ascii_lowercase().contains("depth")` check. Matches eval Rec #5.
- ✅ REMOVE `safe_json_with_depth_64_throws_exception`: deleted. No unique coverage lost — depth-33 test covers the same `if depth > max_depth` arm. Confirmed eval analysis correct.
- ✅ ADD `safe_json_array_nesting_at_max_depth_deserializes`: new `create_nested_array` helper produces `[[[...]]]` payloads. Tests `[` nesting at boundary 32 (accept). Addresses eval Gap 1.
- ✅ ADD `safe_json_array_nesting_at_33_returns_err`: array nesting at 33 rejected. Mirrors object-nesting rejection test. Addresses eval Gap 1.
- ✅ ADD `safe_json_brackets_inside_strings_are_not_counted`: uses exact JSON payload from eval Gap 2 recommendation. Tests `in_string` + escape tracking logic. Guards against scanner regression where `{`/`[` inside strings inflate depth counter.
- ✅ ADD `safe_json_mixed_nesting_at_max_depth_deserializes`: alternating `{`/`[` nesting at boundary 32. Addresses mixed-nesting case from eval Gap 1.
- ✅ FIX `SEC-001`: zero occurrences of `SEC-001` remain in `crates/` source tree. All test comments now reference `NFR-3` only. Matches eval Rec #6.

### Production Code Audit

- Only production change: 2 comment lines in `crates/maenifold-core/src/util/json.rs` removing `/ SEC-001` from traceability tags. No logic, no assertion, no signature changes. No unrelated modifications introduced.

### Test Execution

- All 7 tests pass: `cargo test --package maenifold-core safe_json` → `7 passed; 0 failed`.
- Final count 7 matches SWE claim (was 4).

### Minor Finding

- ⚠️ Commit reference in SWE Remediation section says `2286708` but that commit is `test(repair_concepts_replacement)`. The actual safe_json_tests remediation commit is `e1b58c8`. This is a metadata error in the eval spec, not a code defect.

### Not Addressed (Accepted)

- Eval Gap 3 (escape sequence edge cases: `"\\"`, `"\\{"`) — not in SWE remediation plan. Lower priority. Acceptable.
- Eval Gap 4 (`get_string_array` silent-swallow behavior test) — not in SWE remediation plan. Lower priority. Acceptable.
- Eval Rec #4 (`rstest` parameterization) — SWE used `for` loop instead. Eval explicitly allowed this alternative. Acceptable.

### Verdict: **PASS**

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- safe_json_tests`: 7 passed, 0 failed
- Verdict: PASS
