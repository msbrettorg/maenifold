use crate::error::{MaenifoldError, Result};
use crate::markdown::{Frontmatter, append_h2_section, file_exists, read_markdown, write_markdown};
use crate::sessions::{display_utc_now, frontmatter_string, workflow_session_uri};
use crate::thinking::sequential_session_uri;
use crate::tools::ToolContext;
use crate::workflow::{build_workflow_response_content, read_workflow_definition, workflow_steps};
use serde_yaml::Value as YamlValue;
use std::fs;
use std::path::{Path, PathBuf};

use super::{frontmatter_usize, read_workflow_queue};

// T-12.0: RTM NFR-1 keeps submachine registration and waiting/resume transitions isolated from queue helpers.

pub(crate) enum ResumeSubmachineResult {
    Ready(Frontmatter),
    Waiting(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HostSessionType {
    Workflow,
    Sequential,
}

impl HostSessionType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Workflow => "workflow",
            Self::Sequential => "sequential",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::Workflow => "Workflow",
            Self::Sequential => "SequentialThinking session",
        }
    }

    fn link(self, session_id: &str) -> String {
        format!("[[{}{}{}]]", self.as_str(), "/", session_id)
    }

    fn session_uri(self, session_id: &str) -> String {
        match self {
            Self::Workflow => workflow_session_uri(session_id),
            Self::Sequential => sequential_session_uri(session_id),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SubmachineType {
    Sequential,
    Workflow,
    Gate,
}

impl SubmachineType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Sequential => "sequential",
            Self::Workflow => "workflow",
            Self::Gate => "gate",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::Sequential => "SequentialThinking session",
            Self::Workflow => "Workflow session",
            Self::Gate => "gate task",
        }
    }

    fn as_session_kind(self) -> Option<HostSessionType> {
        match self {
            Self::Sequential => Some(HostSessionType::Sequential),
            Self::Workflow => Some(HostSessionType::Workflow),
            Self::Gate => None,
        }
    }
}

struct GateTaskStatus {
    path: PathBuf,
    effective_state: String,
    terminal: bool,
}

struct ResolvedSubmachineState {
    terminal: bool,
    target_description: String,
    target_reference: Option<String>,
}

pub(crate) fn parse_submachine_type(value: Option<&str>) -> Result<SubmachineType> {
    let normalized = value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("sequential")
        .to_ascii_lowercase();

    match normalized.as_str() {
        "sequential" => Ok(SubmachineType::Sequential),
        "workflow" => Ok(SubmachineType::Workflow),
        "gate" => Ok(SubmachineType::Gate),
        _ => Err(MaenifoldError::InvalidArgument(format!(
            "submachineType must be one of sequential, workflow, or gate. Got: {normalized}"
        ))),
    }
}

#[allow(clippy::too_many_lines)]
pub(crate) fn maybe_resume_submachine(
    context: &ToolContext,
    host_type: HostSessionType,
    workflow_uri: &str,
    mut frontmatter: Frontmatter,
    existing_content: &str,
) -> Result<ResumeSubmachineResult> {
    // T-12.0: RTM FR-10 - resume only sessions that are marked waiting and still reference an active submachine.
    if frontmatter_string(&frontmatter, "phase") != "waiting" {
        return Ok(ResumeSubmachineResult::Ready(frontmatter));
    }

    let active_submachine_session_id =
        frontmatter_string(&frontmatter, "activeSubmachineSessionId");
    let active_submachine_type = frontmatter_string(&frontmatter, "activeSubmachineType");
    let active_submachine_target = frontmatter_string(&frontmatter, "activeSubmachineTarget");

    if active_submachine_session_id.is_empty() || active_submachine_type.is_empty() {
        return Ok(ResumeSubmachineResult::Ready(frontmatter));
    }

    let submachine_type = parse_submachine_type(Some(&active_submachine_type))?;
    let Some(resolved_state) = resolved_terminal_state(
        context,
        submachine_type,
        &active_submachine_session_id,
        (!active_submachine_target.is_empty()).then_some(active_submachine_target.as_str()),
    )?
    else {
        clear_submachine_frontmatter(&mut frontmatter);
        write_markdown(
            workflow_uri,
            Some(&frontmatter),
            existing_content,
            &context.config,
        )?;
        append_h2_section(
            workflow_uri,
            "Submachine Cleared",
            &format!(
                "Registered {} '{}' not found. Clearing waiting state and resuming {}.\n\n*{}*",
                submachine_type.display_name(),
                active_submachine_session_id,
                host_type.display_name(),
                display_utc_now(),
            ),
            &context.config,
        )?;
        let (updated_frontmatter, _, _) = read_markdown(workflow_uri, &context.config)?;
        return Ok(ResumeSubmachineResult::Ready(
            updated_frontmatter.unwrap_or(frontmatter),
        ));
    };

    if resolved_state.terminal {
        clear_submachine_frontmatter(&mut frontmatter);
        let (_, content_after_clear, _) = read_markdown(workflow_uri, &context.config)?;
        write_markdown(
            workflow_uri,
            Some(&frontmatter),
            &content_after_clear,
            &context.config,
        )?;
        append_h2_section(
            workflow_uri,
            "Submachine Resumed",
            &format!(
                "{} '{}' reached terminal state '{}'. Resuming {}.\n\n*{}*",
                submachine_type.display_name(),
                active_submachine_session_id,
                resolved_state.target_description,
                host_type.display_name(),
                display_utc_now(),
            ),
            &context.config,
        )?;
        let (updated_frontmatter, _, _) = read_markdown(workflow_uri, &context.config)?;
        return Ok(ResumeSubmachineResult::Ready(
            updated_frontmatter.unwrap_or(frontmatter),
        ));
    }

    Ok(ResumeSubmachineResult::Waiting(format!(
        "{} is waiting on submachine {active_submachine_type} {active_submachine_session_id}. Complete it first, then call {} again.",
        host_type.display_name(),
        host_type.display_name(),
    )))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn register_workflow_submachine(
    context: &ToolContext,
    workflow_session_id: &str,
    workflow_uri: &str,
    mut frontmatter: Frontmatter,
    response: &str,
    thoughts: Option<&str>,
    submachine_session_id: &str,
    submachine_type: SubmachineType,
) -> Result<String> {
    let current_workflow_index = frontmatter_usize(&frontmatter, "currentWorkflow");
    let current_step = frontmatter_usize(&frontmatter, "currentStep");
    let queue = read_workflow_queue(&frontmatter);
    let current_workflow_id = queue
        .get(current_workflow_index)
        .ok_or_else(|| MaenifoldError::Message("Workflow queue is empty".to_string()))?;
    let workflow_value = read_workflow_definition(&context.config, current_workflow_id)?;
    let steps = workflow_steps(&workflow_value, current_workflow_id)?;

    append_h2_section(
        workflow_uri,
        &format!("Step {}/{} Response", current_step + 1, steps.len()),
        &build_workflow_response_content(response, thoughts),
        &context.config,
    )?;

    register_host_submachine(
        context,
        HostSessionType::Workflow,
        workflow_session_id,
        workflow_uri,
        &mut frontmatter,
        submachine_session_id,
        submachine_type,
    )
}

pub(crate) fn register_host_submachine(
    context: &ToolContext,
    host_type: HostSessionType,
    host_session_id: &str,
    host_uri: &str,
    frontmatter: &mut Frontmatter,
    submachine_session_id: &str,
    submachine_type: SubmachineType,
) -> Result<String> {
    let Some(resolved_state) =
        resolved_terminal_state(context, submachine_type, submachine_session_id, None)?
    else {
        return Ok(format!(
            "ERROR: {} '{}' not found under type '{}'. Ensure the child exists before registering.",
            submachine_type.display_name(),
            submachine_session_id,
            submachine_type.as_str()
        ));
    };

    validate_submachine_registration(
        context,
        host_type,
        host_session_id,
        submachine_type,
        submachine_session_id,
    )?;

    if let Some(child_kind) = submachine_type.as_session_kind() {
        link_child_to_parent(
            context,
            child_kind,
            submachine_session_id,
            host_type,
            host_session_id,
        )?;
        push_related_link(frontmatter, &child_kind.link(submachine_session_id));
    }

    // T-HSM-001.1b: RTM FR-10.1 - register submachine lifecycle state in workflow frontmatter.
    // T-HSM-001.1c: RTM FR-10.4 - do not advance currentStep while waiting on the submachine.
    frontmatter.insert(
        "phase".to_string(),
        YamlValue::String("waiting".to_string()),
    );
    frontmatter.insert(
        "activeSubmachineType".to_string(),
        YamlValue::String(submachine_type.as_str().to_string()),
    );
    frontmatter.insert(
        "activeSubmachineSessionId".to_string(),
        YamlValue::String(submachine_session_id.to_string()),
    );
    frontmatter.insert(
        "activeSubmachineTarget".to_string(),
        YamlValue::String(resolved_state.target_reference.unwrap_or_default()),
    );
    let (_, content_before_register, _) = read_markdown(host_uri, &context.config)?;
    write_markdown(
        host_uri,
        Some(frontmatter),
        &content_before_register,
        &context.config,
    )?;
    append_h2_section(
        host_uri,
        "Submachine Registered",
        &format!(
            "Submachine registered: {} {submachine_session_id}\n\n*{}*",
            submachine_type.as_str(),
            display_utc_now(),
        ),
        &context.config,
    )?;

    Ok(format!(
        "Registered submachine {} {submachine_session_id}. {} is waiting. Complete the submachine, then call {} again to continue.",
        submachine_type.as_str(),
        host_type.display_name(),
        host_type.display_name(),
    ))
}

pub(crate) fn clear_submachine_frontmatter(frontmatter: &mut Frontmatter) {
    frontmatter.insert(
        "phase".to_string(),
        YamlValue::String("running".to_string()),
    );
    frontmatter.insert(
        "activeSubmachineType".to_string(),
        YamlValue::String(String::new()),
    );
    frontmatter.insert(
        "activeSubmachineSessionId".to_string(),
        YamlValue::String(String::new()),
    );
    frontmatter.insert(
        "activeSubmachineTarget".to_string(),
        YamlValue::String(String::new()),
    );
}

fn push_related_link(frontmatter: &mut Frontmatter, related_link: &str) {
    let mut related = match frontmatter.remove("related") {
        Some(YamlValue::Sequence(values)) => values,
        Some(YamlValue::String(value)) if !value.trim().is_empty() => {
            vec![YamlValue::String(value)]
        }
        _ => Vec::new(),
    };
    if !related
        .iter()
        .filter_map(YamlValue::as_str)
        .any(|existing| existing == related_link)
    {
        related.push(YamlValue::String(related_link.to_string()));
    }
    frontmatter.insert("related".to_string(), YamlValue::Sequence(related));
}

fn validate_submachine_registration(
    context: &ToolContext,
    host_type: HostSessionType,
    host_session_id: &str,
    submachine_type: SubmachineType,
    submachine_session_id: &str,
) -> Result<()> {
    let Some(child_kind) = submachine_type.as_session_kind() else {
        return Ok(());
    };

    if child_kind == host_type && submachine_session_id == host_session_id {
        return Err(MaenifoldError::InvalidArgument(format!(
            "{} cannot register itself as a {} submachine",
            host_type.display_name(),
            submachine_type.as_str()
        )));
    }

    let child_link = child_kind.link(submachine_session_id);
    let host_link = host_type.link(host_session_id);
    if session_ancestry_contains(context, host_type, host_session_id, &child_link)? {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Registering {} '{}' under {} '{}' would create a cycle",
            submachine_type.as_str(),
            submachine_session_id,
            host_type.as_str(),
            host_session_id
        )));
    }

    if let Some(existing_parent) = session_parent_link(context, child_kind, submachine_session_id)?
    {
        if existing_parent != host_link {
            return Err(MaenifoldError::InvalidArgument(format!(
                "{} '{}' is already linked to parent {}",
                submachine_type.as_str(),
                submachine_session_id,
                existing_parent
            )));
        }
    }

    Ok(())
}

fn link_child_to_parent(
    context: &ToolContext,
    child_kind: HostSessionType,
    child_session_id: &str,
    host_type: HostSessionType,
    host_session_id: &str,
) -> Result<()> {
    let child_uri = child_kind.session_uri(child_session_id);
    let (child_frontmatter, child_content, _) = read_markdown(&child_uri, &context.config)?;
    let mut child_frontmatter = child_frontmatter.unwrap_or_default();
    let expected_parent = host_type.link(host_session_id);
    if frontmatter_string(&child_frontmatter, "parent").is_empty() {
        child_frontmatter.insert("parent".to_string(), YamlValue::String(expected_parent));
        write_markdown(
            &child_uri,
            Some(&child_frontmatter),
            &child_content,
            &context.config,
        )?;
    }
    Ok(())
}

fn session_parent_link(
    context: &ToolContext,
    session_type: HostSessionType,
    session_id: &str,
) -> Result<Option<String>> {
    let uri = session_type.session_uri(session_id);
    let (frontmatter, _, _) = read_markdown(&uri, &context.config)?;
    let parent = frontmatter
        .as_ref()
        .map(|values| frontmatter_string(values, "parent"))
        .filter(|value| !value.is_empty());
    Ok(parent)
}

fn session_ancestry_contains(
    context: &ToolContext,
    session_type: HostSessionType,
    session_id: &str,
    expected_ancestor_link: &str,
) -> Result<bool> {
    let mut current = session_parent_link(context, session_type, session_id)?;
    while let Some(parent_link) = current {
        if parent_link == expected_ancestor_link {
            return Ok(true);
        }
        let Some((parent_type, parent_session_id)) = parse_session_link(&parent_link) else {
            return Ok(false);
        };
        current = session_parent_link(context, parent_type, &parent_session_id)?;
    }
    Ok(false)
}

fn parse_session_link(value: &str) -> Option<(HostSessionType, String)> {
    let normalized = value.trim().strip_prefix("[[")?.strip_suffix("]]")?;
    let (session_type, session_id) = normalized.split_once('/')?;
    let session_type = match session_type {
        "workflow" => HostSessionType::Workflow,
        "sequential" => HostSessionType::Sequential,
        _ => return None,
    };
    Some((session_type, session_id.to_string()))
}

fn resolved_terminal_state(
    context: &ToolContext,
    submachine_type: SubmachineType,
    submachine_session_id: &str,
    persisted_target: Option<&str>,
) -> Result<Option<ResolvedSubmachineState>> {
    match submachine_type {
        SubmachineType::Sequential | SubmachineType::Workflow => {
            let session_uri = submachine_type
                .as_session_kind()
                .expect("non-gate session kinds are supported")
                .session_uri(submachine_session_id);
            if !file_exists(&session_uri, &context.config)? {
                return Ok(None);
            }
            let (frontmatter, _, _) = read_markdown(&session_uri, &context.config)?;
            let status = frontmatter
                .as_ref()
                .map(|values| frontmatter_string(values, "status"))
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "active".to_string());
            let terminal = matches!(status.as_str(), "completed" | "cancelled" | "abandoned");
            Ok(Some(ResolvedSubmachineState {
                terminal,
                target_description: status,
                target_reference: Some(session_uri),
            }))
        }
        SubmachineType::Gate => Ok(find_gate_task_status(
            context,
            submachine_session_id,
            persisted_target,
        )?
        .map(|task| ResolvedSubmachineState {
            terminal: task.terminal,
            target_description: task.effective_state,
            target_reference: Some(task.path.to_string_lossy().into_owned()),
        })),
    }
}

fn find_gate_task_status(
    context: &ToolContext,
    task_id: &str,
    persisted_target: Option<&str>,
) -> Result<Option<GateTaskStatus>> {
    if let Some(path) = persisted_target
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
    {
        return resolve_registered_gate_task_status(&path);
    }

    for candidate_root in gate_root_candidates(context) {
        let done_dir = candidate_root.join("done");
        if let Some(path) = find_gate_task_in_dir(&done_dir, task_id)? {
            return Ok(Some(gate_task_status_from_path(path)?));
        }

        let todo_dir = candidate_root.join("todo");
        if let Some(path) = find_gate_task_in_dir(&todo_dir, task_id)? {
            return Ok(Some(gate_task_status_from_path(path)?));
        }
    }

    Ok(None)
}

fn resolve_registered_gate_task_status(path: &Path) -> Result<Option<GateTaskStatus>> {
    if path.is_file() {
        return Ok(Some(gate_task_status_from_path(path.to_path_buf())?));
    }

    let Some(file_name) = path.file_name() else {
        return Ok(None);
    };
    let Some(parent) = path.parent() else {
        return Ok(None);
    };

    if parent.file_name().and_then(|segment| segment.to_str()) == Some("todo") {
        let moved_path = parent
            .parent()
            .map(|root| root.join("done").join(file_name))
            .filter(|candidate| candidate.is_file());
        if let Some(moved_path) = moved_path {
            return Ok(Some(gate_task_status_from_path(moved_path)?));
        }
    }

    Ok(None)
}

fn gate_root_candidates(context: &ToolContext) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    collect_gate_roots(&mut candidates, &context.config.maenifold_root);
    if let Ok(current_dir) = std::env::current_dir() {
        collect_gate_roots(&mut candidates, &current_dir);
    }
    candidates
}

fn collect_gate_roots(candidates: &mut Vec<PathBuf>, start: &Path) {
    for ancestor in start.ancestors() {
        if ancestor.join("todo").is_dir()
            && ancestor.join("done").is_dir()
            && !candidates.iter().any(|existing| existing == ancestor)
        {
            candidates.push(ancestor.to_path_buf());
        }
    }
}

fn find_gate_task_in_dir(dir: &Path, task_id: &str) -> Result<Option<PathBuf>> {
    if !dir.is_dir() {
        return Ok(None);
    }

    let mut matches = fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().and_then(|ext| ext.to_str()) == Some("md")
                && path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .is_some_and(|stem| stem == task_id || stem.starts_with(&format!("{task_id}-")))
        })
        .collect::<Vec<_>>();
    matches.sort();
    Ok(matches.into_iter().next())
}

fn read_gate_task_state(path: &Path) -> Result<Option<String>> {
    let content = fs::read_to_string(path)?;
    let yaml = extract_yaml_frontmatter(&content);
    let Some(yaml) = yaml else {
        return Ok(None);
    };
    let parsed = serde_yaml::from_str::<serde_yaml::Mapping>(&yaml)?;
    Ok(parsed
        .get(&YamlValue::String("state".to_string()))
        .and_then(YamlValue::as_str)
        .map(str::to_string))
}

fn gate_task_status_from_path(path: PathBuf) -> Result<GateTaskStatus> {
    let effective_state = if path
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|segment| segment.to_str())
        .is_some_and(|segment| segment == "done")
    {
        "done".to_string()
    } else {
        read_gate_task_state(&path)?.unwrap_or_else(|| "pending".to_string())
    };
    Ok(GateTaskStatus {
        path,
        terminal: effective_state.eq_ignore_ascii_case("done"),
        effective_state,
    })
}

fn extract_yaml_frontmatter(content: &str) -> Option<String> {
    let normalized = content.strip_prefix('\u{feff}').unwrap_or(content);
    let remainder = normalized.strip_prefix("---\n")?;
    let end = remainder.find("\n---")?;
    Some(remainder[..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        HostSessionType, ResumeSubmachineResult, SubmachineType, find_gate_task_status,
        maybe_resume_submachine, parse_submachine_type, register_host_submachine,
    };
    use crate::config::Config;
    use crate::markdown::{Frontmatter, read_markdown, write_markdown};
    use crate::sessions::frontmatter_string;
    use crate::test_util::ConfigResetGuard;
    use crate::tools::ToolContext;
    use crate::util::json::OutputFormat;
    use serde_yaml::Value as YamlValue;
    use std::collections::BTreeMap;
    use tempfile::TempDir;

    fn test_context() -> (TempDir, ToolContext, ConfigResetGuard) {
        let tmp = TempDir::new().expect("temp dir");
        Config::override_root(tmp.path());
        Config::set_database_path(tmp.path().join("memory.db"));
        let config = Config::current();
        config.ensure_directories().expect("ensure directories");
        (
            tmp,
            ToolContext::new(config, OutputFormat::Markdown),
            ConfigResetGuard,
        )
    }

    fn write_session(
        context: &ToolContext,
        session_type: HostSessionType,
        session_id: &str,
        parent: Option<&str>,
    ) {
        let mut frontmatter: Frontmatter = BTreeMap::new();
        frontmatter.insert(
            "title".to_string(),
            YamlValue::String(format!("{} {}", session_type.as_str(), session_id)),
        );
        frontmatter.insert(
            "status".to_string(),
            YamlValue::String("active".to_string()),
        );
        frontmatter.insert(
            "phase".to_string(),
            YamlValue::String("running".to_string()),
        );
        frontmatter.insert(
            "activeSubmachineType".to_string(),
            YamlValue::String(String::new()),
        );
        frontmatter.insert(
            "activeSubmachineSessionId".to_string(),
            YamlValue::String(String::new()),
        );
        frontmatter.insert(
            "activeSubmachineTarget".to_string(),
            YamlValue::String(String::new()),
        );
        if let Some(parent) = parent {
            frontmatter.insert("parent".to_string(), YamlValue::String(parent.to_string()));
        }
        write_markdown(
            &session_type.session_uri(session_id),
            Some(&frontmatter),
            "# Session\n",
            &context.config,
        )
        .expect("write session");
    }

    #[test]
    fn parse_submachine_type_defaults_to_sequential() {
        assert_eq!(
            parse_submachine_type(None).expect("default type"),
            SubmachineType::Sequential
        );
    }

    #[test]
    fn gate_task_status_uses_done_directory_as_terminal() {
        let (tmp, context, _guard) = test_context();
        let done_dir = tmp.path().join("done");
        std::fs::create_dir_all(&done_dir).expect("create done dir");
        std::fs::create_dir_all(tmp.path().join("todo")).expect("create todo dir");
        std::fs::write(
            done_dir.join("T-108.1-example.md"),
            "---\nstate: blue-team\n---\n",
        )
        .expect("write gate task");

        let status = find_gate_task_status(&context, "T-108.1", None)
            .expect("gate status")
            .expect("task found");
        assert!(status.terminal);
        assert_eq!(status.effective_state, "done");
        assert!(status.path.ends_with("done/T-108.1-example.md"));
    }

    #[test]
    fn register_host_submachine_rejects_workflow_cycles() {
        let (_tmp, context, _guard) = test_context();
        let host_session_id = "workflow-1735689600001";
        let child_session_id = "workflow-1735689600000";
        write_session(&context, HostSessionType::Workflow, child_session_id, None);
        write_session(
            &context,
            HostSessionType::Workflow,
            host_session_id,
            Some(&HostSessionType::Workflow.link(child_session_id)),
        );

        let mut host_frontmatter = read_markdown(
            &HostSessionType::Workflow.session_uri(host_session_id),
            &context.config,
        )
        .expect("read host")
        .0
        .expect("host frontmatter");
        let error = register_host_submachine(
            &context,
            HostSessionType::Workflow,
            host_session_id,
            &HostSessionType::Workflow.session_uri(host_session_id),
            &mut host_frontmatter,
            child_session_id,
            SubmachineType::Workflow,
        )
        .expect_err("cycle should be rejected");
        assert!(error.to_string().contains("create a cycle"));
    }

    #[test]
    fn gate_submachine_resume_uses_persisted_target_path() {
        let (tmp, context, _guard) = test_context();
        std::fs::create_dir_all(tmp.path().join("todo")).expect("create todo dir");
        std::fs::create_dir_all(tmp.path().join("done")).expect("create done dir");
        std::fs::create_dir_all(tmp.path().join("nested/todo")).expect("create nested todo dir");
        std::fs::create_dir_all(tmp.path().join("nested/done")).expect("create nested done dir");

        let authoritative_task = tmp.path().join("todo/T-108.1-authoritative.md");
        std::fs::write(&authoritative_task, "---\nstate: blue-team\n---\n").expect("write task");
        std::fs::write(
            tmp.path().join("nested/done/T-108.1-conflict.md"),
            "---\nstate: done\n---\n",
        )
        .expect("write conflicting task");

        let host_session_id = "workflow-1735689600001";
        write_session(&context, HostSessionType::Workflow, host_session_id, None);
        let host_uri = HostSessionType::Workflow.session_uri(host_session_id);
        let (frontmatter, _, _) = read_markdown(&host_uri, &context.config).expect("read host");
        let mut frontmatter = frontmatter.expect("host frontmatter");

        register_host_submachine(
            &context,
            HostSessionType::Workflow,
            host_session_id,
            &host_uri,
            &mut frontmatter,
            "T-108.1",
            SubmachineType::Gate,
        )
        .expect("register gate");

        let moved_task = tmp.path().join("done/T-108.1-authoritative.md");
        std::fs::rename(&authoritative_task, &moved_task).expect("move authoritative task");

        let (frontmatter, existing_content, _) =
            read_markdown(&host_uri, &context.config).expect("read host after register");
        let frontmatter = frontmatter.expect("frontmatter after register");
        assert_eq!(
            frontmatter_string(&frontmatter, "activeSubmachineTarget"),
            authoritative_task.to_string_lossy()
        );

        let resumed = maybe_resume_submachine(
            &context,
            HostSessionType::Workflow,
            &host_uri,
            frontmatter,
            &existing_content,
        )
        .expect("resume");

        match resumed {
            ResumeSubmachineResult::Ready(updated_frontmatter) => {
                assert_eq!(frontmatter_string(&updated_frontmatter, "phase"), "running");
                assert_eq!(
                    frontmatter_string(&updated_frontmatter, "activeSubmachineTarget"),
                    ""
                );
            }
            ResumeSubmachineResult::Waiting(message) => {
                panic!("expected resumed state, got waiting: {message}");
            }
        }
    }
}
