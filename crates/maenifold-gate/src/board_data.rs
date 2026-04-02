use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::GateError;
use crate::gate_catalog::GateDefinitionCatalog;
use crate::gate_definition::GateDefinition;
use crate::stage_labels::display_label;
use crate::task_file::TaskFileReader;
use crate::task_repository::{TaskPathSet, TaskRepository};
use crate::task_status::TaskStatusDeriver;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardTaskSummary {
    pub task_id: String,
    pub title: String,
    pub pipeline: String,
    pub state: String,
    pub location: String,
    pub attempts: u32,
    pub path: PathBuf,
    pub phase_label: String,
    pub issue: String,
    pub steps: Vec<BoardStep>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardStep {
    pub id: String,
    pub label: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct BoardTaskDetail {
    pub task: BoardTaskSummary,
    pub markdown: String,
}

#[derive(Debug, Clone)]
pub struct GateBoardData;

#[derive(Debug, Clone, Default, Deserialize)]
struct FrontmatterTask {
    title: Option<String>,
    status: Option<String>,
    pipeline: Option<String>,
}

impl GateBoardData {
    pub fn list(root: &Path) -> Result<Vec<BoardTaskSummary>, GateError> {
        let mut tasks: Vec<_> = TaskRepository::list_tasks(root.to_path_buf())?
            .into_iter()
            .map(|task| summarize_task(root, &task))
            .collect();
        tasks.sort_by(|left, right| {
            left.location
                .cmp(&right.location)
                .then(left.task_id.cmp(&right.task_id))
        });
        Ok(tasks)
    }

    pub fn detail(root: &Path, task_id: &str) -> Result<BoardTaskDetail, GateError> {
        let task = TaskRepository::find_task(root.to_path_buf(), task_id)?;
        let summary = summarize_task(root, &task);
        let markdown = std::fs::read_to_string(&task.path)?;
        Ok(BoardTaskDetail {
            task: summary,
            markdown,
        })
    }
}

fn summarize_task(root: &Path, task: &TaskPathSet) -> BoardTaskSummary {
    match std::fs::read_to_string(&task.path) {
        Ok(markdown) => summarize_markdown(root, task, &markdown),
        Err(err) => fallback_summary(
            task,
            "Unreadable".to_string(),
            "unreadable".to_string(),
            format!("Failed to read task: {err}"),
        ),
    }
}

fn summarize_markdown(root: &Path, task: &TaskPathSet, markdown: &str) -> BoardTaskSummary {
    if let Some(frontmatter) = parse_frontmatter(markdown) {
        return summarize_frontmatter_task(root, task, markdown, frontmatter);
    }

    match summarize_current_task(root, task) {
        Ok(summary) => summary,
        Err(err) => fallback_summary(
            task,
            extract_markdown_title(markdown)
                .unwrap_or_else(|| task.task_id.clone()),
            extract_labeled_pipeline(markdown).unwrap_or_else(|| "Unbound".to_string()),
            err.to_string(),
        ),
    }
}

fn summarize_frontmatter_task(
    root: &Path,
    task: &TaskPathSet,
    markdown: &str,
    frontmatter: FrontmatterTask,
) -> BoardTaskSummary {
    let title = frontmatter
        .title
        .or_else(|| extract_markdown_title(markdown))
        .unwrap_or_else(|| task.task_id.clone());
    let state = frontmatter
        .status
        .unwrap_or_else(|| default_state_for_location(&task.location).to_string());
    let pipeline = frontmatter.pipeline.unwrap_or_else(|| "Unbound".to_string());
    let issue = if pipeline == "Unbound" {
        Some("Missing pipeline in frontmatter".to_string())
    } else {
        None
    };

    build_summary(
        root,
        task,
        title,
        pipeline,
        state,
        0,
        issue,
    )
}

fn summarize_current_task(root: &Path, task: &TaskPathSet) -> Result<BoardTaskSummary, GateError> {
    let document = TaskFileReader::read(&task.path)?;
    let definition = GateDefinitionCatalog::resolve_for_task(root, &document.metadata, None)?;
    let effective = TaskStatusDeriver::derive(&document, &definition, &task.location)?;

    Ok(build_summary(
        root,
        task,
        document.metadata.title,
        document.metadata.pipeline,
        effective.state,
        effective.attempts,
        None,
    ))
}

fn build_summary(
    root: &Path,
    task: &TaskPathSet,
    title: String,
    pipeline: String,
    state: String,
    attempts: u32,
    initial_issue: Option<String>,
) -> BoardTaskSummary {
    let mut issue = initial_issue.unwrap_or_default();
    let mut steps = Vec::new();
    let mut phase_label = display_phase_label(&state);

    if pipeline != "Unbound" {
        match GateDefinitionCatalog::resolve(root, &pipeline) {
            Ok(definition) => match build_steps(&definition, &state) {
                Ok(result) => steps = result,
                Err(err) => {
                    issue = merge_issue(&issue, err.to_string());
                    phase_label = display_phase_label(&state);
                }
            },
            Err(err) => {
                issue = merge_issue(&issue, err.to_string());
            }
        }
    }

    BoardTaskSummary {
        task_id: task.task_id.clone(),
        title,
        pipeline,
        state: state.clone(),
        location: task.location.clone(),
        attempts,
        path: task.path.clone(),
        phase_label,
        issue,
        steps,
    }
}

fn fallback_summary(
    task: &TaskPathSet,
    title: String,
    phase_label: String,
    issue: String,
) -> BoardTaskSummary {
    BoardTaskSummary {
        task_id: task.task_id.clone(),
        title,
        pipeline: "Unbound".to_string(),
        state: phase_label.to_ascii_lowercase(),
        location: task.location.clone(),
        attempts: 0,
        path: task.path.clone(),
        phase_label,
        issue,
        steps: Vec::new(),
    }
}

fn build_steps(definition: &GateDefinition, state: &str) -> Result<Vec<BoardStep>, GateError> {
    let sequence = pass_sequence(definition)?;
    let initial = definition.initial_state()?;
    let current_index = if state == initial {
        0
    } else {
        sequence
            .iter()
            .position(|step| step == state)
            .ok_or_else(|| GateError::UnknownState(state.to_string()))?
    };

    Ok(sequence
        .into_iter()
        .enumerate()
        .map(|(index, id)| BoardStep {
            label: display_label(&id),
            status: if index < current_index {
                "complete".to_string()
            } else if index == current_index {
                "current".to_string()
            } else {
                "upcoming".to_string()
            },
            id,
        })
        .collect())
}

fn pass_sequence(definition: &GateDefinition) -> Result<Vec<String>, GateError> {
    let initial = definition.initial_state()?;
    let mut current = definition
        .find_state(initial)
        .and_then(|state| state.on.get("pass"))
        .cloned()
        .ok_or_else(|| {
            GateError::TaskMetadataParse(format!(
                "initial state '{initial}' must declare an on.pass transition"
            ))
        })?;
    let mut seen = HashSet::new();
    let mut sequence = Vec::new();

    loop {
        if !seen.insert(current.clone()) {
            return Err(GateError::TaskMetadataParse(
                "pipeline pass path must not cycle".to_string(),
            ));
        }

        let gate_state = definition
            .find_state(&current)
            .ok_or_else(|| GateError::UnknownState(current.clone()))?;
        sequence.push(current.clone());

        if gate_state.terminal {
            break;
        }

        current = gate_state.on.get("pass").cloned().ok_or_else(|| {
            GateError::TaskMetadataParse(format!(
                "state '{}' must declare an on.pass transition for board rendering",
                current
            ))
        })?;
    }

    Ok(sequence)
}

fn parse_frontmatter(markdown: &str) -> Option<FrontmatterTask> {
    if !markdown.starts_with("---\n") {
        return None;
    }
    let rest = &markdown[4..];
    let end = rest.find("\n---")?;
    serde_yaml::from_str::<FrontmatterTask>(&rest[..end]).ok()
}

fn extract_markdown_title(markdown: &str) -> Option<String> {
    markdown
        .lines()
        .find_map(|line| line.strip_prefix("# ").map(|value| value.trim().to_string()))
}

fn extract_labeled_pipeline(markdown: &str) -> Option<String> {
    markdown.lines().find_map(|line| {
        line.strip_prefix("**Pipeline**:")
            .map(|value| value.trim().to_string())
    })
}

fn display_phase_label(state: &str) -> String {
    if state == "pending" {
        "Pending".to_string()
    } else {
        display_label(state)
    }
}

fn default_state_for_location(location: &str) -> &'static str {
    if location == "done" { "done" } else { "pending" }
}

fn merge_issue(existing: &str, next: String) -> String {
    if existing.is_empty() {
        next
    } else {
        format!("{existing} | {next}")
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use sha2::{Digest, Sha256};
    use tempfile::TempDir;

    use super::GateBoardData;

    #[test]
    fn list_supports_frontmatter_tasks_used_in_repo() {
        let root = temp_root();
        fs::write(
            root.path().join("todo").join("T-1.1.md"),
            r#"---
id: T-1.1
title: Example Frontmatter Task
status: pending
pipeline: sdd
---

# T-1.1: Example Frontmatter Task
"#,
        )
        .expect("write frontmatter task");

        let tasks = GateBoardData::list(root.path()).expect("list");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].task_id, "T-1.1");
        assert_eq!(tasks[0].pipeline, "sdd");
        assert_eq!(tasks[0].phase_label, "Pending");
        assert!(!tasks[0].steps.is_empty());
    }

    #[test]
    fn list_keeps_tasks_visible_when_pipeline_is_missing() {
        let root = temp_root();
        fs::write(
            root.path().join("todo").join("T-1.2.md"),
            r#"---
id: T-1.2
title: Missing Pipeline
status: pending
---

# T-1.2: Missing Pipeline
"#,
        )
        .expect("write task");

        let tasks = GateBoardData::list(root.path()).expect("list");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].pipeline, "Unbound");
        assert!(tasks[0].issue.contains("Missing pipeline"));
    }

    #[test]
    fn list_supports_current_gate_markdown_tasks() {
        let root = temp_root();
        let digest = authoritative_digest("sdd");
        fs::write(
            root.path().join("todo").join("T-1.3.md"),
            format!(
                "# T-1.3 — Current Gate Task\n\n**Pipeline**: sdd\n**Pipeline Digest**: {digest}\n\n## Acceptance Criteria\n\n1. Example\n"
            ),
        )
        .expect("write current task");

        let tasks = GateBoardData::list(root.path()).expect("list");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].pipeline, "sdd");
        assert_eq!(tasks[0].phase_label, "Pending");
    }

    fn temp_root() -> TempDir {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("todo")).expect("todo");
        fs::create_dir_all(temp.path().join("done")).expect("done");
        temp
    }

    fn authoritative_digest(name: &str) -> String {
        let bytes = fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join(format!("{name}.json")),
        )
        .expect("read asset");
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        format!("{:x}", hasher.finalize())
    }
}
