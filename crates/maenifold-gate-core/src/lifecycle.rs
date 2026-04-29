use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::LazyLock;

use regex::Regex;
use serde_json::{Value, json};

use crate::error::GateError;
use crate::section_factory::CanonicalSectionFactory;
use crate::state_machine::StateMachine;
use crate::status_filter::StatusFilter;
use crate::task_file::{GateProjectLock, TaskFileLock};
use crate::task_metadata::TaskMetadataParser;
use crate::task_repository::TaskPathSet;
use crate::task_repository::TaskRepository;
use crate::task_title_slug;

#[path = "lifecycle_helpers.rs"]
mod lifecycle_helpers;
pub use lifecycle_helpers::resolve_definition;
use lifecycle_helpers::{
    active_task_state, apply_outcome, canonical_root, status_for_document, status_row,
};

#[derive(Debug, Clone)]
pub struct GateContext {
    pub root: PathBuf,
    pub requested_pipeline_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StatusArgs {
    pub context: GateContext,
    pub filter: StatusFilter,
}

#[derive(Debug, Clone)]
pub struct SyncArgs {
    pub context: GateContext,
    pub rebuild: bool,
}

#[derive(Debug, Clone)]
pub struct TaskArgs {
    pub context: GateContext,
    pub task_id: String,
}

#[derive(Debug, Clone)]
pub struct CreateTaskArgs {
    pub context: GateContext,
    pub task_id: String,
    pub title: String,
    pub body: Option<String>,
    pub priority: Option<String>,
    pub sprint: Option<u64>,
    pub parent: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StageNoteArgs {
    pub context: GateContext,
    pub task_id: String,
    pub agent: String,
    pub notes: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GateLifecycle;

static CREATE_TASK_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^T-(\d+)\.\d+$").expect("gate create task id regex"));

impl GateLifecycle {
    pub fn create(args: CreateTaskArgs) -> Result<Value, GateError> {
        let requested_root = args.context.root.clone();
        let root = canonical_root(&args.context.root)?;

        let captures = CREATE_TASK_ID_RE.captures(&args.task_id).ok_or_else(|| {
            GateError::Cli(format!(
                "invalid taskId '{}': expected regex ^T-\\d+\\.\\d+$",
                args.task_id
            ))
        })?;
        let title = args.title.trim();
        if title.is_empty() {
            return Err(GateError::Cli(
                "title is required and must not be empty".to_string(),
            ));
        }
        let pipeline = args
            .context
            .requested_pipeline_name
            .clone()
            .ok_or_else(|| GateError::Cli("pipeline is required".to_string()))?;
        resolve_definition(&root, &pipeline).map_err(|error| {
            GateError::Cli(format!(
                "{error}. Known built-in pipelines include sdd, tdd, gsd, lightweight"
            ))
        })?;

        let priority = args.priority.unwrap_or_else(|| "P2".to_string());
        if !matches!(priority.as_str(), "P0" | "P1" | "P2" | "P3" | "P4") {
            return Err(GateError::Cli(format!(
                "invalid priority '{priority}': expected one of P0, P1, P2, P3, P4"
            )));
        }
        let sprint = match args.sprint {
            Some(value) => value,
            None => captures
                .get(1)
                .expect("task id sprint capture")
                .as_str()
                .parse::<u64>()
                .map_err(|error| {
                    GateError::Cli(format!("failed to parse taskId sprint: {error}"))
                })?,
        };
        if let Some(parent) = args.parent.as_deref()
            && !CREATE_TASK_ID_RE.is_match(parent)
        {
            return Err(GateError::Cli(format!(
                "invalid parent task id '{parent}': expected regex ^T-\\d+\\.\\d+$"
            )));
        }

        let slug = task_title_slug(title);
        let markdown = create_task_markdown(
            &args.task_id,
            title,
            &pipeline,
            &priority,
            sprint,
            args.parent.as_deref(),
            args.body.as_deref(),
        );
        {
            let _project_lock = GateProjectLock::acquire_exclusive(&root)?;
            let _task_lock = TaskFileLock::acquire(&root, &args.task_id)?;
            TaskRepository::create_task(&root, &args.task_id, &slug, &markdown)?;
        }
        let display_path = requested_root
            .join("todo")
            .join(format!("{}-{}.md", args.task_id, slug));
        let _ = Self::sync(SyncArgs {
            context: GateContext {
                root: root.clone(),
                requested_pipeline_name: args.context.requested_pipeline_name.clone(),
            },
            rebuild: false,
        })?;

        Ok(json!({
            "taskId": args.task_id,
            "path": display_path,
            "pipeline": pipeline,
            "sprint": sprint,
        }))
    }

    pub fn status(args: StatusArgs) -> Result<Value, GateError> {
        let root = canonical_root(&args.context.root)?;
        let _lock = GateProjectLock::acquire_shared(&root)?;
        let mut rows = Vec::new();
        for task in TaskRepository::list_tasks(root.clone())? {
            // Skip done/ tasks early when not requested — avoids replaying
            // incomplete stage records for legacy/manually-moved tasks.
            if task.location == "done" && !args.filter.include_done {
                continue;
            }
            let row = match status_row(
                &root,
                &task,
                args.context.requested_pipeline_name.as_deref(),
            ) {
                Ok(row) => row,
                Err(e) => error_status_row(&root, &task, e),
            };
            if args.filter.matches(&row) {
                rows.push(row);
            }
        }
        Ok(Value::Array(rows))
    }

    pub fn sync(args: SyncArgs) -> Result<Value, GateError> {
        let root = canonical_root(&args.context.root)?;
        let _lock = GateProjectLock::acquire_exclusive(&root)?;
        let gate_dir = root.join(".gate");
        if !gate_dir.exists() {
            std::fs::create_dir_all(&gate_dir).map_err(|e| {
                GateError::Io(std::io::Error::new(
                    e.kind(),
                    format!("failed to create .gate/: {e}"),
                ))
            })?;
        }

        let gitignore_path = root.join(".gitignore");
        let needs_entry = if gitignore_path.exists() {
            let content = std::fs::read_to_string(&gitignore_path).unwrap_or_default();
            !content
                .lines()
                .any(|line| line.trim() == ".gate/" || line.trim() == ".gate")
        } else {
            true
        };
        if needs_entry {
            use std::io::Write;

            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&gitignore_path)
                .map_err(|e| {
                    GateError::Io(std::io::Error::new(
                        e.kind(),
                        format!("failed to open .gitignore: {e}"),
                    ))
                })?;
            writeln!(file, "\n.gate/").map_err(|e| {
                GateError::Io(std::io::Error::new(
                    e.kind(),
                    format!("failed to write .gitignore: {e}"),
                ))
            })?;
        }
        let tasks = TaskRepository::list_tasks(root.clone())?;
        write_gate_db_snapshot(&root, &gate_dir, &tasks)?;
        let task_count = tasks.len();
        Ok(json!({
            "root": root,
            "pipeline": args.context.requested_pipeline_name,
            "rebuild": args.rebuild,
            "taskCount": task_count,
            "synced": true
        }))
    }

    pub fn history(args: TaskArgs) -> Result<Value, GateError> {
        let root = canonical_root(&args.context.root)?;
        let _lock = GateProjectLock::acquire_shared(&root)?;
        let task = TaskRepository::find_task(root.clone(), &args.task_id)?;
        let document = TaskRepository::read_task_document(&root, &task)?;
        Ok(Value::Array(
            document
                .records
                .into_iter()
                .map(|record| {
                    json!({
                        "stage": record.stage,
                        "event": record.outcome.map(|value| value.to_uppercase()).unwrap_or(record.record_kind),
                        "agent": record.agent,
                        "model": record.model,
                        "notes": record.notes,
                        "timestamp": record.recorded_at
                    })
                })
                .collect(),
        ))
    }

    pub fn task_body(args: TaskArgs) -> Result<Value, GateError> {
        let root = canonical_root(&args.context.root)?;
        let _lock = GateProjectLock::acquire_shared(&root)?;
        let task = TaskRepository::find_task(root.clone(), &args.task_id)?;
        let path = TaskRepository::canonical_task_markdown_path(&root, &task)?;
        Ok(json!({
            "taskId": task.task_id,
            "path": path,
            "markdown": fs::read_to_string(&path)?
        }))
    }

    pub fn append(args: StageNoteArgs) -> Result<Value, GateError> {
        let root = canonical_root(&args.context.root)?;
        let _project_lock = GateProjectLock::acquire_exclusive(&root)?;
        let _task_lock = TaskFileLock::acquire(&root, &args.task_id)?;
        let task = TaskRepository::find_task(root.clone(), &args.task_id)?;
        let (document, definition, state) = active_task_state(
            &root,
            &task,
            args.context.requested_pipeline_name.as_deref(),
        )?;
        if definition
            .find_state(&state.state)
            .ok_or_else(|| GateError::UnknownState(state.state.clone()))?
            .terminal
        {
            return Err(GateError::Cli(format!(
                "task '{}' is already terminal; append is not allowed on completed tasks",
                args.task_id
            )));
        }
        let acting_stage = StateMachine::acting_stage_name(&state.state, &definition)?;
        let section = CanonicalSectionFactory::build_append_section(
            &acting_stage,
            &args.agent,
            &args.notes,
            args.model.as_deref(),
        );
        TaskRepository::append_task_section(&root, &task, &section)?;
        status_for_document(&root, &task, &document.metadata.pipeline)
    }

    pub fn pass(args: StageNoteArgs) -> Result<Value, GateError> {
        apply_outcome(args, "pass")
    }

    pub fn fail(args: StageNoteArgs) -> Result<Value, GateError> {
        apply_outcome(args, "fail")
    }
}

fn create_task_markdown(
    task_id: &str,
    title: &str,
    pipeline: &str,
    priority: &str,
    sprint: u64,
    parent: Option<&str>,
    body: Option<&str>,
) -> String {
    let mut frontmatter = vec![
        "---".to_string(),
        format!("id: {}", yaml_string(task_id)),
        format!("title: {}", yaml_string(title)),
        format!("pipeline: {}", yaml_string(pipeline)),
        format!("priority: {}", yaml_string(priority)),
        format!("sprint: {sprint}"),
    ];
    if let Some(parent) = parent {
        frontmatter.push(format!("parent: {}", yaml_string(parent)));
    }
    frontmatter.push("---".to_string());

    let body = match body {
        Some(body) => normalize_body(body),
        None => default_task_body(),
    };
    format!("{}\n\n{body}", frontmatter.join("\n"))
}

fn normalize_body(body: &str) -> String {
    format!("{}\n", body.trim_end())
}

fn default_task_body() -> String {
    normalize_body("## Problem\n\n## Plan\n\n## Acceptance\n\n## References\n")
}

fn yaml_string(value: &str) -> String {
    serde_json::to_string(value).expect("serialize YAML-compatible JSON string")
}

fn error_status_row(root: &std::path::Path, task: &TaskPathSet, error: GateError) -> Value {
    let mut title = format!("[error] {}", task.task_id);
    let mut pipeline = "unknown".to_string();
    let mut canonical_task_id = task.task_id.clone();

    if let Ok(path) = TaskRepository::canonical_task_markdown_path(root, task)
        && let Ok(text) = fs::read_to_string(&path)
    {
        let lines = text.lines().collect::<Vec<_>>();
        if let Ok(metadata) = TaskMetadataParser::parse(&lines) {
            title = metadata.title;
            pipeline = metadata.pipeline;
            if let Some(id) = metadata.id {
                canonical_task_id = id;
            }
        } else if let Ok(metadata) = TaskMetadataParser::parse_lenient(&lines) {
            title = metadata.title;
            pipeline = metadata.pipeline;
            if let Some(id) = metadata.id {
                canonical_task_id = id;
            }
        }
    }

    json!({
        "taskId": task.task_id,
        "canonicalTaskId": canonical_task_id,
        "title": title,
        "pipeline": pipeline,
        "state": "error",
        "stages": [],
        "attempts": 0,
        "location": task.location,
        "path": task.path.display().to_string(),
        "linked": false,
        "issue": format!("{error}"),
    })
}

fn write_gate_db_snapshot(
    root: &std::path::Path,
    gate_dir: &std::path::Path,
    tasks: &[TaskPathSet],
) -> Result<(), GateError> {
    let rows = tasks
        .iter()
        .map(|task| match status_row(root, task, None) {
            Ok(row) => row,
            Err(error) => error_status_row(root, task, error),
        })
        .collect::<Vec<_>>();
    let snapshot = json!({
        "schemaVersion": 1,
        "source": "maenifold.gate markdown read model",
        "tasks": rows,
    });
    let db_path = gate_dir.join("gate.db");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&db_path)
        .map_err(|error| {
            GateError::Io(std::io::Error::new(
                error.kind(),
                format!(
                    "failed to open gate.db read model '{}': {error}",
                    db_path.display()
                ),
            ))
        })?;
    file.write_all(serde_json::to_string_pretty(&snapshot)?.as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
