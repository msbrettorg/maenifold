use std::collections::HashSet;
use std::path::{Path, PathBuf};

use pulldown_cmark::{Options, Parser, html};
use serde::Serialize;

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
    pub steps: Vec<BoardStep>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardStep {
    pub id: String,
    pub label: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardTaskDetail {
    pub task: BoardTaskSummary,
    pub markdown_html: String,
}

#[derive(Debug, Clone)]
pub struct GateBoardData;

impl GateBoardData {
    pub fn list(root: &Path) -> Result<Vec<BoardTaskSummary>, GateError> {
        TaskRepository::list_tasks(root.to_path_buf())?
            .into_iter()
            .map(|task| summarize_task(root, &task))
            .collect()
    }

    pub fn detail(root: &Path, task_id: &str) -> Result<BoardTaskDetail, GateError> {
        let task = TaskRepository::find_task(root.to_path_buf(), task_id)?;
        let summary = summarize_task(root, &task)?;
        let markdown = std::fs::read_to_string(&task.path)?;
        Ok(BoardTaskDetail {
            task: summary,
            markdown_html: render_markdown(&markdown),
        })
    }
}

fn summarize_task(root: &Path, task: &TaskPathSet) -> Result<BoardTaskSummary, GateError> {
    let document = TaskFileReader::read(&task.path)?;
    let definition = GateDefinitionCatalog::resolve_for_task(root, &document.metadata, None)?;
    let effective = TaskStatusDeriver::derive(&document, &definition, &task.location)?;
    let steps = build_steps(&definition, &effective.state)?;

    Ok(BoardTaskSummary {
        task_id: task.task_id.clone(),
        title: document.metadata.title,
        pipeline: document.metadata.pipeline,
        state: effective.state.clone(),
        location: effective.location,
        attempts: effective.attempts,
        path: task.path.clone(),
        phase_label: display_phase_label(&effective.state, &definition)?,
        steps,
    })
}

fn display_phase_label(state: &str, definition: &GateDefinition) -> Result<String, GateError> {
    let initial = definition.initial_state()?;
    if state == initial {
        Ok("Pending".to_string())
    } else {
        Ok(display_label(state))
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

fn render_markdown(markdown: &str) -> String {
    let mut html_output = String::new();
    let parser = Parser::new_ext(markdown, Options::all());
    html::push_html(&mut html_output, parser);
    html_output
}

