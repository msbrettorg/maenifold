use serde_json::json;

use super::{CREATE_TASK_ID_RE, CreateTaskArgs, GateContext, SyncArgs, resolve_definition};
use crate::error::GateError;
use crate::lifecycle::GateLifecycle;
use crate::task_file::{GateProjectLock, TaskFileLock};
use crate::task_repository::TaskRepository;
use crate::task_title_slug;

pub(super) fn create_task(args: CreateTaskArgs) -> Result<serde_json::Value, GateError> {
    let requested_root = args.context.root.clone();
    let root = super::lifecycle_helpers::canonical_root(&args.context.root)?;

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
            .map_err(|error| GateError::Cli(format!("failed to parse taskId sprint: {error}")))?,
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
    let _ = GateLifecycle::sync(SyncArgs {
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
