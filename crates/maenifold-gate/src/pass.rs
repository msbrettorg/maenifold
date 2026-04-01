use std::path::PathBuf;

use anyhow::{Result, anyhow, bail};

use crate::{
    db::{GateDb, WorkLogEntry},
    frontmatter::read_task_frontmatter,
    fs::{append_orchestrator_section, move_to_done, resolve_file_path},
    pipeline::{PipelineDef, load_embedded_pipeline},
};

pub struct PassArgs {
    pub id: String,
    pub notes: Option<String>,
    pub db_path: PathBuf,
    pub root: PathBuf,
}

pub fn run(args: &PassArgs) -> Result<()> {
    let message = execute(args)?;
    eprintln!("{message}");
    Ok(())
}

pub fn execute(args: &PassArgs) -> Result<String> {
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
            .ok_or_else(|| anyhow!("pipeline '{}' has no stages", pipeline.name))?
    } else {
        let stage = pipeline
            .stages
            .iter()
            .find(|stage| stage.name == task.status)
            .ok_or_else(|| invalid_stage_error(&pipeline, &task.status))?;

        stage
            .transitions
            .get("pass")
            .ok_or_else(|| {
                anyhow!(
                    "pipeline '{}' stage '{}' has no pass transition",
                    pipeline.name,
                    task.status
                )
            })?
            .clone()
    };

    validate_transition_target(&pipeline, &next)?;

    append_orchestrator_section(&file_path, "PASS", args.notes.as_deref())?;
    if next == "done" {
        crate::frontmatter::write_frontmatter_status(&file_path, "done")?;
        move_to_done(&args.root, &task.file)?;
        db.update_task_status(&task.file, "done", "done")?;
    } else {
        crate::frontmatter::write_frontmatter_status(&file_path, &next)?;
        db.update_task_status(&task.file, &next, "todo")?;
    }
    db.insert_work_log(&WorkLogEntry {
        file: &task.file,
        stage: "orchestrator",
        agent_model: None,
        verdict: Some("PASS"),
        duration_secs: None,
        notes: args.notes.as_deref(),
    })?;
    Ok(if next == "done" {
        format!(
            "{}: {} → done [PASS] ✓ moved to done/",
            args.id, task.status
        )
    } else {
        format!("{}: {} → {} [PASS]", args.id, task.status, next)
    })
}

fn invalid_stage_error(pipeline: &PipelineDef, status: &str) -> anyhow::Error {
    anyhow!(
        "task status '{}' is not a valid pipeline stage in pipeline '{}'; valid stages: {}",
        status,
        pipeline.name,
        valid_stage_list(pipeline)
    )
}

fn validate_transition_target(pipeline: &PipelineDef, target: &str) -> Result<()> {
    if matches!(target, "done" | "pending")
        || pipeline.stages.iter().any(|stage| stage.name == target)
    {
        return Ok(());
    }

    bail!(
        "pipeline '{}' transitions to unknown stage '{}'; valid stages: {}",
        pipeline.name,
        target,
        valid_stage_list(pipeline)
    )
}

fn valid_stage_list(pipeline: &PipelineDef) -> String {
    pipeline
        .stages
        .iter()
        .map(|stage| stage.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::pipeline::StageDef;

    use super::*;

    #[test]
    fn invalid_stage_error_lists_pipeline_and_valid_stages() {
        let pipeline = PipelineDef {
            name: "gsd".into(),
            stages: vec![
                StageDef {
                    name: "research".into(),
                    verdicts: vec![],
                    transitions: HashMap::from([("pass".into(), "plan".into())]),
                },
                StageDef {
                    name: "plan".into(),
                    verdicts: vec!["pass".into()],
                    transitions: HashMap::from([("pass".into(), "done".into())]),
                },
            ],
        };

        let err = invalid_stage_error(&pipeline, "swe");
        let text = err.to_string();
        assert!(text.contains("pipeline 'gsd'"));
        assert!(text.contains("valid stages: research, plan"));
    }

    #[test]
    fn transition_target_must_exist_in_pipeline_or_be_terminal() {
        let pipeline = PipelineDef {
            name: "gsd".into(),
            stages: vec![StageDef {
                name: "research".into(),
                verdicts: vec![],
                transitions: HashMap::new(),
            }],
        };

        let err = validate_transition_target(&pipeline, "swe").unwrap_err();
        let text = err.to_string();
        assert!(text.contains("pipeline 'gsd' transitions to unknown stage 'swe'"));
        assert!(text.contains("valid stages: research"));
        assert!(validate_transition_target(&pipeline, "done").is_ok());
    }
}
