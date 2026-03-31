use anyhow::Result;
use maenifold_gate::{
    fs::{append_orchestrator_section, discover_tasks, move_to_done, read_task, resolve_task_path},
    types::{StageState, StageVerdict, TaskLocation, TaskState},
};
use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};
use tempfile::tempdir;

fn write_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

#[test]
fn discover_tasks_filters_recurses_and_matches_task_names() -> Result<()> {
    let dir = tempdir()?;
    let root = dir.path();
    for rel in [
        "todo/T-1.md",
        "todo/deep/T-99.3-foo.md",
        "done/T-2.md",
        "todo/README.md",
        "todo/notes.md",
        "todo/T-.md",
        "done/T-4.txt",
    ] {
        write_file(&root.join(rel), "# task")?;
    }
    let mut found = discover_tasks(root)?
        .into_iter()
        .map(|t| {
            (
                t.path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
                t.location,
            )
        })
        .collect::<Vec<_>>();
    found.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        found,
        vec![
            ("done/T-2.md".into(), TaskLocation::Done),
            ("todo/T-1.md".into(), TaskLocation::Todo),
            ("todo/deep/T-99.3-foo.md".into(), TaskLocation::Todo),
        ]
    );
    Ok(())
}

#[test]
fn discover_tasks_errors_when_neither_todo_nor_done_exists() {
    let dir = tempdir().unwrap();
    let err = discover_tasks(dir.path()).unwrap_err();
    assert!(err.to_string().contains("todo or"), "{err}");
}

#[test]
fn move_to_done_creates_done_dir_and_rejects_done_inputs() -> Result<()> {
    let dir = tempdir()?;
    let root = dir.path();
    write_file(&root.join("todo/T-7.md"), "# todo")?;

    let moved = move_to_done(root, "T-7.md")?;
    assert_eq!(moved, root.join("done/T-7.md"));
    assert!(root.join("done").is_dir());
    assert!(!root.join("todo/T-7.md").exists());

    let err = move_to_done(root, "T-7.md").unwrap_err();
    assert!(err.to_string().contains("already under"), "{err}");
    Ok(())
}

#[test]
fn resolve_task_path_checks_todo_done_and_missing() -> Result<()> {
    let dir = tempdir()?;
    let root = dir.path();
    write_file(&root.join("todo/T-1.md"), "# todo")?;
    write_file(&root.join("done/T-2.md"), "# done")?;

    assert_eq!(
        resolve_task_path(root, "T-1.md")?,
        (root.join("todo/T-1.md"), TaskLocation::Todo)
    );
    assert_eq!(
        resolve_task_path(root, "T-2.md")?,
        (root.join("done/T-2.md"), TaskLocation::Done)
    );
    assert!(resolve_task_path(root, "T-3.md").is_err());
    Ok(())
}

#[test]
fn read_task_returns_content_for_valid_file() -> Result<()> {
    let dir = tempdir()?;
    let path = dir.path().join("todo/T-1.md");
    write_file(&path, "# task\nbody\n")?;
    assert_eq!(read_task(&path)?, "# task\nbody\n");
    Ok(())
}

#[test]
fn read_task_errors_for_missing_file() {
    let dir = tempdir().unwrap();
    let err = read_task(&dir.path().join("todo/missing.md")).unwrap_err();
    assert!(err.to_string().contains("reading task file"), "{err}");
}

#[test]
fn read_task_returns_empty_string_for_empty_file() -> Result<()> {
    let dir = tempdir()?;
    let path = dir.path().join("todo/T-2.md");
    write_file(&path, "")?;
    assert_eq!(read_task(&path)?, "");
    Ok(())
}

#[test]
fn append_orchestrator_section_handles_format_empty_and_markdown_notes() -> Result<()> {
    let dir = tempdir()?;
    let empty = dir.path().join("empty.md");
    let rich = dir.path().join("rich.md");
    write_file(&empty, "# Task")?;
    write_file(&rich, "# Task")?;

    append_orchestrator_section(&empty, "REJECT", None)?;
    append_orchestrator_section(&rich, "PASS", Some("- **Bold**\n- `code`\n- [link](url)"))?;

    let empty_text = fs::read_to_string(&empty)?;
    let rich_text = fs::read_to_string(&rich)?;
    assert!(empty_text.ends_with("\n## Orchestrator — REJECT\n\n\n"));
    assert!(rich_text.contains("## Orchestrator — PASS\n\n- **Bold**\n- `code`\n- [link](url)\n"));
    Ok(())
}

#[test]
fn task_location_display_and_debug_cover_known_paths() {
    assert_eq!(TaskLocation::Todo.as_str(), "todo");
    assert_eq!(TaskLocation::Done.to_string(), "done");
    assert_eq!(format!("{:?}", TaskLocation::Todo), "Todo");
    assert_eq!(
        TaskLocation::from_path(Path::new("/tmp/todo/T-1.md")),
        Some(TaskLocation::Todo)
    );
    assert_eq!(
        TaskLocation::from_path(Path::new("/tmp/done/T-1.md")),
        Some(TaskLocation::Done)
    );
    assert_eq!(
        TaskLocation::from_path(Path::new("/tmp/notes/T-1.md")),
        None
    );
}

#[test]
fn stage_verdict_conversions_display_and_debug_cover_all_variants() {
    for (raw, verdict) in [
        ("PASS", StageVerdict::Pass),
        ("REJECT", StageVerdict::Reject),
        ("FAIL", StageVerdict::Fail),
    ] {
        assert_eq!(StageVerdict::from_str(raw).unwrap(), verdict);
        assert_eq!(verdict.as_str(), raw);
        assert_eq!(verdict.to_string(), raw);
    }
    assert_eq!(format!("{:?}", StageVerdict::Reject), "Reject");
    assert!(!StageVerdict::Pass.counts_as_attempt());
    assert!(StageVerdict::Reject.counts_as_attempt());
    assert!(StageVerdict::Fail.counts_as_attempt());
    assert!(StageVerdict::from_str("maybe").is_err());
}

#[test]
fn stage_state_and_task_state_cover_construction_and_defaults() {
    let stage = StageState {
        stage: "red".into(),
        heading_line: 7,
        verdict: None,
    };
    let task = TaskState::new(
        PathBuf::from("todo/T-5.md"),
        TaskLocation::Todo,
        "red".into(),
        2,
        vec![stage.clone()],
    );

    assert_eq!(stage.stage, "red");
    assert_eq!(stage.heading_line, 7);
    assert_eq!(stage.verdict, None);
    assert_eq!(task.path, PathBuf::from("todo/T-5.md"));
    assert_eq!(task.location, TaskLocation::Todo);
    assert_eq!(task.status, "red");
    assert_eq!(task.attempts, 2);
    assert_eq!(task.stages, vec![stage]);
    assert_eq!(task.effective_status(), "red");
    assert!(!task.is_terminal());

    let done = TaskState::new(
        PathBuf::from("done/T-5.md"),
        TaskLocation::Done,
        "done".into(),
        0,
        vec![],
    );
    assert_eq!(done.location, TaskLocation::Done);
    assert_eq!(done.status, "done");
    assert_eq!(done.attempts, 0);
    assert!(done.stages.is_empty());
    assert_eq!(done.effective_status(), "done");
    assert!(done.is_terminal());
}

#[test]
fn discover_tasks_empty_dirs_returns_empty_vec() -> Result<()> {
    let dir = tempdir()?;
    fs::create_dir_all(dir.path().join("todo"))?;
    assert!(discover_tasks(dir.path())?.is_empty());
    Ok(())
}

#[test]
fn task_state_effective_status_uses_frontmatter_status_even_in_done() {
    let ts = TaskState::new(
        PathBuf::from("done/T-1.md"),
        TaskLocation::Done,
        "pending".into(),
        3,
        vec![StageState {
            stage: "swe".into(),
            heading_line: 1,
            verdict: None,
        }],
    );
    assert_eq!(ts.effective_status(), "pending");
    assert_eq!(ts.attempts, 3);
    assert_eq!(ts.stages.len(), 1);
}

#[test]
fn stage_verdict_from_str_is_case_insensitive() {
    assert_eq!("pass".parse::<StageVerdict>().unwrap(), StageVerdict::Pass);
    assert_eq!(
        "Reject".parse::<StageVerdict>().unwrap(),
        StageVerdict::Reject
    );
    assert_eq!("FAIL".parse::<StageVerdict>().unwrap(), StageVerdict::Fail);
    assert!("unknown".parse::<StageVerdict>().is_err());
}
