use std::path::PathBuf;

use anyhow::{Result, anyhow, bail};

use crate::{
    db::{GateDb, WorkLogEntry},
    frontmatter::read_task_frontmatter,
    fs::{append_orchestrator_section, resolve_file_path},
    pipeline::load_embedded_pipeline,
};

pub struct RejectArgs {
    pub id: String,
    pub notes: Option<String>,
    pub db_path: PathBuf,
    pub root: PathBuf,
}

pub fn run(args: &RejectArgs) -> Result<()> {
    let db = GateDb::open(&args.db_path)?;
    let task = db
        .get_task_by_id(&args.id)?
        .ok_or_else(|| anyhow!("unknown task ID: {}", args.id))?;
    if task.location != "todo" {
        bail!("task file '{}' is not in todo/", task.file);
    }
    let file_path = resolve_file_path(&args.root, &task.file)?;
    let pipeline_name = read_task_frontmatter(&file_path)?
        .0
        .and_then(|frontmatter| {
            frontmatter
                .get("pipeline")
                .and_then(serde_yaml::Value::as_str)
                .map(str::to_owned)
        })
        .ok_or_else(|| anyhow!("task '{}' has no 'pipeline' field in frontmatter", args.id))?;
    let pipeline = load_embedded_pipeline(&pipeline_name)?;
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
    append_orchestrator_section(&file_path, "REJECT", args.notes.as_deref())?;
    crate::frontmatter::write_frontmatter_status(&file_path, &target)?;
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
