use std::path::PathBuf;

use anyhow::{Result, anyhow, bail};

use crate::{
    db::GateDb,
    fs::{append_orchestrator_section, move_to_done, resolve_file_path},
    pipeline::load_pipeline_def,
};

pub struct PassArgs {
    pub id: String,
    pub notes: Option<String>,
    pub pipeline_path: Option<PathBuf>,
    pub db_path: PathBuf,
    pub root: PathBuf,
}

pub fn run(args: &PassArgs) -> Result<()> {
    let pipeline = load_pipeline_def(args.pipeline_path.as_deref(), &args.root)?;
    let db = GateDb::open(&args.db_path)?;
    let task = db
        .get_task_by_id(&args.id)?
        .ok_or_else(|| anyhow!("unknown task ID: {}", args.id))?;
    if task.location != "todo" {
        bail!("task file '{}' is not in todo/", task.file);
    }

    let next = if task.status == "pending" {
        pipeline
            .stages
            .first()
            .map(|stage| stage.name.clone())
            .ok_or_else(|| anyhow!("pipeline has no stages"))?
    } else {
        pipeline
            .stages
            .iter()
            .find(|stage| stage.name == task.status)
            .ok_or_else(|| {
                anyhow!(
                    "task status '{}' is not a valid pipeline stage",
                    task.status
                )
            })?
            .transitions
            .get("pass")
            .ok_or_else(|| anyhow!("stage '{}' has no 'pass' transition", task.status))?
            .clone()
    };

    let file_path = resolve_file_path(&args.root, &task.file)?;
    append_orchestrator_section(&file_path, "PASS", args.notes.as_deref())?;
    if next == "done" {
        move_to_done(&args.root, &task.file)?;
        db.update_task_status(&task.file, "done", "done")?;
        eprintln!(
            "{}: {} → done [PASS] ✓ moved to done/",
            args.id, task.status
        );
    } else {
        db.update_task_status(&task.file, &next, "todo")?;
        eprintln!("{}: {} → {} [PASS]", args.id, task.status, next);
    }
    db.insert_work_log(
        &task.file,
        "orchestrator",
        None,
        Some("PASS"),
        None,
        args.notes.as_deref(),
    )?;
    Ok(())
}
