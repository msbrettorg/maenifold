// Regression tests for T-90.1: legacy done tasks that were recorded before
// `pending` became the canonical SDD bootstrap stage.
//
// ROOT CAUSE (T-90.1): Legacy done tasks record `Stage ID: swe` for the first
// PASS outcome.  The state machine starts replay at `pending`, and
// `next_on_pass("pending")` produces `acting_stage: "pending"`.  The equality
// check `transition.acting_stage != record.stage` rejects the record because
// `"pending" != "swe"`.  The fix in `maybe_seed_legacy_entry_stage` detects
// this pattern and advances replay state to the entry stage before the
// transition fires.

mod support;

use std::fs;

use serde_json::Value;
use support::{done_task_path, new_pipeline_root, status_json, sync_gate};

/// Minimal legacy done task: outcomes only, no interleaved appends.
#[test]
fn sync_replays_legacy_done_task_that_omits_pending_pass_records() {
    let root = new_pipeline_root();
    let task_path = done_task_path(root.path(), "T-89.12");
    fs::write(&task_path, LEGACY_DONE_OUTCOMES_ONLY).expect("write legacy done task");

    sync_gate(root.path());

    let status = status_json(root.path());
    let task = find_task(&status, "T-89.12");
    assert_eq!(task["pipeline"], "sdd");
    assert_eq!(task["state"], "done");
    assert_eq!(task["issue"], "");
}

/// Real-world legacy task with append records interleaved between outcomes
/// (matches the actual done/T-89.12.md shape).
#[test]
fn sync_replays_legacy_done_task_with_interleaved_appends() {
    let root = new_pipeline_root();
    let task_path = done_task_path(root.path(), "T-89.12");
    fs::write(&task_path, LEGACY_DONE_WITH_APPENDS).expect("write legacy done task");

    sync_gate(root.path());

    let status = status_json(root.path());
    let task = find_task(&status, "T-89.12");
    assert_eq!(task["pipeline"], "sdd");
    assert_eq!(task["state"], "done");
    assert_eq!(task["issue"], "");
}

/// Legacy done task with `status: pending` in frontmatter (some tasks were
/// moved to done/ without updating the YAML status field).
#[test]
fn sync_replays_legacy_done_task_with_stale_frontmatter_status() {
    let root = new_pipeline_root();
    let task_path = done_task_path(root.path(), "T-87.1");
    fs::write(&task_path, LEGACY_DONE_STALE_STATUS).expect("write legacy done task");

    sync_gate(root.path());

    let status = status_json(root.path());
    let task = find_task(&status, "T-87.1");
    assert_eq!(task["pipeline"], "sdd");
    assert_eq!(task["state"], "done");
    assert_eq!(task["issue"], "");
}

fn find_task<'a>(status: &'a Value, task_id: &str) -> &'a Value {
    status
        .as_array()
        .expect("status array")
        .iter()
        .find(|task| task["taskId"] == task_id)
        .unwrap_or_else(|| panic!("missing task {task_id}"))
}

const LEGACY_DONE_OUTCOMES_ONLY: &str = r#"---
id: T-89.12
title: Legacy archived gate task
status: done
pipeline: sdd
---

# T-89.12 — Legacy archived gate task

## SWE

- Stage ID: swe
- Record Kind: outcome
- Agent: pm
- Recorded At: 2026-04-03T18:26:18.233034+00:00
- Outcome: PASS
- Notes: Legacy tasks started recording at swe before pending became the bootstrap stage.

## RED-Team

- Stage ID: red-team
- Record Kind: outcome
- Agent: pm
- Recorded At: 2026-04-03T18:37:19.903542+00:00
- Outcome: PASS
- Notes: Red-team approved.

## BLUE-Team

- Stage ID: blue-team
- Record Kind: outcome
- Agent: pm
- Recorded At: 2026-04-03T18:37:19.903595+00:00
- Outcome: PASS
- Notes: Blue-team verified and archived the task.
"#;

const LEGACY_DONE_WITH_APPENDS: &str = r#"---
id: T-89.12
title: Endophasia tray menu item + browser launch
status: done
pipeline: sdd
---

# T-89.12 — Tray Menu Item + Browser Launch

## Description

Add "Endophasia" to tray_menu.rs in both core and gate.

## Acceptance Criteria

- Menu item appears with --features app
- Activating the menu item opens /endophasia in the system browser

## SWE

- Stage ID: swe
- Record Kind: outcome
- Agent: pm
- Recorded At: 2026-04-03T18:26:18.233034+00:00
- Outcome: PASS
- Notes: Dispatching to SWE.

## RED-Team

- Stage ID: red-team
- Record Kind: append
- Agent: pm
- Recorded At: 2026-04-03T18:36:33.194637+00:00
- Notes: Red-team found minor issue in pre-existing helper. Accepting T-89.12.

## RED-Team

- Stage ID: red-team
- Record Kind: outcome
- Agent: pm
- Recorded At: 2026-04-03T18:37:19.903542+00:00
- Outcome: PASS
- Notes: Red-team PASS.

## BLUE-Team

- Stage ID: blue-team
- Record Kind: outcome
- Agent: pm
- Recorded At: 2026-04-03T18:37:19.903595+00:00
- Outcome: PASS
- Notes: Blue-team verified and archived the task.
"#;

const LEGACY_DONE_STALE_STATUS: &str = r#"---
id: T-87.1
title: Fix pipeline schema drift
status: pending
pipeline: sdd
---

# T-87.1 — Fix Pipeline Schema Drift

## Description

Pipeline schema files have drift from the prior sprint audit.

## SWE

- Stage ID: swe
- Record Kind: append
- Agent: pm
- Recorded At: 2026-04-03T06:29:23.564043+00:00
- Notes: Dispatching SWE agent for schema drift fix.

## SWE

- Stage ID: swe
- Record Kind: outcome
- Agent: swe
- Recorded At: 2026-04-03T06:38:55.271622+00:00
- Outcome: PASS
- Notes: Schema drift fixed. Build passes.

## RED-Team

- Stage ID: red-team
- Record Kind: outcome
- Agent: red-team
- Recorded At: 2026-04-03T06:40:24.576218+00:00
- Outcome: PASS
- Notes: Red-team approved.

## BLUE-Team

- Stage ID: blue-team
- Record Kind: outcome
- Agent: blue-team
- Recorded At: 2026-04-03T06:40:28.530122+00:00
- Outcome: PASS
- Notes: Blue-team verified.
"#;
