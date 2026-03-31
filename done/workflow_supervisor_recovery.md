---
id: workflow_supervisor_recovery
status: done
---

# Test Evaluation: workflow_supervisor_recovery
**File:** `crates/maenifold-core/tests/workflow_supervisor_recovery.rs`
**Test count:** 4
**Evaluated:** 2026-07-14
**Traceability:** T-W1-7 · T-12.7 · NFR-1 · FR-10.1 · FR-10.3

---

## Summary

Four tests covering the *recovery* surface of the workflow supervisor: the content annotation written when a completed submachine unblocks a workflow, the unblocking of a workflow whose submachine entered the `abandoned` status, the recovery path when the submachine's markdown file has been physically deleted, and the full re-registration cycle executed twice in sequence. No test is a pure duplicate of another within this file, and each exercises a code path untouched by `workflow_supervisor_lifecycle.rs` or `workflow_supervisor_registration.rs` — with one important exception detailed below.

**Critical overlap — test 1 vs `lifecycle.rs`.**  
`workflow_supervisor_lifecycle.rs::continue_after_submachine_completed_clears_supervisor_fields_and_resumes` executes the identical setup (start workflow → register submachine → mark "completed" → invoke resume) and asserts the state machine's output: `!waiting on submachine`, `!ERROR`, `Step 2/`, `phase == "running"`, `activeSubmachineType == ""`, `activeSubmachineSessionId == ""`. Test 1 in this file (`unblock_after_completion_appends_resume_note_to_content`) adds one genuinely unique dimension — checking that the body annotation "Submachine Resumed" and the prior submachine status are written to the session content — but is **entirely missing the state assertions** the lifecycle test already provides. As written, test 1 catches a failure of the annotation write but says nothing about whether the workflow actually unblocked or the frontmatter was correctly cleared. It is load-bearing only for the content path; the state path is held by the lifecycle test.

Three other concerns:

1. **`content.contains("completed")` is an indirect assertion** (test 1). The word "completed" must be written by the resume annotation logic into the workflow body — the initial response texts contain no such word — so the check is not trivially vacuous. But it is indirect: a reworded annotation that writes `status: done` or `Finished` instead would silently pass. A targeted assertion (`content.contains("Status: completed")` or the exact annotation heading) would be more resilient.

2. **`!resume_result.contains("ERROR")` is case-sensitive** (tests 2 and 3). An error surface that emits lowercase `"error"` or uses a different sentinel would pass this assertion. This is a minor but real blind spot present across the entire supervisor test suite.

3. **`activeSubmachineType` is not cleared after cycle A in test 4.** The re-registration test asserts `activeSubmachineSessionId == ""` after submachine A completes, but does not assert `activeSubmachineType == ""` until after submachine B completes. A bug that clears the session ID but not the type after the first cycle would not be caught until the second cycle's terminal assertion — and by then the signal is masked by the B-cycle's own completion.

No test is spurious. The file earns a **3/4 load-bearing score**: three tests (2, 3, 4) are comprehensive and independently necessary; test 1 is load-bearing only for the content annotation path and requires a rewrite to be defensively complete.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `unblock_after_completion_appends_resume_note_to_content` | **REWRITE** | Genuinely unique in checking the content annotation ("Submachine Resumed") written on a completed-submachine resume — no other test in the suite verifies this body write. However, the two assertions (`content.contains("Submachine Resumed")` and `content.contains("completed")`) are the only ones made. The test says nothing about whether the workflow actually unblocked: no result step check, no `phase == "running"`, no `activeSubmachineSessionId == ""`. A bug that writes the annotation but leaves the workflow stuck in "waiting" would pass silently. The `lifecycle.rs` counterpart already holds the state assertions; this test should add them. Also, `content.contains("completed")` is imprecise — it passes for any occurrence of "completed" in the body. Rewrite to: (a) assert `resume_result.contains("Step 2/")`, (b) assert `phase == "running"` and both supervisor fields cleared, and (c) tighten the content assertion to a more specific string like `"Status: completed"` or the exact annotation header. |
| `abandoned_submachine_unblocks_workflow` | **KEEP** | The only test in the suite that exercises the `abandoned` status path. The lifecycle file covers `cancelled`; registration covers the double-registration block while `active`. `abandoned` is a distinct status code path requiring its own test. Assertions are comprehensive: two negative result guards, the step-advancement positive check, and all three frontmatter fields (`phase`, `activeSubmachineType`, `activeSubmachineSessionId`). No structural weaknesses beyond the case-sensitive `"ERROR"` guard noted in the summary. |
| `missing_submachine_file_unblocks_workflow` | **KEEP** | The only test in the entire suite that exercises the physical-file-deletion recovery path: the system must unblock a "waiting" workflow whose submachine file no longer exists on disk. The intermediate state assertion (`phase == "waiting"` before deletion) is a correct guard. The `assert!(submachine_path.exists())` pre-condition check prevents a silent failure if `path_for_uri` returns a wrong path. The content assertions — `content.contains("Submachine Cleared")`, `content.contains(&thinking_session_id)`, and `content.contains("deleted or expired")` — are the strongest in the file: they verify the session ID was recorded in the annotation and that the reason text is present. No other test would catch a regression in this recovery branch. |
| `re_registration_cycle_works_across_multiple_submachines` | **KEEP** (harden) | The only test covering the multi-cycle scenario: register, complete, advance, then register again. Correctly uses `step_before_a` and `step_after_resume_a` captures to assert the step-freeze invariant across both cycles without hard-coding step numbers. Two gaps: (1) `activeSubmachineType` is not asserted after cycle A resume — only after cycle B; a bug that fails to clear the type field on the first cycle would not be caught until the second cycle's terminal check, obscuring the failure point. (2) `step_after_resume_a > step_before_a` is never explicitly asserted; step advancement is inferred solely from the `"Step 2/"` string. Adding `assert!(step_after_resume_a > step_before_a)` and the missing A-cycle `activeSubmachineType == ""` check would close both gaps. |

---

## Recommendations

### P1 — Rewrite test 1 to add state assertions and tighten content check

`unblock_after_completion_appends_resume_note_to_content` currently asserts only that the body contains two strings. It should mirror the assertion pattern of tests 2 and 3, which check both the result and the frontmatter. Suggested replacement assertions (in addition to the body checks):

```rust
// Result-level check: workflow actually advanced
assert!(!resume_result.contains("waiting on submachine"));
assert!(!resume_result.contains("ERROR"));
assert!(resume_result.contains("Step 2/"));

// State-machine check: supervisor fields cleared
let frontmatter = read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
assert_eq!(yaml_string(&frontmatter, "phase"), "running");
assert_eq!(yaml_string(&frontmatter, "activeSubmachineType"), "");
assert_eq!(yaml_string(&frontmatter, "activeSubmachineSessionId"), "");

// Content check: annotation written (tighten the string)
let content = read_body(&env.cfg, &workflow_uri(&workflow_session_id));
assert!(content.contains("Submachine Resumed"));
assert!(content.contains("Status: completed"));   // more specific than bare "completed"
```

With these additions the test becomes independently verifiable — it covers both the content annotation path (not covered by lifecycle) and the state machine path (already covered by lifecycle but now doubly asserted here, which is cheap insurance).

### P2 — Assert `activeSubmachineType` cleared after cycle A in test 4

In `re_registration_cycle_works_across_multiple_submachines`, the assertions after submachine A's resume only check `activeSubmachineSessionId`. Add:

```rust
assert_eq!(yaml_string(&frontmatter_after_resume_a, "activeSubmachineType"), "");
```

A bug that clears the session ID field but not the type field after the first cycle is currently invisible until the second cycle's assertions fire. Isolating the check to cycle A surfaces the failure point immediately.

### P3 — Add explicit numeric step-advancement assertion in test 4

`step_after_resume_a` is captured but only used as a baseline for the B-cycle step-freeze check. The step actually advanced (from `step_before_a` to `step_after_resume_a`) is confirmed only via the `"Step 2/"` string in the result. Add:

```rust
assert!(step_after_resume_a > step_before_a, "step should advance after A-cycle resume");
```

This documents the intent explicitly and would catch an off-by-zero regression where the step counter misbehaves in a way that still produces the "Step 2/" text.

### P4 — Consider tightening the `"ERROR"` guard (tests 2 and 3)

`!resume_result.contains("ERROR")` is uppercase-only. The production error surface may evolve to emit mixed-case variants. A case-insensitive check:

```rust
assert!(!resume_result.to_lowercase().contains("error"));
```

is more resilient. This is a low-priority cosmetic concern — the uppercase guard has not misfired in practice — but it is consistent with the skeptical stance applied to all negative assertions in this file.

### P5 — Document the intentional relationship with `lifecycle.rs` test 4

`continue_after_submachine_completed_clears_supervisor_fields_and_resumes` in `workflow_supervisor_lifecycle.rs` and `unblock_after_completion_appends_resume_note_to_content` in this file both exercise the "completed submachine triggers resume" path. The split is intentional (lifecycle tests state transitions; recovery tests content annotations) but is not documented anywhere. A one-line comment in the recovery test — `// Complements lifecycle::continue_after_submachine_completed; focuses on the body annotation write.` — would prevent a future evaluator from marking one as a duplicate.

---

## Load-Bearing Score: 3/4

Tests 2, 3, and 4 are independently necessary — removing any one would allow a distinct class of regression to ship undetected. Test 1 is load-bearing only for the content annotation path (`"Submachine Resumed"` written to body) and requires a rewrite to be defensively complete against state-machine regressions on the completed-submachine path.

---
## SWE Remediation
- REWRITE `unblock_after_completion_appends_resume_note_to_content`: added resume result assertions, frontmatter clearing checks, and a precise completion annotation assertion; documented its relationship to the lifecycle coverage.
- HARDEN `re_registration_cycle_works_across_multiple_submachines`: asserted `activeSubmachineType` clears after cycle A and that the step counter advances after the first resume.
- Commit: 5fe347d
