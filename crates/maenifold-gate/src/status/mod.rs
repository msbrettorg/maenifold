use crate::{
    CommandStubError, StatusArgs,
    db::GateDb,
    fs, parser,
    pipeline::load_pipeline_def,
    types::{TaskLocation, TaskState},
};
use anyhow::{Result, anyhow};
use std::{collections::BTreeMap, path::Path};

mod output;
use output::{discovered_stages, json_output, maybe_drift, print_summary, stage_text};

const INCOMPLETE_AUDIT_TRAIL_NOTE: &str = "incomplete audit trail (location override)";

pub fn run(args: &StatusArgs) -> Result<(), CommandStubError> {
    run_status(args, &args.root).map_err(|e| CommandStubError::new(e.to_string()))
}

pub fn run_status(args: &StatusArgs, root: &Path) -> Result<()> {
    let pipeline = load_pipeline_def(args.pipeline_path.as_deref(), root)?;
    let db = open_db(args, root)?;
    let check_drift = db.is_some();
    if let Some(id) = args.id.as_deref() {
        return run_one_by_id(root, &pipeline, id, args.json, db.as_ref(), check_drift);
    }
    run_all(root, &pipeline, args.json, db.as_ref(), check_drift)
}

fn open_db(args: &StatusArgs, root: &Path) -> Result<Option<GateDb>> {
    match (args.db.as_deref(), args.id.as_ref()) {
        (Some(path), _) => Ok(Some(GateDb::open(path)?)),
        (None, Some(_)) => Ok(Some(GateDb::open(&root.join("pipeline.db"))?)),
        (None, None) => Ok(None),
    }
}

fn run_one_by_id(
    root: &Path,
    pipeline: &crate::pipeline::PipelineDef,
    id: &str,
    json: bool,
    db: Option<&GateDb>,
    check_drift: bool,
) -> Result<()> {
    let db = db.ok_or_else(|| anyhow!("task ID lookup requires a database"))?;
    let row = db
        .get_task_by_id(id)?
        .ok_or_else(|| anyhow!("unknown task ID: {id}"))?;
    let (path, location) = fs::resolve_task_path(root, &row.file)?;
    let content = fs::read_task(&path)?;
    match parser::parse_task_content_with_pipeline(&path, &content, pipeline) {
        Ok(state) => output_one(
            &row.file,
            location.as_str(),
            Some(&state),
            None,
            maybe_drift(check_drift, &state, Some(&row)),
            json,
        )?,
        Err(parser::ParseTaskError::LocationStatusMismatch {
            location: TaskLocation::Done,
            ..
        }) => {
            let state =
                TaskState::done_override_from_content_with_pipeline(&path, &content, pipeline);
            output_one(
                &row.file,
                location.as_str(),
                Some(&state),
                Some(INCOMPLETE_AUDIT_TRAIL_NOTE.into()),
                maybe_drift(check_drift, &state, Some(&row)),
                json,
            )?
        }
        Err(err) => output_one(
            &row.file,
            location.as_str(),
            None,
            Some(err.to_string()),
            vec![],
            json,
        )?,
    }
    Ok(())
}

fn run_all(
    root: &Path,
    pipeline: &crate::pipeline::PipelineDef,
    json: bool,
    db: Option<&GateDb>,
    check_drift: bool,
) -> Result<()> {
    let mut tasks = fs::discover_tasks(root)?;
    let rows = db
        .map(GateDb::get_all_tasks)
        .transpose()?
        .unwrap_or_default()
        .into_iter()
        .map(|row| (row.file.clone(), row))
        .collect::<BTreeMap<_, _>>();
    let mut counts = BTreeMap::<String, usize>::new();
    let mut out = Vec::new();
    tasks.sort_by(|a, b| a.path.cmp(&b.path));
    for task in tasks {
        let file = task
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        let content = match fs::read_task(&task.path) {
            Ok(content) => content,
            Err(err) => {
                *counts.entry("error".to_string()).or_default() += 1;
                let message = err.to_string();
                if json {
                    out.push(json_output(
                        file,
                        task.location.as_str(),
                        None,
                        Some(message),
                        vec![],
                    ));
                } else {
                    eprintln!("warning: {file}: status=error: {message}");
                }
                continue;
            }
        };
        let (state, note) =
            match TaskState::parse_quietly_with_pipeline(&task.path, &content, pipeline) {
                Ok(state) => (state, None),
                Err(parser::ParseTaskError::LocationStatusMismatch {
                    location: TaskLocation::Done,
                    ..
                }) => (
                    TaskState::done_override_from_content_with_pipeline(
                        &task.path, &content, pipeline,
                    ),
                    Some(INCOMPLETE_AUDIT_TRAIL_NOTE.to_string()),
                ),
                Err(err) => {
                    *counts.entry("error".to_string()).or_default() += 1;
                    let message = err.to_string();
                    if json {
                        out.push(json_output(
                            file,
                            task.location.as_str(),
                            None,
                            Some(message),
                            vec![],
                        ));
                    } else {
                        eprintln!("warning: {file}: status=error: {message}");
                    }
                    continue;
                }
            };
        *counts
            .entry(state.effective_status().to_string())
            .or_default() += 1;
        let db_drift = maybe_drift(check_drift, &state, rows.get(file));
        if json {
            out.push(json_output(
                file,
                task.location.as_str(),
                Some(&state),
                note,
                db_drift,
            ));
        } else {
            for item in db_drift {
                eprintln!("warning: {file}: {item}");
            }
        }
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        print_summary(&counts);
    }
    Ok(())
}

fn output_one(
    file: &str,
    location: &str,
    state: Option<&TaskState>,
    error: Option<String>,
    db_drift: Vec<String>,
    json: bool,
) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json_output(file, location, state, error, db_drift))?
        );
        return Ok(());
    }
    println!("{:<9} {}", "FILE", file);
    println!("{:<9} {}", "LOCATION", location);
    println!(
        "{:<9} {}",
        "STATUS",
        state.map_or("error", |s| s.effective_status())
    );
    if let Some(state) = state {
        for stage_name in discovered_stages(state) {
            println!(
                "{:<9} {}",
                stage_name.to_ascii_uppercase(),
                stage_text(state, &stage_name)
            );
        }
    }
    if let Some(error) = error.as_deref() {
        println!("{:<9} {}", "ERROR", error);
        eprintln!("warning: {file}: {error}");
    }
    for item in db_drift {
        eprintln!("warning: {file}: {item}");
    }
    Ok(())
}
