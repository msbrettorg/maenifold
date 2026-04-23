use std::fs;
#[cfg(unix)]
use std::os::unix::fs as unix_fs;
use std::path::PathBuf;

use super::*;

#[test]
fn filters_match_id_title_and_issue_toggle() {
    let mut app = GateUiApp::new(PathBuf::from("."), sample_tasks(), None);
    app.search_text = "acp".to_string();
    assert_eq!(app.visible_tasks().len(), 1);

    app.search_text.clear();
    app.issues_only = true;
    let visible = app.visible_tasks();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].task_id, "T-2");
}

#[cfg(unix)]
#[test]
fn open_task_rejects_symlinked_task_path() {
    let temp = tempfile::tempdir().expect("create temp root");
    let root = temp.path();
    fs::create_dir_all(root.join("todo")).expect("create todo");
    fs::create_dir_all(root.join("done")).expect("create done");

    let target = root.join("todo").join("ui-target.md");
    fs::write(&target, "# UI target\n").expect("write ui target");
    let task_path = root.join("todo").join("T-1.md");
    unix_fs::symlink(&target, &task_path).expect("create task symlink");

    let task = TaskEntry {
        task_id: "T-1".to_string(),
        title: "Symlinked UI task".to_string(),
        pipeline: Some("sdd".to_string()),
        state: "pending".to_string(),
        stages: vec!["pending".to_string()],
        location: "todo".to_string(),
        attempts: 0,
        path: task_path,
        issue: String::new(),
    };
    let mut app = GateUiApp::new(root.to_path_buf(), vec![task.clone()], None);

    app.open_task(&task);

    assert!(app.selected.is_none());
    assert!(!app.detail_open);
}

#[test]
fn progress_sort_orders_todo_before_done() {
    let mut tasks = sample_tasks();
    sort_tasks(
        &mut tasks,
        SortState {
            column: Some(SortColumn::Progress),
            direction: SortDirection::Asc,
        },
    );
    let ids = tasks
        .iter()
        .map(|task| task.task_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["T-1", "T-2", "T-3"]);
}

#[test]
fn default_task_order_uses_natural_task_id_sort() {
    let mut tasks = vec![
        TaskEntry {
            task_id: "T-90.10".to_string(),
            ..sample_tasks()[0].clone()
        },
        TaskEntry {
            task_id: "T-90.2".to_string(),
            ..sample_tasks()[0].clone()
        },
        TaskEntry {
            task_id: "T-90.9".to_string(),
            ..sample_tasks()[0].clone()
        },
    ];
    sort_tasks(&mut tasks, SortState::default());
    let ids = tasks
        .iter()
        .map(|task| task.task_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["T-90.2", "T-90.9", "T-90.10"]);
}

#[test]
fn id_column_sort_uses_natural_task_id_order_in_both_directions() {
    let mut ascending = vec![
        TaskEntry {
            task_id: "misc-10".to_string(),
            ..sample_tasks()[0].clone()
        },
        TaskEntry {
            task_id: "misc-2".to_string(),
            ..sample_tasks()[0].clone()
        },
        TaskEntry {
            task_id: "misc".to_string(),
            ..sample_tasks()[0].clone()
        },
    ];
    sort_tasks(
        &mut ascending,
        SortState {
            column: Some(SortColumn::Id),
            direction: SortDirection::Asc,
        },
    );
    let ascending_ids = ascending
        .iter()
        .map(|task| task.task_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ascending_ids, vec!["misc", "misc-2", "misc-10"]);

    sort_tasks(
        &mut ascending,
        SortState {
            column: Some(SortColumn::Id),
            direction: SortDirection::Desc,
        },
    );
    let descending_ids = ascending
        .iter()
        .map(|task| task.task_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(descending_ids, vec!["misc-10", "misc-2", "misc"]);
}

#[test]
fn stage_progress_marks_completed_active_and_pending_steps() {
    let task = TaskEntry {
        task_id: "T-4".to_string(),
        title: "Progress".to_string(),
        pipeline: Some("sdd".to_string()),
        state: "red-team".to_string(),
        stages: vec![
            "swe".to_string(),
            "red-team".to_string(),
            "blue-team".to_string(),
        ],
        location: "todo".to_string(),
        attempts: 0,
        path: PathBuf::from("./todo/T-4.md"),
        issue: String::new(),
    };

    assert_eq!(stage_progress("swe", &task), StageProgress::Completed);
    assert_eq!(stage_progress("red-team", &task), StageProgress::Active);
    assert_eq!(stage_progress("blue-team", &task), StageProgress::Pending);
}

#[test]
fn progress_color_uses_stoplight_mapping() {
    let done = TaskEntry {
        location: "done".to_string(),
        state: "done".to_string(),
        ..sample_tasks()[0].clone()
    };
    let pending = TaskEntry {
        state: "pending".to_string(),
        ..sample_tasks()[0].clone()
    };
    let active = TaskEntry {
        state: "research".to_string(),
        ..sample_tasks()[1].clone()
    };
    let failed = TaskEntry {
        state: "error".to_string(),
        ..sample_tasks()[1].clone()
    };

    assert_eq!(progress_color(&done), completed_progress_color());
    assert_eq!(progress_color(&pending), pending_progress_color());
    assert_eq!(progress_color(&active), active_progress_color());
    assert_eq!(progress_color(&failed), failed_progress_color());
}

#[test]
fn stage_dot_uses_stoplight_colors() {
    let task = TaskEntry {
        task_id: "T-5".to_string(),
        title: "Progress".to_string(),
        pipeline: Some("sdd".to_string()),
        state: "red-team".to_string(),
        stages: vec![
            "swe".to_string(),
            "red-team".to_string(),
            "blue-team".to_string(),
        ],
        location: "todo".to_string(),
        attempts: 0,
        path: PathBuf::from("./todo/T-5.md"),
        issue: String::new(),
    };

    assert_eq!(stage_dot("swe", &task), ("●", completed_progress_color()));
    assert_eq!(stage_dot("red-team", &task), ("◉", active_progress_color()));
    assert_eq!(
        stage_dot("blue-team", &task),
        ("○", pending_progress_color())
    );
}

fn sample_tasks() -> Vec<TaskEntry> {
    vec![
        TaskEntry {
            task_id: "T-1".to_string(),
            title: "ACP authentication".to_string(),
            pipeline: Some("sdd".to_string()),
            state: "pending".to_string(),
            stages: vec![
                "swe".to_string(),
                "red-team".to_string(),
                "blue-team".to_string(),
            ],
            location: "todo".to_string(),
            attempts: 0,
            path: PathBuf::from("./todo/T-1.md"),
            issue: String::new(),
        },
        TaskEntry {
            task_id: "T-2".to_string(),
            title: "Gate sync issue".to_string(),
            pipeline: Some("gsd".to_string()),
            state: "research".to_string(),
            stages: vec![
                "discover".to_string(),
                "research".to_string(),
                "deliver".to_string(),
            ],
            location: "todo".to_string(),
            attempts: 1,
            path: PathBuf::from("./todo/T-2.md"),
            issue: "Missing pipeline".to_string(),
        },
        TaskEntry {
            task_id: "T-3".to_string(),
            title: "Finished task".to_string(),
            pipeline: Some("sdd".to_string()),
            state: "done".to_string(),
            stages: vec![
                "swe".to_string(),
                "red-team".to_string(),
                "blue-team".to_string(),
            ],
            location: "done".to_string(),
            attempts: 0,
            path: PathBuf::from("./done/T-3.md"),
            issue: String::new(),
        },
    ]
}
