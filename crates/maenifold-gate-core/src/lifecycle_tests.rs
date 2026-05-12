use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use super::{
    GateContext, GateLifecycle, StageNoteArgs, StatusArgs, SyncArgs, TaskArgs, resolve_definition,
};
use crate::status_filter::StatusFilter;

#[test]
fn append_records_canonical_active_stage_without_transition() {
    let root = new_root();
    write_task(root.path(), "T-237.1", "sdd");
    let path = task_path(root.path(), "T-237.1", "todo");

    GateLifecycle::append(note(root.path(), "T-237.1", "swe", "captured progress"))
        .expect("append succeeds");

    let text = fs::read_to_string(path).expect("read task");
    assert!(text.contains("## Pending"));
    assert!(text.contains("- Stage ID: pending"));
    assert!(text.contains("- Record Kind: append"));
    assert!(text.contains("- Agent: swe"));
    assert!(text.contains("- Notes: captured progress"));
    assert!(!text.contains("- Outcome:"));
    let row = single_status(root.path());
    assert_eq!(row["state"], "pending");
}

#[test]
fn pass_and_fail_apply_state_machine_transitions() {
    let root = new_root();
    write_task(root.path(), "T-237.2", "sdd");

    let after_pass =
        GateLifecycle::pass(note(root.path(), "T-237.2", "pm", "ready")).expect("pass succeeds");
    assert_eq!(after_pass["state"], "swe");

    let after_fail = GateLifecycle::fail(note(root.path(), "T-237.2", "swe", "needs spec work"))
        .expect("fail succeeds");
    assert_eq!(after_fail["state"], "pending");

    let text = fs::read_to_string(task_path(root.path(), "T-237.2", "todo")).expect("read task");
    assert!(text.contains("- Outcome: PASS"));
    assert!(text.contains("- Outcome: FAIL"));
    assert!(text.contains("- Stage ID: swe"));
}

#[test]
fn pass_with_matching_expected_stage_advances_to_next_stage() {
    let root = new_root();
    write_task(root.path(), "T-245.6", "sdd");

    let after_pass = GateLifecycle::pass(note_with_expected_stage(
        root.path(),
        "T-245.6",
        "pm",
        "pending",
        "ready",
    ))
    .expect("pass succeeds when expected stage matches");

    assert_eq!(after_pass["state"], "swe");
}

#[test]
fn pass_with_wrong_expected_stage_returns_error_and_preserves_state() {
    let root = new_root();
    write_task(root.path(), "T-245.6", "sdd");

    let error = GateLifecycle::pass(note_with_expected_stage(
        root.path(),
        "T-245.6",
        "pm",
        "swe",
        "stale snapshot",
    ))
    .expect_err("pass should reject stale expected stage");

    assert!(error.to_string().contains("expected stage"));
    assert_eq!(single_status(root.path())["state"], "pending");
}

#[test]
fn fail_with_wrong_expected_stage_returns_error_and_preserves_state() {
    let root = new_root();
    write_task(root.path(), "T-245.6", "sdd");

    let error = GateLifecycle::fail(note_with_expected_stage(
        root.path(),
        "T-245.6",
        "pm",
        "swe",
        "stale snapshot",
    ))
    .expect_err("fail should reject stale expected stage");

    assert!(error.to_string().contains("expected stage"));
    assert_eq!(single_status(root.path())["state"], "pending");
}

#[test]
fn terminal_pass_moves_task_to_done() {
    let root = new_root();
    write_task(root.path(), "T-237.3", "test");

    let after_pass =
        GateLifecycle::pass(note(root.path(), "T-237.3", "pm", "complete")).expect("pass");

    assert_eq!(after_pass["state"], "done");
    assert!(!task_path(root.path(), "T-237.3", "todo").exists());
    assert!(task_path(root.path(), "T-237.3", "done").exists());
}

#[test]
fn read_commands_do_not_mutate_task_files() {
    let root = new_root();
    write_task(root.path(), "T-237.4", "sdd");
    let path = task_path(root.path(), "T-237.4", "todo");
    let before = fs::read_to_string(&path).expect("read before");

    GateLifecycle::status(StatusArgs {
        context: context(root.path()),
        filter: default_filter(),
    })
    .expect("status");
    GateLifecycle::history(TaskArgs {
        context: context(root.path()),
        task_id: "T-237.4".to_string(),
    })
    .expect("history");
    GateLifecycle::task_body(TaskArgs {
        context: context(root.path()),
        task_id: "T-237.4".to_string(),
    })
    .expect("task body");

    let after = fs::read_to_string(&path).expect("read after");
    assert_eq!(after, before);
}

#[test]
fn invalid_transition_and_unknown_task_errors_are_actionable() {
    let root = new_root();
    write_task(root.path(), "T-237.5", "sdd");

    let invalid = GateLifecycle::fail(note(root.path(), "T-237.5", "pm", "bad fail"))
        .expect_err("pending fail is invalid");
    assert!(invalid.to_string().contains("illegal transition"));

    let unknown = GateLifecycle::append(note(root.path(), "T-237.missing", "swe", "notes"))
        .expect_err("unknown task");
    assert!(unknown.to_string().contains("not found"));
    assert!(unknown.to_string().contains("searched directories"));
}

#[test]
fn sync_reports_counts_without_requiring_standalone_gate() {
    let root = new_root();
    write_task(root.path(), "T-237.6", "sdd");

    let value = GateLifecycle::sync(SyncArgs {
        context: context(root.path()),
        rebuild: true,
    })
    .expect("sync");

    assert_eq!(value["taskCount"], 1);
    assert_eq!(value["synced"], true);
    let metadata = fs::metadata(root.path().join(".gate/gate.db")).expect("gate.db metadata");
    assert!(metadata.len() > 0);
}

#[test]
fn sync_rebuild_true_auto_creates_gate_dir_for_fresh_root() {
    let root = new_root();
    GateLifecycle::sync(SyncArgs {
        context: context(root.path()),
        rebuild: true,
    })
    .expect("sync");
    assert!(root.path().join(".gate/gate.db").exists());
}

#[test]
fn sync_rebuild_false_auto_creates_gate_dir_for_fresh_root() {
    let root = new_root();
    GateLifecycle::sync(SyncArgs {
        context: context(root.path()),
        rebuild: false,
    })
    .expect("sync");
    assert!(root.path().join(".gate/gate.db").exists());
}

#[test]
fn sync_preserves_existing_gate_root_state() {
    let root = new_root();
    fs::create_dir_all(root.path().join(".gate")).expect("gate dir");
    fs::write(root.path().join(".gitignore"), ".gate/\n").expect("gitignore");
    GateLifecycle::sync(SyncArgs {
        context: context(root.path()),
        rebuild: true,
    })
    .expect("sync");
    assert_eq!(
        fs::read_to_string(root.path().join(".gitignore")).expect("gitignore"),
        ".gate/\n"
    );
    assert!(root.path().join(".gate/gate.db").exists());
}

#[test]
fn sync_augments_gitignore_when_creating_fresh_gate_root() {
    let root = new_root();
    fs::write(root.path().join(".gitignore"), "target/\n").expect("gitignore");
    GateLifecycle::sync(SyncArgs {
        context: context(root.path()),
        rebuild: true,
    })
    .expect("sync");
    let gitignore = fs::read_to_string(root.path().join(".gitignore")).expect("gitignore");
    assert!(gitignore.contains("target/\n"));
    assert!(gitignore.contains(".gate/"));
}

#[test]
fn sync_accepts_three_segment_task_ids_in_status() {
    let root = new_root();
    fs::write(
        root.path().join("todo/T-243.1.5-sub-task.md"),
        "---\nid: T-243.1.5\ntitle: sub task\npipeline: sdd\npriority: P1\n---\n# sub task\n",
    )
    .expect("write task");
    GateLifecycle::sync(SyncArgs {
        context: context(root.path()),
        rebuild: true,
    })
    .expect("sync");
    assert_eq!(single_status(root.path())["taskId"], "T-243.1.5");
}

#[test]
fn sync_rejects_four_segment_task_ids() {
    let root = new_root();
    fs::write(
        root.path().join("todo/T-243.1.5.9-bad.md"),
        "---\nid: T-243.1.5.9\ntitle: bad task\npipeline: sdd\npriority: P1\n---\n# bad task\n",
    )
    .expect("write task");
    GateLifecycle::sync(SyncArgs {
        context: context(root.path()),
        rebuild: true,
    })
    .expect("sync");
    assert_ne!(single_status(root.path())["taskId"], "T-243.1.5.9");
}

#[test]
fn sync_keeps_two_segment_task_ids_working() {
    let root = new_root();
    fs::write(root.path().join("todo/T-999.1-regression-check.md"), "---\nid: T-999.1\ntitle: regression check\npipeline: sdd\npriority: P1\n---\n# regression check\n").expect("write task");
    GateLifecycle::sync(SyncArgs {
        context: context(root.path()),
        rebuild: true,
    })
    .expect("sync");
    assert_eq!(single_status(root.path())["taskId"], "T-999.1");
}

#[test]
fn status_surfaces_malformed_task_id_with_path_expected_and_actual() {
    let root = new_root();
    fs::write(
        task_path(root.path(), "T-243.2.0-delta-review", "todo"),
        "---\nid: T-243.2.0\ntitle: malformed task\npipeline: sdd\n---\n# malformed task\n",
    )
    .expect("write malformed task");

    let row = single_status(root.path());
    assert_eq!(row["state"], "error");
    let issue = row["issue"].as_str().expect("issue string");
    assert!(issue.contains("todo/T-243.2.0-delta-review.md"));
    assert!(issue.contains(
        "expected format T-{sprint}.{N}.md or T-{sprint}.{N}.{M}.md"
    ));
    assert!(issue.contains("actual id 'T-243.2.0'"));
}

#[test]
fn status_error_rows_leniently_preserve_title_and_pipeline_for_malformed_ids() {
    let root = new_root();
    fs::write(
        task_path(root.path(), "T-243.2.0-lenient-salvage", "todo"),
        "---\nid: T-243.2.0\ntitle: malformed salvage task\npipeline: sdd\n---\n# malformed salvage task\n",
    )
    .expect("write malformed task");

    let row = single_status(root.path());

    assert_eq!(row["state"], "error");
    assert_eq!(row["title"], "malformed salvage task");
    assert_eq!(row["pipeline"], "sdd");
    assert_eq!(row["canonicalTaskId"], "T-243.2.0");
    let issue = row["issue"].as_str().expect("issue string");
    assert!(issue.contains(
        "expected format T-{sprint}.{N}.md or T-{sprint}.{N}.{M}.md"
    ));
    assert!(issue.contains("actual id 'T-243.2.0'"));
}

#[test]
fn builtin_definitions_are_cycle_free_gate_core_assets() {
    let definition = resolve_definition(Path::new("/"), "slc").expect("slc definition");
    assert_eq!(definition.initial_state().expect("initial"), "pending");
    assert!(definition.find_state("done").expect("done").terminal);
}

#[test]
fn status_error_rows_preserve_task_title_and_pipeline_when_status_derivation_fails() {
    let root = new_root();
    write_task(root.path(), "T-237.7", "sdd");
    fs::rename(
        task_path(root.path(), "T-237.7", "todo"),
        task_path(root.path(), "T-237.7", "done"),
    )
    .expect("move non-terminal task into done");

    let rows = GateLifecycle::status(StatusArgs {
        context: context(root.path()),
        filter: default_filter(),
    })
    .expect("status")
    .as_array()
    .expect("array")
    .clone();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["state"], "error");
    assert_eq!(rows[0]["title"], "T-237.7 task");
    assert_eq!(rows[0]["pipeline"], "sdd");
    assert_ne!(rows[0]["title"], "[error] T-237.7");
    assert_ne!(rows[0]["pipeline"], "unknown");
    assert!(
        rows[0]["issue"]
            .as_str()
            .expect("issue string")
            .contains("task archived in done/ but effective state is pending")
    );
}

fn new_root() -> TempDir {
    let root = tempfile::tempdir().expect("temp root");
    fs::create_dir_all(root.path().join("todo")).expect("todo");
    fs::create_dir_all(root.path().join("done")).expect("done");
    root
}

fn write_task(root: &Path, id: &str, pipeline: &str) {
    fs::write(
        task_path(root, id, "todo"),
        format!(
            "---\nid: {id}\ntitle: {id} task\npipeline: {pipeline}\n---\n# {id} task\n\n## Acceptance Criteria\n\n1. test task\n"
        ),
    )
    .expect("write task");
}

fn task_path(root: &Path, id: &str, location: &str) -> PathBuf {
    root.join(location).join(format!("{id}.md"))
}

fn note(root: &Path, task_id: &str, agent: &str, notes: &str) -> StageNoteArgs {
    StageNoteArgs {
        context: context(root),
        task_id: task_id.to_string(),
        agent: agent.to_string(),
        notes: notes.to_string(),
        model: Some("test-model".to_string()),
        expected_stage: None,
    }
}

fn note_with_expected_stage(
    root: &Path,
    task_id: &str,
    agent: &str,
    expected_stage: &str,
    notes: &str,
) -> StageNoteArgs {
    StageNoteArgs {
        context: context(root),
        task_id: task_id.to_string(),
        agent: agent.to_string(),
        notes: notes.to_string(),
        model: Some("test-model".to_string()),
        expected_stage: Some(expected_stage.to_string()),
    }
}

fn single_status(root: &Path) -> serde_json::Value {
    let rows = GateLifecycle::status(StatusArgs {
        context: context(root),
        filter: default_filter(),
    })
    .expect("status");
    rows.as_array().expect("array")[0].clone()
}

fn context(root: &Path) -> GateContext {
    GateContext {
        root: root.to_path_buf(),
        requested_pipeline_name: None,
    }
}

fn default_filter() -> StatusFilter {
    StatusFilter {
        state: None,
        ready_only: false,
        blocked_only: false,
        include_done: true,
    }
}

// --- T-250.0.1: priority / sprint / lastUpdatedAt in status_row ---

#[test]
fn status_row_includes_priority_sprint_and_last_updated_at_when_present() {
    let root = new_root();
    fs::write(
        task_path(root.path(), "T-250.99", "todo"),
        "---\nid: T-250.99\ntitle: T-250.99 task\npipeline: sdd\npriority: P1\nsprint: 250\n---\n# T-250.99 task\n\n## Acceptance Criteria\n\n1. test task\n",
    )
    .expect("write task");

    // pending → swe
    GateLifecycle::pass(note(root.path(), "T-250.99", "pm", "first pass")).expect("pass 1");
    // swe → red-team; returned row has 2 records
    let row = GateLifecycle::pass(note(root.path(), "T-250.99", "swe", "second pass"))
        .expect("pass 2");

    assert_eq!(row["priority"], "P1", "priority should be P1");
    assert_eq!(row["sprint"], 250u64, "sprint should be 250");
    assert!(
        row["lastUpdatedAt"]
            .as_str()
            .map(|s| !s.is_empty())
            .unwrap_or(false),
        "lastUpdatedAt should be a non-empty RFC 3339 string"
    );
    assert_eq!(
        row.as_object().unwrap().len(),
        14,
        "status_row must emit exactly 14 fields"
    );
}

#[test]
fn status_row_emits_null_priority_sprint_and_last_updated_at_when_absent() {
    let root = new_root();
    write_task(root.path(), "T-250.100", "sdd");
    let row = single_status(root.path());

    assert!(row["priority"].is_null(), "priority should be null");
    assert!(row["sprint"].is_null(), "sprint should be null");
    assert!(
        row["lastUpdatedAt"].is_null(),
        "lastUpdatedAt should be null when there are no records"
    );
    assert_eq!(
        row.as_object().unwrap().len(),
        14,
        "status_row must emit exactly 14 fields"
    );
}
