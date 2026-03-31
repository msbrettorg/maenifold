use crate::{CommandStubError, StatusArgs, db::GateDb, fs, types::TaskState};
use anyhow::{Result, anyhow};
use std::{collections::BTreeMap, path::Path};

mod details;
mod output;
use output::{json_output, maybe_drift, print_summary};

struct StatusContext<'a> {
    root: &'a Path,
    json: bool,
    db: Option<&'a GateDb>,
    check_drift: bool,
}

struct OutputEntry<'a> {
    file: &'a str,
    location: &'a str,
    state: Option<&'a TaskState>,
    content: Option<&'a str>,
    error: Option<String>,
    db_drift: Vec<String>,
}

pub fn run(args: &StatusArgs) -> Result<(), CommandStubError> {
    run_status(args, &args.root).map_err(|e| CommandStubError::new(e.to_string()))
}

pub fn run_status(args: &StatusArgs, root: &Path) -> Result<()> {
    let db = open_db(args, root)?;
    let context = StatusContext {
        root,
        json: args.json,
        db: db.as_ref(),
        check_drift: db.is_some(),
    };
    if let Some(id) = args.id.as_deref() {
        return run_one_by_id(&context, id);
    }
    run_all(&context)
}

fn open_db(args: &StatusArgs, root: &Path) -> Result<Option<GateDb>> {
    match (args.db.as_deref(), args.id.as_ref()) {
        (Some(path), _) => Ok(Some(GateDb::open(path)?)),
        (None, Some(_)) => Ok(Some(GateDb::open(&root.join("gate.db"))?)),
        (None, None) => Ok(None),
    }
}

fn run_one_by_id(context: &StatusContext<'_>, id: &str) -> Result<()> {
    let db = context
        .db
        .ok_or_else(|| anyhow!("task ID lookup requires a database"))?;
    let row = db
        .get_task_by_id(id)?
        .ok_or_else(|| anyhow!("unknown task ID: {id}"))?;
    let (path, location) = fs::resolve_task_path(context.root, &row.file)?;
    let content = fs::read_task(&path)?;
    let pipeline = crate::sync::load_task_pipeline(&path, &content)?;
    let state = crate::sync::resolve_task_state(&path, &content, &pipeline)?;
    output_one(
        context,
        OutputEntry {
            file: &row.file,
            location: location.as_str(),
            state: Some(&state),
            content: Some(&content),
            error: None,
            db_drift: maybe_drift(context.check_drift, &state, Some(&row)),
        },
    )?;
    Ok(())
}

fn run_all(context: &StatusContext<'_>) -> Result<()> {
    let mut tasks = fs::discover_tasks(context.root)?;
    let rows = context
        .db
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
                if context.json {
                    out.push(json_output(
                        file,
                        task.location.as_str(),
                        None,
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
        let pipeline = match crate::sync::load_task_pipeline(&task.path, &content) {
            Ok(pipeline) => pipeline,
            Err(err) => {
                *counts.entry("error".to_string()).or_default() += 1;
                let message = err.to_string();
                if context.json {
                    out.push(json_output(
                        file,
                        task.location.as_str(),
                        None,
                        Some(&content),
                        Some(message),
                        vec![],
                    ));
                } else {
                    eprintln!("warning: {file}: status=error: {message}");
                }
                continue;
            }
        };
        let state = match crate::sync::resolve_task_state(&task.path, &content, &pipeline) {
            Ok(state) => state,
            Err(err) => {
                *counts.entry("error".to_string()).or_default() += 1;
                let message = err.to_string();
                if context.json {
                    out.push(json_output(
                        file,
                        task.location.as_str(),
                        None,
                        Some(&content),
                        Some(message),
                        vec![],
                    ));
                } else {
                    eprintln!("warning: {file}: status=error: {message}");
                }
                continue;
            }
        };
        let status_key = state.status.clone();
        *counts.entry(status_key).or_default() += 1;
        let db_drift = maybe_drift(context.check_drift, &state, rows.get(file));
        if context.json {
            out.push(json_output(
                file,
                task.location.as_str(),
                Some(&state),
                Some(&content),
                None,
                db_drift,
            ));
        } else {
            for item in db_drift {
                eprintln!("warning: {file}: {item}");
            }
        }
    }
    if context.json {
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        print_summary(&counts);
    }
    Ok(())
}

fn output_one(context: &StatusContext<'_>, entry: OutputEntry<'_>) -> Result<()> {
    if context.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json_output(
                entry.file,
                entry.location,
                entry.state,
                entry.content,
                entry.error,
                entry.db_drift,
            ))?
        );
        return Ok(());
    }
    println!("{:<9} {}", "FILE", entry.file);
    println!("{:<9} {}", "LOCATION", entry.location);
    println!(
        "{:<9} {}",
        "STATUS",
        entry.state.map_or("error", |s| &s.status)
    );
    if let Some(error) = entry.error.as_deref() {
        println!("{:<9} {}", "ERROR", error);
        eprintln!("warning: {}: {error}", entry.file);
    }
    for item in entry.db_drift {
        eprintln!("warning: {}: {item}", entry.file);
    }
    Ok(())
}
