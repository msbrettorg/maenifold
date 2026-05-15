use super::*;
use std::fs;
use std::path::Path;

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
    assert!(issue.contains("expected format T-{sprint}.{N}.md or T-{sprint}.{N}.{M}.md"));
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
    assert!(issue.contains("expected format T-{sprint}.{N}.md or T-{sprint}.{N}.{M}.md"));
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
        summary: false,
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

#[test]
fn status_row_includes_priority_sprint_and_last_updated_at_when_present() {
    let root = new_root();
    fs::write(
        task_path(root.path(), "T-250.99", "todo"),
        "---\nid: T-250.99\ntitle: T-250.99 task\npipeline: sdd\npriority: P1\nsprint: 250\n---\n# T-250.99 task\n\n## Acceptance Criteria\n\n1. test task\n",
    )
    .expect("write task");

    GateLifecycle::pass(note(root.path(), "T-250.99", "pm", "first pass")).expect("pass 1");
    let row =
        GateLifecycle::pass(note(root.path(), "T-250.99", "swe", "second pass")).expect("pass 2");

    assert_eq!(row["priority"], "P1", "priority should be P1");
    assert_eq!(row["sprint"], 250u64, "sprint should be 250");
    assert!(
        row["lastUpdatedAt"]
            .as_str()
            .map(|s| !s.is_empty())
            .unwrap_or(false),
        "lastUpdatedAt should be a non-empty RFC 3339 string"
    );
    assert_eq!(row.as_object().unwrap().len(), 18);
}

#[test]
fn status_row_emits_null_priority_sprint_and_last_updated_at_when_absent() {
    let root = new_root();
    write_task(root.path(), "T-250.100", "sdd");
    let row = single_status(root.path());

    assert!(row["priority"].is_null(), "priority should be null");
    assert!(row["sprint"].is_null(), "sprint should be null");
    assert!(row["lastUpdatedAt"].is_null());
    assert_eq!(row.as_object().unwrap().len(), 18);
}

#[test]
fn status_returns_blocked_state_and_frontmatter_fields() {
    let root = new_root();
    write_blocked_task(root.path(), "T-309.7");

    let row = single_status(root.path());

    assert_eq!(row["state"], "blocked");
    assert_eq!(row["ready"], false);
    assert_eq!(row["blocked"], true);
    assert_eq!(row["blocked_reason"], "waiting on user authorization");
    assert_eq!(row["superseded_by"], serde_json::json!(["T-309.8"]));
}

#[test]
fn ready_status_excludes_blocked_tasks() {
    let root = new_root();
    write_task(root.path(), "T-309.6", "gsd");
    write_blocked_task(root.path(), "T-309.7");

    let rows = GateLifecycle::status(StatusArgs {
        context: context(root.path()),
        filter: StatusFilter {
            state: None,
            ready_only: true,
            blocked_only: false,
            include_done: false,
        },
        summary: false,
    })
    .expect("status");
    let rows = rows.as_array().expect("array");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["taskId"], "T-309.6");
    assert_eq!(rows[0]["state"], "pending");
}

fn write_blocked_task(root: &Path, id: &str) {
    fs::write(
        task_path(root, id, "todo"),
        format!(
            "---\nid: {id}\ntitle: {id} blocked\npipeline: gsd\nstate: execute\nblocked: true\nblocked_reason: waiting on user authorization\nsuperseded_by: [T-309.8]\n---\n# {id} blocked\n"
        ),
    )
    .expect("write blocked task");
}
