use std::{fs, path::Path};

use anyhow::Result;
use tempfile::tempdir;

use super::{AppendArgs, run};
use crate::{SyncArgs, db::GateDb, sync::run_sync};

#[test]
fn append_writes_section_to_file() -> Result<()> {
    let dir = tempdir()?;
    let root = dir.path();
    let db_path = sync_task(root, "T-55.3-gate-append-command.md")?;
    let path = root.join("todo/T-55.3-gate-append-command.md");
    run(&AppendArgs {
        id: "T-55.3".into(),
        notes: "wave 2 dispatch".into(),
        agent: "swe".into(),
        db_path,
        root: root.to_path_buf(),
    })?;
    assert!(fs::read_to_string(path)?.contains("\n## Swe Notes\n\nwave 2 dispatch\n"));
    Ok(())
}

#[test]
fn append_unknown_id_errors() -> Result<()> {
    let dir = tempdir()?;
    let root = dir.path();
    let db_path = root.join("gate.db");
    let err = run(&AppendArgs {
        id: "T-404".into(),
        notes: "missing".into(),
        agent: "swe".into(),
        db_path,
        root: root.to_path_buf(),
    })
    .unwrap_err();
    assert!(err.to_string().contains("unknown task ID"));
    Ok(())
}

#[test]
fn append_does_not_change_status() -> Result<()> {
    let dir = tempdir()?;
    let root = dir.path();
    let db_path = sync_task(root, "T-55.3-gate-append-command.md")?;
    let before = GateDb::open(&db_path)?
        .get_task_by_id("T-55.3")?
        .expect("task should exist")
        .status;
    run(&AppendArgs {
        id: "T-55.3".into(),
        notes: "still pending".into(),
        agent: "red-team".into(),
        db_path: db_path.clone(),
        root: root.to_path_buf(),
    })?;
    let after = GateDb::open(&db_path)?
        .get_task_by_id("T-55.3")?
        .expect("task should exist")
        .status;
    assert_eq!(before, after);
    Ok(())
}

fn sync_task(root: &Path, file: &str) -> Result<std::path::PathBuf> {
    fs::create_dir_all(root.join("todo"))?;
    fs::write(
        root.join("todo").join(file),
        "---\npipeline: sdd\n---\n\n# T-55.3 — gate append\n\n**Requirement**: FR-42\n",
    )?;
    let db_path = root.join("gate.db");
    run_sync(
        &SyncArgs {
            db: Some(db_path.clone()),
            root: root.to_path_buf(),
            rebuild: false,
        },
        root,
    )?;
    Ok(db_path)
}
