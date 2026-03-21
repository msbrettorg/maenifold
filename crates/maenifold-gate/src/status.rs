use crate::{
    CommandStubError, StatusArgs,
    db::{GateDb, TaskRow},
    fs, parser,
    types::{TaskLocation, TaskStage, TaskState},
};
use anyhow::{Result, anyhow};
use serde::Serialize;
use std::{collections::BTreeMap, path::Path};

const INCOMPLETE_AUDIT_TRAIL_NOTE: &str = "incomplete audit trail (location override)";

#[derive(Serialize)]
struct StatusOutput {
    file: String,
    location: String,
    status: &'static str,
    swe: bool,
    red: Option<&'static str>,
    blue: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    db_drift: Vec<String>,
}

pub fn run(args: &StatusArgs) -> Result<(), CommandStubError> {
    run_status(args, &args.root).map_err(|e| CommandStubError::new(e.to_string()))
}

pub fn run_status(args: &StatusArgs, root: &Path) -> Result<()> {
    let db = open_db(args, root)?;
    let check_drift = db.is_some();
    if let Some(id) = args.id.as_deref() {
        return run_one_by_id(root, id, args.json, db.as_ref(), check_drift);
    }
    run_all(root, args.json, db.as_ref(), check_drift)
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
    match parser::parse_task_content(&path, &content) {
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
            let state = TaskState::done_override_from_content(&path, &content);
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

fn run_all(root: &Path, json: bool, db: Option<&GateDb>, check_drift: bool) -> Result<()> {
    let mut tasks = fs::discover_tasks(root)?;
    let rows = db
        .map(GateDb::get_all_tasks)
        .transpose()?
        .unwrap_or_default()
        .into_iter()
        .map(|row| (row.file.clone(), row))
        .collect::<BTreeMap<_, _>>();
    let mut counts = BTreeMap::<&str, usize>::new();
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
                *counts.entry("error").or_default() += 1;
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
        let (state, note) = match TaskState::parse_quietly(&task.path, &content) {
            Ok(state) => (state, None),
            Err(parser::ParseTaskError::LocationStatusMismatch {
                location: TaskLocation::Done,
                ..
            }) => (
                TaskState::done_override_from_content(&task.path, &content),
                Some(INCOMPLETE_AUDIT_TRAIL_NOTE.to_string()),
            ),
            Err(err) => {
                *counts.entry("error").or_default() += 1;
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
        *counts.entry(state.effective_status().as_str()).or_default() += 1;
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
        state.map_or("error", |s| s.effective_status().as_str())
    );
    if let Some(state) = state {
        for stage in TaskStage::ALL {
            println!(
                "{:<9} {}",
                stage.as_str().to_ascii_uppercase(),
                stage_text(state, stage)
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

fn print_summary(counts: &BTreeMap<&str, usize>) {
    println!("{:<8} {:>5}", "STATUS", "COUNT");
    for key in ["pending", "red", "blue", "done"] {
        println!("{:<8} {:>5}", key, counts.get(key).copied().unwrap_or(0));
    }
    if let Some(errors) = counts.get("error") {
        println!("{:<8} {:>5}", "error", errors);
    }
    println!("---");
    println!("{:<8} {:>5}", "total", counts.values().sum::<usize>());
}

fn stage_value(state: &TaskState, stage: TaskStage) -> Option<Option<&'static str>> {
    state
        .stages
        .iter()
        .find(|item| item.stage == stage)
        .map(|item| item.verdict.map(|verdict| verdict.as_str()))
}

fn stage_text(state: &TaskState, stage: TaskStage) -> &'static str {
    match stage_value(state, stage) {
        Some(Some(verdict)) => verdict,
        Some(None) if stage == TaskStage::Swe => "yes",
        Some(None) => "-",
        None => "no",
    }
}

fn maybe_drift(check_drift: bool, state: &TaskState, row: Option<&TaskRow>) -> Vec<String> {
    check_drift.then(|| drift(state, row)).unwrap_or_default()
}

fn drift(state: &TaskState, row: Option<&TaskRow>) -> Vec<String> {
    let mut out = Vec::new();
    match row {
        Some(row) => {
            if row.status != state.effective_status().as_str() {
                out.push(format!(
                    "db status={} but file status={}",
                    row.status,
                    state.effective_status().as_str()
                ));
            }
            if row.location != state.location.as_str() {
                out.push(format!(
                    "db location={} but file location={}",
                    row.location,
                    state.location.as_str()
                ));
            }
        }
        None => out.push("missing from db cache".into()),
    }
    out
}

fn json_output(
    file: &str,
    location: &str,
    state: Option<&TaskState>,
    error: Option<String>,
    db_drift: Vec<String>,
) -> StatusOutput {
    StatusOutput {
        file: file.into(),
        location: location.into(),
        status: state.map_or("error", |item| item.effective_status().as_str()),
        swe: state.is_some_and(|item| stage_value(item, TaskStage::Swe).is_some()),
        red: state
            .and_then(|item| stage_value(item, TaskStage::Red))
            .flatten(),
        blue: state
            .and_then(|item| stage_value(item, TaskStage::Blue))
            .flatten(),
        error,
        db_drift,
    }
}
