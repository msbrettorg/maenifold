use std::{collections::BTreeMap, path::Path};
use anyhow::Result;
use serde::Serialize;
use crate::{CommandStubError, StatusArgs, db::{GateDb, TaskRow}, fs, parser, types::{TaskStage, TaskState}};

#[derive(Serialize)]
struct StatusOutput {
    file: String, location: String, status: &'static str, swe: bool,
    red: Option<&'static str>, blue: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")] error: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")] db_drift: Vec<String>,
}

pub fn run(args: &StatusArgs) -> Result<(), CommandStubError> {
    run_status(args, args.root.as_deref().unwrap_or(Path::new("."))).map_err(|e| CommandStubError::new(e.to_string()))
}

pub fn run_status(args: &StatusArgs, root: &Path) -> Result<()> {
    let db = args.db.as_deref().map(GateDb::open).transpose()?;
    let check_drift = db.is_some();
    if args.task.as_os_str().is_empty() { return run_all(root, args.json, db.as_ref()); }
    let file = args.task.file_name().unwrap_or(args.task.as_os_str()).to_string_lossy().into_owned();
    let (path, location) = fs::resolve_task_path(root, &file)?;
    let row = db.as_ref().map(|db| db.get_task(&file)).transpose()?.flatten();
    match parser::parse_task_file(&path) {
        Ok(state) => output_one(&file, location.as_str(), Some(&state), None, maybe_drift(check_drift, &state, row.as_ref()), args.json)?,
        Err(err) => output_one(&file, location.as_str(), None, Some(err.to_string()), vec![], args.json)?,
    }
    Ok(())
}

fn run_all(root: &Path, json: bool, db: Option<&GateDb>) -> Result<()> {
    let mut tasks = fs::discover_tasks(root)?;
    let check_drift = db.is_some();
    let rows = db.map(GateDb::get_all_tasks).transpose()?.unwrap_or_default().into_iter().map(|row| (row.file.clone(), row)).collect::<BTreeMap<_, _>>();
    let mut counts = BTreeMap::<&str, usize>::new();
    let mut out = Vec::new();
    tasks.sort_by(|a, b| a.path.cmp(&b.path));
    for task in tasks {
        let file = task.path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        match parser::parse_task_file(&task.path) {
            Ok(state) => {
                *counts.entry(state.status.as_str()).or_default() += 1;
                let db_drift = maybe_drift(check_drift, &state, rows.get(file));
                if json { out.push(json_output(file, task.location.as_str(), Some(&state), None, db_drift)); }
                else { for item in db_drift { eprintln!("warning: {file}: {item}"); } }
            }
            Err(err) => {
                *counts.entry("error").or_default() += 1;
                let message = err.to_string();
                if json { out.push(json_output(file, task.location.as_str(), None, Some(message), vec![])); }
                else { eprintln!("warning: {file}: status=error: {message}"); }
            }
        }
    }
    if json { println!("{}", serde_json::to_string_pretty(&out)?); } else { print_summary(&counts); }
    Ok(())
}

fn output_one(file: &str, location: &str, state: Option<&TaskState>, error: Option<String>, db_drift: Vec<String>, json: bool) -> Result<()> {
    if json { println!("{}", serde_json::to_string_pretty(&json_output(file, location, state, error, db_drift))?); return Ok(()); }
    println!("{:<9} {}", "FILE", file);
    println!("{:<9} {}", "LOCATION", location);
    println!("{:<9} {}", "STATUS", state.map_or("error", |s| s.status.as_str()));
    if let Some(state) = state { for stage in TaskStage::ALL { println!("{:<9} {}", stage.as_str().to_ascii_uppercase(), stage_text(state, stage)); } }
    if let Some(error) = error.as_deref() { println!("{:<9} {}", "ERROR", error); eprintln!("warning: {file}: {error}"); }
    for item in db_drift { eprintln!("warning: {file}: {item}"); }
    Ok(())
}

fn print_summary(counts: &BTreeMap<&str, usize>) {
    println!("{:<8} {:>5}", "STATUS", "COUNT");
    for key in ["pending", "red", "blue", "done"] { println!("{:<8} {:>5}", key, counts.get(key).copied().unwrap_or(0)); }
    if let Some(errors) = counts.get("error") { println!("{:<8} {:>5}", "error", errors); }
    println!("---");
    println!("{:<8} {:>5}", "total", counts.values().sum::<usize>());
}

fn stage_value(state: &TaskState, stage: TaskStage) -> Option<Option<&'static str>> {
    state.stages.iter().find(|item| item.stage == stage).map(|item| item.verdict.map(|verdict| verdict.as_str()))
}

fn stage_text(state: &TaskState, stage: TaskStage) -> &'static str {
    match stage_value(state, stage) { Some(Some(verdict)) => verdict, Some(None) if stage == TaskStage::Swe => "yes", Some(None) => "-", None => "no" }
}

fn maybe_drift(check_drift: bool, state: &TaskState, row: Option<&TaskRow>) -> Vec<String> { check_drift.then(|| drift(state, row)).unwrap_or_default() }

fn drift(state: &TaskState, row: Option<&TaskRow>) -> Vec<String> {
    let mut out = Vec::new();
    match row {
        Some(row) => {
            if row.status != state.status.as_str() { out.push(format!("db status={} but file status={}", row.status, state.status.as_str())); }
            if row.location != state.location.as_str() { out.push(format!("db location={} but file location={}", row.location, state.location.as_str())); }
        }
        None => out.push("missing from db cache".into()),
    }
    out
}

fn json_output(file: &str, location: &str, state: Option<&TaskState>, error: Option<String>, db_drift: Vec<String>) -> StatusOutput {
    StatusOutput {
        file: file.into(), location: location.into(), status: state.map_or("error", |item| item.status.as_str()),
        swe: state.is_some_and(|item| stage_value(item, TaskStage::Swe).is_some()),
        red: state.and_then(|item| stage_value(item, TaskStage::Red)).flatten(),
        blue: state.and_then(|item| stage_value(item, TaskStage::Blue)).flatten(), error, db_drift,
    }
}
