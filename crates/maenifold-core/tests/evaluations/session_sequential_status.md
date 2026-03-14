# Test Evaluation: `session_sequential_status.rs`

**Traceability**: T-12.7 | NFR-1 (C# Oracle Parity)  
**Reviewer perspective**: 🔘 Gray — Skeptical Inquiry  
**Sister files examined**: `session_sequential_lifecycle.rs`, `session_sequential_flow.rs`, `session_sequential_errors.rs`, `date_detection_ts_tests.rs`

---

## Summary

This file contains **4 tests** targeting the `SequentialThinking` tool's status-transition surface: the closed-parent guard, two cancel-path variants, and session completion. The file name (`session_sequential_status.rs`) correctly telegraphs a status-focused scope, which is the right organisational intent.

**What it does well**: Tests 2 and 3 together give solid cancellation coverage — one with a full payload, one with a minimal bare cancel — and together they constitute the only cancellation tests in this cluster. Test 1 is the only test anywhere that exercises the closed-parent-workflow guard, making it uniquely load-bearing.

**What is questioned**: Test 4 (`session_with_conclusion_completes_session`) is the weakest member. Nearly every assertion it makes is already made, in more detail, by `complete_session_lifecycle_create_continue_complete_succeeds` in `session_sequential_errors.rs`. The one assertion it adds that is genuinely novel — WikiLink preservation inside the conclusion body — is buried and not named to signal its value.

Test 1 leaves two of three "closed" states (`cancelled`, `abandoned`) completely untested despite the production code in `validate_session_state` checking all three explicitly. This is a coverage gap the name conceals.

Test 3's assertion on `thoughtCount` is an existence check only; it does not verify the actual value. This is a latent correctness risk if the field is ever populated with a wrong value.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Assertions Correct? | Name Clear? | Parameterisable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|---------|
| 1 | `new_session_with_closed_parent_workflow_returns_error` | ✅ Yes | ✅ Yes | ⚠️ Partial — only `completed` tested; `cancelled`/`abandoned` not covered; no error-code assertion | ⚠️ Partial — implies total coverage of closed states | ✅ Strongly (3 closed statuses) | **REWRITE** |
| 2 | `session_with_cancel_updates_status_to_cancelled` | ✅ Yes | ✅ Yes | ⚠️ Partial — no assertion that cancelled thought section is absent from content | ⚠️ Partial — "with_cancel" is vague | ❌ No | **KEEP** |
| 3 | `session_with_cancel_without_response_updates_status_to_cancelled` | ✅ Yes | ✅ Yes | ⚠️ Weak — `thoughtCount` existence check only; value `"1"` never asserted | ✅ Yes | ❌ No | **REWRITE** |
| 4 | `session_with_conclusion_completes_session` | ⚠️ Partial | ⚠️ Marginal | ✅ All correct, but nearly all duplicated | ⚠️ Misleading — hides the only novel assertion (WikiLink in conclusion) | ❌ No | **MERGE** |

---

## Per-Test Analysis

### Test 1 — `new_session_with_closed_parent_workflow_returns_error`

**What it does**: Creates a workflow session, manually forces its `status` frontmatter to `"completed"`, then asserts that a new SequentialThinking call with that parent returns an error containing `"is completed"`.

**Code path exercised**: `validate_session_state` → branch `["completed", "cancelled", "abandoned"].iter().any(...)`.

**Gap — partial state coverage**: The production guard checks three terminal states. This test covers exactly one. The question is: *how do we know the guard also fires for `cancelled` and `abandoned`?* We don't. The code shares one branch, so a regression that broke, say, the `abandoned` string comparison would pass this test silently. The test name gives no signal that coverage is partial.

**Gap — no error-code assertion**: The `render_failure` call emits the code `"PARENT_WORKFLOW_CLOSED"` in the output. The test only checks the human-readable message fragment `"is completed"`, not the machine-readable code. If the error code is ever consumed programmatically (e.g., by the ACP orchestrator in T-23.7), a typo in the code goes undetected.

**Recommended rewrite**:
```rust
#[test]
#[case("completed")]
#[case("cancelled")]
#[case("abandoned")]
fn new_session_with_closed_parent_workflow_returns_error(status: &str) {
    // set parent frontmatter status = status
    // assert result.starts_with("ERROR:")
    // assert result.contains("PARENT_WORKFLOW_CLOSED")
    // assert result.contains(status)
}
```
Using `rstest` (already a dev-dep candidate) or manual loop over statuses.

---

### Test 2 — `session_with_cancel_updates_status_to_cancelled`

**What it does**: Creates a session, then cancels it *with* a full payload (`response`, `nextThoughtNeeded`, `thoughtNumber: 1`, `totalThoughts: 5`). Asserts message content, frontmatter `status`, `cancelled` timestamp key, and `thoughtCount`.

**Assertion correctness confirmed**: `"Added thought 1 to session:"` is correct — `build_completion_message` always forms the base message from `thought_number`, then appends `"❌ Thinking cancelled"`. The emoji is not asserted (minor), but `contains("Thinking cancelled")` matches the substring.

**`thoughtCount: "2"` is correct**: `finalize_session` computes `thought_number + 1 = 1 + 1 = 2`. ✅

**Missing assertion — content**: The handler skips `append_h2_section` when `cancel == true`. The test does not verify that no thought section was written. This means a regression that removes the `if !input.cancel` guard would still pass. Add:
```rust
let (_, content) = harness.read(&uri);
assert!(!content.contains("Thought 1/5"), "cancelled thought must not be appended to content");
```

**Name suggestion**: `cancel_with_full_payload_sets_status_and_thought_count` — makes it easier to distinguish from test 3 at a glance.

---

### Test 3 — `session_with_cancel_without_response_updates_status_to_cancelled`

**What it does**: Creates a session, then sends the minimal cancel form: only `sessionId` and `cancel: true`. This exercises the path where `thought_number` defaults to `0` and `response` is `None`.

**`thoughtCount` assertion is weak**: The test does:
```rust
assert!(frontmatter.contains_key("thoughtCount"));
```
`finalize_session` sets `thoughtCount = thought_number + 1 = 0 + 1 = 1`. The test never checks this value. If a bug set `thoughtCount` to `0` or `99`, this test would pass. Replace with:
```rust
assert_eq!(frontmatter_string(&frontmatter, "thoughtCount"), "1");
```

**Missing assertion — message format**: For `thought_number == 0`, `build_completion_message` generates `"Created session: {id}"` as the base, not `"Added thought 0 to session:"`. The test does not verify the message format for the zero-thought cancel case. This is a meaningful path in the formatter.

**Recommended rewrite additions**:
```rust
assert!(cancel_result.contains("Created session:"));
assert_eq!(frontmatter_string(&frontmatter, "thoughtCount"), "1");
```

---

### Test 4 — `session_with_conclusion_completes_session`

**What it does**: Creates a 2-thought session and completes it with a `conclusion` containing `[[synthesis]]`. Asserts status, `completed` key, `thoughtCount`, `Conclusion` heading in content, and WikiLink preservation.

**Duplication map** against `complete_session_lifecycle_create_continue_complete_succeeds` in `session_sequential_errors.rs`:

| Assertion | This test | `errors.rs` lifecycle | `lifecycle.rs` | 
|-----------|:---------:|:---------------------:|:--------------:|
| `result.contains("Thinking complete")` | ✅ | ✅ | ✅ |
| `status == "completed"` | ✅ | ✅ | ✅ |
| `frontmatter.contains_key("completed")` | ✅ | ✅ | ❌ |
| `thoughtCount == "2"` (or "3") | ✅ | ✅ | ✅ |
| `content.contains("Conclusion")` | ✅ | ✅ | ❌ |
| WikiLink preserved in conclusion body | `[[synthesis]]` ✅ | `[[modular-design]]` ✅ | ❌ |

The only marginally unique aspect is the *explicit WikiLink name* asserted — and both `[[synthesis]]` and `[[modular-design]]` test the same code path, so there is no new coverage.

**What should survive a MERGE**: The concept of asserting that WikiLinks are preserved in the conclusion body is worth keeping — but as a targeted test in `session_sequential_flow.rs` focused on conclusion formatting, with a name like `session_conclusion_preserves_wikilinks_in_content`, rather than a generic lifecycle rehash.

---

## Recommendations

1. **Parameterise Test 1** across all three closed statuses (`completed`, `cancelled`, `abandoned`) and add an error-code assertion. The current test gives false confidence that all terminal states are guarded.

2. **Strengthen Test 3's `thoughtCount` assertion** from existence check to value check (`"1"`). Add a message-format assertion for the zero-thought cancel case.

3. **Add a content-negative assertion to Test 2**: verify that a cancelled thought is *not* appended to the session file. This guards the `if !input.cancel` branch in `handler.rs`.

4. **MERGE Test 4 into `session_sequential_errors.rs`** (`complete_session_lifecycle_create_continue_complete_succeeds`), extracting the WikiLink-in-conclusion assertion as an additional assert there, or as its own focused test named `session_conclusion_preserves_wikilinks_in_content` in `session_sequential_flow.rs`.

5. **RTM coverage gap**: The closed-parent guard (`RTM` via `NFR-1` / T-12.7) is only tested for the `completed` state. If the product ever adds a fourth terminal status, there is no structural signal (test) to verify the guard handles it. Consider tracking this under a NFR-1 parity task.

---

## Load-Bearing Score: 3/4

- ✅ **Test 1** (`closed_parent_error`) — uniquely load-bearing; no coverage exists elsewhere, but partially covers its own stated domain
- ✅ **Test 2** (`cancel_with_response`) — uniquely load-bearing; only cancellation-with-payload test in the cluster
- ✅ **Test 3** (`cancel_without_response`) — uniquely load-bearing; only bare-cancel test; assertion quality is weak but the path is covered
- ⚠️ **Test 4** (`conclusion_completes_session`) — substantially duplicated by `session_sequential_errors.rs`; the one unique assertion (WikiLink in conclusion) is not named to signal its value
