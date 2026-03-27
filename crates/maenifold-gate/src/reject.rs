use std::path::PathBuf;

use anyhow::{Result, anyhow, bail};

use crate::{
    db::{GateDb, WorkLogEntry},
    fs::{append_orchestrator_section, resolve_file_path},
    pipeline::load_pipeline_def,
};

pub struct RejectArgs {
    pub id: String,
    pub notes: Option<String>,
    pub pipeline_path: Option<PathBuf>,
    pub db_path: PathBuf,
    pub root: PathBuf,
}

pub fn run(args: &RejectArgs) -> Result<()> {
    let pipeline = load_pipeline_def(args.pipeline_path.as_deref(), &args.root)?;
    let db = GateDb::open(&args.db_path)?;
    let task = db
        .get_task_by_id(&args.id)?
        .ok_or_else(|| anyhow!("unknown task ID: {}", args.id))?;
    if task.location != "todo" {
        bail!("task file '{}' is not in todo/", task.file);
    }
    let stage = pipeline
        .stages
        .iter()
        .find(|stage| stage.name == task.status)
        .ok_or_else(|| {
            anyhow!(
                "task status '{}' is not a valid pipeline stage",
                task.status
            )
        })?;
    let stage_name = stage.name.clone();
    let target = stage
        .transitions
        .get("reject")
        .ok_or_else(|| anyhow!("stage '{}' has no reject transition", stage_name))?
        .clone();
    append_orchestrator_section(
        &resolve_file_path(&args.root, &task.file)?,
        "REJECT",
        args.notes.as_deref(),
    )?;
    db.update_task_status(&task.file, &target, "todo")?;
    db.increment_attempts(&task.file)?;
    db.insert_work_log(&WorkLogEntry {
        file: &task.file,
        stage: "orchestrator",
        agent_model: None,
        verdict: Some("REJECT"),
        duration_secs: None,
        notes: args.notes.as_deref(),
    })?;
    println!("{}: {} → {} [REJECT]", task.id, stage_name, target);
    Ok(())
}

#[cfg(test)]
#[path = "reject_tests.rs"]
mod tests;
