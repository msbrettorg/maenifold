use std::path::PathBuf;

use anyhow::{Result, anyhow, bail};

use crate::{
    db::{GateDb, WorkLogEntry},
    frontmatter::read_task_frontmatter,
    fs::{append_orchestrator_section, move_to_done, resolve_file_path},
    pipeline::load_embedded_pipeline,
};

pub struct PassArgs {
    pub id: String,
    pub notes: Option<String>,
    pub db_path: PathBuf,
    pub root: PathBuf,
}

pub fn run(args: &PassArgs) -> Result<()> {
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

    let next = if task.status == "pending" {
        pipeline
            .stages
            .first()
            .map(|stage| stage.name.clone())
            .ok_or_else(|| anyhow!("pipeline has no stages"))?
    } else {
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

        stage
            .transitions
            .get("pass")
            .ok_or_else(|| anyhow!("stage '{}' has no 'pass' transition", task.status))?
            .clone()
    };

    append_orchestrator_section(&file_path, "PASS", args.notes.as_deref())?;
    if next == "done" {
        crate::frontmatter::write_frontmatter_status(&file_path, "done")?;
        move_to_done(&args.root, &task.file)?;
        db.update_task_status(&task.file, "done", "done")?;
        eprintln!(
            "{}: {} → done [PASS] ✓ moved to done/",
            args.id, task.status
        );
    } else {
        crate::frontmatter::write_frontmatter_status(&file_path, &next)?;
        db.update_task_status(&task.file, &next, "todo")?;
        eprintln!("{}: {} → {} [PASS]", args.id, task.status, next);
    }
    db.insert_work_log(&WorkLogEntry {
        file: &task.file,
        stage: "orchestrator",
        agent_model: None,
        verdict: Some("PASS"),
        duration_secs: None,
        notes: args.notes.as_deref(),
    })?;
    Ok(())
}
