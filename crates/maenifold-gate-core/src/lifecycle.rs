use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::error::GateError;
use crate::gate_definition::GateDefinition;
use crate::section_factory::CanonicalSectionFactory;
use crate::state_machine::StateMachine;
use crate::status_filter::StatusFilter;
use crate::task_file::{GateProjectLock, TaskFileLock};
use crate::task_metadata::{TaskMetadata, is_linked_session_id};
use crate::task_repository::{TaskPathSet, TaskRepository};
use crate::task_status::TaskStatusDeriver;

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
pub struct StageNoteArgs {
    pub context: GateContext,
    pub task_id: String,
    pub agent: String,
    pub notes: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GateLifecycle;

impl GateLifecycle {
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
                Err(e) => json!({
                    "taskId": task.task_id,
                    "canonicalTaskId": task.task_id,
                    "title": format!("[error] {}", task.task_id),
                    "pipeline": "unknown",
                    "state": "error",
                    "stages": [],
                    "attempts": 0,
                    "location": task.location,
                    "path": task.path.display().to_string(),
                    "linked": false,
                    "issue": format!("{e}"),
                }),
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
        Ok(json!({
            "root": root,
            "pipeline": args.context.requested_pipeline_name,
            "rebuild": args.rebuild,
            "taskCount": TaskRepository::list_tasks(root)?.len(),
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

pub fn resolve_definition(root: &Path, pipeline: &str) -> Result<GateDefinition, GateError> {
    if let Some(text) = builtin_definition_text(pipeline) {
        return GateDefinition::from_json_text(text);
    }
    let path = root.join("pipeline.json");
    let text = fs::read_to_string(&path).map_err(|error| {
        GateError::Cli(format!(
            "unknown gate pipeline '{pipeline}' and failed to read custom definition '{}': {error}",
            path.display()
        ))
    })?;
    GateDefinition::from_json_text(&text)
}

fn apply_outcome(args: StageNoteArgs, outcome: &str) -> Result<Value, GateError> {
    let root = canonical_root(&args.context.root)?;
    let _project_lock = GateProjectLock::acquire_exclusive(&root)?;
    let _task_lock = TaskFileLock::acquire(&root, &args.task_id)?;
    let task = TaskRepository::find_task(root.clone(), &args.task_id)?;
    let (_document, definition, state) = active_task_state(
        &root,
        &task,
        args.context.requested_pipeline_name.as_deref(),
    )?;
    let transition = match outcome {
        "pass" => StateMachine::next_on_pass(&state.state, &definition)?,
        "fail" => StateMachine::next_on_fail(&state.state, &definition)?,
        _ => unreachable!(),
    };
    let section = CanonicalSectionFactory::build_outcome_section(
        &transition.acting_stage,
        outcome,
        &args.agent,
        &args.notes,
        args.model.as_deref(),
    );
    TaskRepository::append_task_section(&root, &task, &section)?;
    if definition
        .find_state(&transition.next_state)
        .ok_or_else(|| GateError::UnknownState(transition.next_state.clone()))?
        .terminal
    {
        TaskRepository::move_to_done(&root, &task)?;
    }
    let refreshed = TaskRepository::find_task(root.clone(), &args.task_id)?;
    status_row(
        &root,
        &refreshed,
        args.context.requested_pipeline_name.as_deref(),
    )
}

fn active_task_state(
    root: &Path,
    task: &TaskPathSet,
    requested_pipeline: Option<&str>,
) -> Result<
    (
        crate::task_file::TaskDocument,
        GateDefinition,
        crate::task_status::EffectiveTaskState,
    ),
    GateError,
> {
    let document = TaskRepository::read_task_document(root, task)?;
    let definition = resolve_definition_for_task(root, &document.metadata, requested_pipeline)?;
    let state = TaskStatusDeriver::derive(&document, &definition, &task.location)?;
    Ok((document, definition, state))
}

fn status_for_document(
    root: &Path,
    task: &TaskPathSet,
    pipeline: &str,
) -> Result<Value, GateError> {
    status_row(root, task, Some(pipeline))
}

fn status_row(
    root: &Path,
    task: &TaskPathSet,
    requested_pipeline: Option<&str>,
) -> Result<Value, GateError> {
    let document = TaskRepository::read_task_document(root, task)?;
    let definition = resolve_definition_for_task(root, &document.metadata, requested_pipeline)?;
    let effective = TaskStatusDeriver::derive(&document, &definition, &task.location)?;
    Ok(json!({
        "taskId": task.task_id,
        "canonicalTaskId": document.metadata.id,
        "title": document.metadata.title,
        "pipeline": document.metadata.pipeline,
        "state": effective.state,
        "stages": definition.ordered_stage_ids()?,
        "attempts": effective.attempts,
        "location": task.location,
        "path": task.path,
        "linked": is_linked_session_id(document.metadata.session_id.as_deref()),
        "issue": ""
    }))
}

fn resolve_definition_for_task(
    root: &Path,
    metadata: &TaskMetadata,
    requested_pipeline: Option<&str>,
) -> Result<GateDefinition, GateError> {
    if let Some(requested) = requested_pipeline
        && requested != metadata.pipeline
    {
        return Err(GateError::Cli(format!(
            "task pipeline binding '{}' does not match requested pipeline '{}'",
            metadata.pipeline, requested
        )));
    }
    resolve_definition(root, &metadata.pipeline)
}

fn canonical_root(root: &Path) -> Result<PathBuf, GateError> {
    if !root.is_absolute() {
        return Err(GateError::Cli(
            "gate root must be an absolute path containing todo/ and done/".to_string(),
        ));
    }
    Ok(fs::canonicalize(root)?)
}

fn builtin_definition_text(name: &str) -> Option<&'static str> {
    Some(match name {
        "sdd" => {
            r#"{"schemaVersion":1,"id":"sdd","name":"Spec Driven Development","initial":"pending","states":{"pending":{"on":{"pass":"swe"}},"swe":{"on":{"fail":"pending","pass":"red-team"}},"red-team":{"on":{"fail":"pending","pass":"blue-team"}},"blue-team":{"on":{"fail":"pending","pass":"done"}},"done":{"terminal":true}}}"#
        }
        "tdd" => {
            r#"{"schemaVersion":1,"id":"tdd","name":"Test Driven Development","initial":"pending","states":{"pending":{"on":{"pass":"blue-team"}},"blue-team":{"on":{"fail":"pending","pass":"swe"}},"swe":{"on":{"fail":"pending","pass":"red-team"}},"red-team":{"on":{"fail":"pending","pass":"done"}},"done":{"terminal":true}}}"#
        }
        "gsd" => {
            r#"{"schemaVersion":1,"id":"gsd","name":"Get Shit Done","initial":"pending","states":{"pending":{"on":{"pass":"research"}},"research":{"on":{"fail":"pending","pass":"plan"}},"plan":{"on":{"fail":"pending","pass":"execute"}},"execute":{"on":{"fail":"pending","pass":"verify"}},"verify":{"on":{"fail":"execute","pass":"done"}},"done":{"terminal":true}}}"#
        }
        "lightweight" => {
            r#"{"schemaVersion":1,"id":"lightweight","name":"Lightweight","initial":"pending","states":{"pending":{"on":{"pass":"swe"}},"swe":{"on":{"fail":"pending","pass":"blue-team"}},"blue-team":{"on":{"fail":"pending","pass":"done"}},"done":{"terminal":true}}}"#
        }
        "mdd" => {
            r#"{"schemaVersion":1,"id":"mdd","name":"Model Driven Development","initial":"pending","states":{"pending":{"on":{"pass":"opus-plan"}},"opus-plan":{"on":{"fail":"pending","pass":"goldeneye-plan-review"}},"goldeneye-plan-review":{"on":{"fail":"opus-plan","pass":"gpt-5.4-test-writing"}},"gpt-5.4-test-writing":{"on":{"fail":"opus-plan","pass":"gpt-5.4-implementation"}},"gpt-5.4-implementation":{"on":{"fail":"gpt-5.4-test-writing","pass":"gpt-5.4-test-execution"}},"gpt-5.4-test-execution":{"on":{"fail":"gpt-5.4-implementation","pass":"goldeneye-work-review"}},"goldeneye-work-review":{"on":{"fail":"gpt-5.4-implementation","pass":"opus-acceptance"}},"opus-acceptance":{"on":{"fail":"gpt-5.4-implementation","pass":"done"}},"done":{"terminal":true}}}"#
        }
        "slc" => {
            r#"{"schemaVersion":1,"id":"slc","name":"Simple, Lovable, Complete","initial":"pending","states":{"pending":{"on":{"pass":"discovery"}},"discovery":{"on":{"fail":"pending","pass":"spec"}},"spec":{"on":{"fail":"discovery","pass":"swe"}},"swe":{"on":{"fail":"spec","pass":"red-team"}},"red-team":{"on":{"fail":"remediate","pass":"blue-team"}},"remediate":{"on":{"fail":"spec","pass":"red-team"}},"blue-team":{"on":{"fail":"remediate","pass":"done"}},"done":{"terminal":true}}}"#
        }
        "sdd-c" => {
            r#"{"schemaVersion":1,"id":"sdd-c","name":"Spec Driven Development (with Confessions)","initial":"pending","states":{"pending":{"on":{"pass":"swe"}},"swe":{"on":{"fail":"pending","pass":"swe-confess"}},"swe-confess":{"on":{"fail":"swe","pass":"red-team"}},"red-team":{"on":{"fail":"pending","pass":"red-team-confess"}},"red-team-confess":{"on":{"fail":"red-team","pass":"blue-team"}},"blue-team":{"on":{"fail":"pending","pass":"blue-team-confess"}},"blue-team-confess":{"on":{"fail":"blue-team","pass":"pm-confess"}},"pm-confess":{"on":{"fail":"pending","pass":"red-team-audit"}},"red-team-audit":{"on":{"fail":"pending","pass":"done"}},"done":{"terminal":true}}}"#
        }
        "tdd-c" => {
            r#"{"schemaVersion":1,"id":"tdd-c","name":"Test Driven Development (with Confessions)","initial":"pending","states":{"pending":{"on":{"pass":"blue-team"}},"blue-team":{"on":{"fail":"pending","pass":"blue-team-confess"}},"blue-team-confess":{"on":{"fail":"blue-team","pass":"swe"}},"swe":{"on":{"fail":"pending","pass":"swe-confess"}},"swe-confess":{"on":{"fail":"swe","pass":"red-team"}},"red-team":{"on":{"fail":"pending","pass":"red-team-confess"}},"red-team-confess":{"on":{"fail":"red-team","pass":"pm-confess"}},"pm-confess":{"on":{"fail":"pending","pass":"red-team-audit"}},"red-team-audit":{"on":{"fail":"pending","pass":"done"}},"done":{"terminal":true}}}"#
        }
        "test" => {
            r#"{"schemaVersion":1,"id":"test","name":"Test","initial":"pending","states":{"pending":{"on":{"pass":"done"}},"done":{"terminal":true}}}"#
        }
        _ => return None,
    })
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
