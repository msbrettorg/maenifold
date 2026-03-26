use crate::acpx_driver::AcpxDriver;
use crate::error::{MaenifoldError, Result};
use crate::thinking::sequential_thinking;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

static WIKILINKS_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\[\[.*?\]\]").unwrap());
static WORKFLOW_SESSION_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"workflow-\d+").unwrap());
static STEP_PROGRESS_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"Step (\d+)/(\d+)").unwrap());

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThinkResult {
    pub session_id: String,
    pub conclusion: Option<String>,
    pub thoughts_completed: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThinkOptions {
    pub total_thoughts: u32,
    pub parent_workflow_id: Option<String>,
}

impl Default for ThinkOptions {
    fn default() -> Self {
        Self {
            total_thoughts: 5,
            parent_workflow_id: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowRunStatus {
    Completed,
    Failed(String),
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowResult {
    pub session_id: String,
    pub conclusion: Option<String>,
    pub steps_completed: u32,
    pub status: WorkflowRunStatus,
}

#[derive(Debug, Deserialize)]
struct Envelope<T> {
    success: bool,
    data: T,
    error: Option<EnvelopeError>,
}

#[derive(Debug, Deserialize)]
struct EnvelopeError {
    message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThinkSessionData {
    session_id: String,
}

fn think_seed_payload(topic: &str, opts: &ThinkOptions) -> Value {
    let mut seed = json!({
        "thoughtNumber": 0,
        "totalThoughts": opts.total_thoughts,
        "nextThoughtNeeded": true,
        "response": format!("[[{}]]", slug(topic))
    });
    if let Some(parent) = &opts.parent_workflow_id {
        seed["parentWorkflowId"] = json!(parent);
    }
    seed
}

pub(crate) async fn seed_think_session(
    context: &ToolContext,
    topic: &str,
    opts: &ThinkOptions,
) -> Result<String> {
    let json_context = ToolContext::new(context.config.clone(), OutputFormat::Json);
    let ThinkSessionData { session_id } =
        think_call(&json_context, &think_seed_payload(topic, opts))?;
    Ok(session_id)
}

pub(crate) async fn drive_think_session(
    context: &ToolContext,
    agent: &str,
    session_id: &str,
    topic: &str,
    opts: &ThinkOptions,
) -> Result<ThinkResult> {
    let json_context = ToolContext::new(context.config.clone(), OutputFormat::Json);
    let acpx_session = format!("think-{session_id}");
    let driver = AcpxDriver::new(agent.to_string(), 0, context.config.maenifold_root.clone())
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    driver
        .create_session(&acpx_session)
        .await
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    let mut n = 1u32;
    loop {
        let prompt = format!(
            "Thought {n}/~{} on: {topic}. Include [[WikiLinks]]. Say DONE when complete.",
            opts.total_thoughts
        );
        let response = driver
            .prompt(&acpx_session, &prompt)
            .await
            .map_err(|error| MaenifoldError::Message(error.to_string()))?;
        let response = tag_response(&response.text, &WIKILINKS_RE);
        let done = response.trim().to_ascii_lowercase().starts_with("done")
            || n >= opts.total_thoughts + 3;
        let payload = if done {
            json!({"sessionId":session_id,"response":&response,"thoughtNumber":n,"totalThoughts":opts.total_thoughts,"nextThoughtNeeded":false,"conclusion":&response})
        } else {
            json!({"sessionId":session_id,"response":&response,"thoughtNumber":n,"totalThoughts":opts.total_thoughts,"nextThoughtNeeded":true})
        };
        let _: Value = think_call(&json_context, &payload)?;
        if done {
            return Ok(ThinkResult {
                session_id: session_id.to_string(),
                conclusion: Some(response),
                thoughts_completed: n,
            });
        }
        n += 1;
    }
}

pub async fn run_think(
    context: &ToolContext,
    agent: &str,
    topic: &str,
    opts: ThinkOptions,
) -> Result<ThinkResult> {
    let session_id = seed_think_session(context, topic, &opts).await?;
    drive_think_session(context, agent, &session_id, topic, &opts).await
}

pub async fn run_workflow(
    context: &ToolContext,
    workflow_id: &str,
    agent: &str,
    prompt: &str,
) -> Result<WorkflowResult> {
    let start = super::handler::workflow(context, &json!({ "workflowId": workflow_id }))?;
    let session_id = extract_workflow_session_id(&start).ok_or_else(|| {
        MaenifoldError::Message("Workflow session ID missing from response".to_string())
    })?;
    let driver = AcpxDriver::new(agent.to_string(), 0, context.config.maenifold_root.clone())
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    driver
        .create_session(&session_id)
        .await
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    let mut step = workflow_text(&start);
    let mut steps_completed = 0u32;
    let mut conclusion = None;

    loop {
        if let Some(status) = terminal_workflow_status(&step) {
            return Ok(WorkflowResult {
                session_id,
                conclusion,
                steps_completed,
                status,
            });
        }
        if step.starts_with("ERROR:") {
            return Err(MaenifoldError::Message(step));
        }

        if let Some((topic, total_thoughts)) = step_think_config(&step) {
            let think_opts = ThinkOptions {
                total_thoughts,
                parent_workflow_id: Some(session_id.clone()),
            };
            let think_session_id = seed_think_session(context, &topic, &think_opts).await?;
            let _ = super::handler::workflow(
                context,
                &json!({
                    "sessionId": &session_id,
                    "response": "Spawning [[thinking]] submachine",
                    "submachineSessionId": &think_session_id
                }),
            )?;
            let _think_result =
                drive_think_session(context, agent, &think_session_id, &topic, &think_opts).await?;
        }

        let response = driver
            .prompt(&session_id, &workflow_prompt(prompt, &step))
            .await
            .map_err(|error| MaenifoldError::Message(error.to_string()))?;
        if let Some(error) = response.error.filter(|value| !value.trim().is_empty()) {
            return Err(MaenifoldError::Message(error));
        }

        let agent_text = tag_response(response.text.trim(), &WIKILINKS_RE);
        let final_step =
            matches!(parse_step_progress(&step), Some((current, total)) if current == total);
        let payload = if final_step {
            conclusion = Some(agent_text.clone());
            json!({
                "sessionId": &session_id,
                "response": &agent_text,
                "status": "completed",
                "conclusion": &agent_text
            })
        } else {
            json!({ "sessionId": &session_id, "response": &agent_text })
        };

        step = workflow_text(&super::handler::workflow(context, &payload)?);
        steps_completed += 1;
    }
}

fn workflow_text(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Ok(envelope) = serde_json::from_str::<Envelope<String>>(trimmed) {
        return if envelope.success {
            envelope.data.trim().to_string()
        } else {
            envelope
                .error
                .map(|error| error.message)
                .unwrap_or_else(|| trimmed.to_string())
        };
    }
    if trimmed.starts_with('{') {
        if let Some(end) = json_prefix_end(trimmed) {
            let rest = trimmed[end..].trim();
            if !rest.is_empty() {
                return rest.to_string();
            }
        }
    }
    trimmed.to_string()
}

fn extract_workflow_session_id(text: &str) -> Option<String> {
    WORKFLOW_SESSION_RE
        .find(text)
        .map(|m| m.as_str().to_string())
}

fn parse_step_progress(text: &str) -> Option<(u32, u32)> {
    let captures = STEP_PROGRESS_RE.captures(text)?;
    let current = captures.get(1)?.as_str().parse().ok()?;
    let total = captures.get(2)?.as_str().parse().ok()?;
    Some((current, total))
}

fn terminal_workflow_status(step: &str) -> Option<WorkflowRunStatus> {
    if step.starts_with('✅') {
        Some(WorkflowRunStatus::Completed)
    } else if step.starts_with('❌') {
        Some(if step.to_ascii_lowercase().contains("cancelled") {
            WorkflowRunStatus::Cancelled
        } else {
            WorkflowRunStatus::Failed(step.to_string())
        })
    } else {
        None
    }
}

fn workflow_prompt(prompt: &str, step: &str) -> String {
    if prompt.trim().is_empty() {
        step.to_string()
    } else {
        format!("{prompt}\n{step}")
    }
}

fn step_think_config(step: &str) -> Option<(String, u32)> {
    if !step.contains(r#""requiresSequentialThinking":true"#)
        && !step.contains(r#""requiresSequentialThinking": true"#)
    {
        return None;
    }
    let parsed = step
        .find('{')
        .and_then(|i| serde_json::from_str::<Value>(&step[i..]).ok());
    let total_thoughts = parsed
        .as_ref()
        .and_then(|v| {
            v.pointer("/metadata/toolHints/SequentialThinking/estimated_thoughts")
                .or_else(|| v.pointer("/metadata/estimated_thoughts"))
                .and_then(Value::as_u64)
        })
        .unwrap_or(5) as u32;
    let topic = parsed
        .as_ref()
        .and_then(|v| v.get("description").and_then(Value::as_str))
        .unwrap_or(step)
        .to_string();
    Some((topic, total_thoughts))
}

fn json_prefix_end(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, ch) in text.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index + ch.len_utf8());
                }
            }
            _ => {}
        }
    }
    None
}

fn think_call<T: DeserializeOwned>(context: &ToolContext, payload: &Value) -> Result<T> {
    let envelope: Envelope<T> = serde_json::from_str(&sequential_thinking(context, payload)?)?;
    if envelope.success {
        Ok(envelope.data)
    } else {
        Err(MaenifoldError::Message(
            envelope
                .error
                .map(|error| error.message)
                .unwrap_or_else(|| "SequentialThinking failed".to_string()),
        ))
    }
}

fn tag_response(response: &str, wikilinks: &Regex) -> String {
    if wikilinks.is_match(response) {
        response.to_string()
    } else {
        format!("{response} [[step]]")
    }
}

fn slug(topic: &str) -> String {
    let slug = topic
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    let mut slug = slug.trim_matches('-').to_string();
    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "topic".to_string()
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::markdown::read_markdown;
    use crate::sessions::{date_parts_from_session_id, workflow_session_uri};
    use crate::workflow::handler;
    use serde_json::json;
    use serial_test::serial;
    use std::ffi::OsString;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    struct ConfigResetGuard;
    impl Drop for ConfigResetGuard {
        fn drop(&mut self) {
            Config::reset_overrides();
        }
    }

    fn think_context() -> (TempDir, ToolContext, ConfigResetGuard) {
        let tmp = TempDir::new().expect("temp dir");
        Config::override_root(tmp.path());
        Config::set_database_path(tmp.path().join("memory.db"));
        let config = Config::current();
        config.ensure_directories().expect("ensure dirs");
        (
            tmp,
            ToolContext::new(config, OutputFormat::Markdown),
            ConfigResetGuard,
        )
    }

    fn workflow_context(
        workflow_id: &str,
        step_count: usize,
    ) -> (TempDir, ToolContext, ConfigResetGuard) {
        let (tmp, context, reset) = think_context();
        fs::create_dir_all(context.config.assets_path.join("workflows")).expect("workflows dir");
        let steps = (1..=step_count)
            .map(|n| {
                json!({
                    "id": format!("step-{n}"),
                    "name": format!("Step {n}"),
                    "description": format!("Handle [[step-{n}]]")
                })
            })
            .collect::<Vec<_>>();
        fs::write(
            context
                .config
                .assets_path
                .join("workflows")
                .join(format!("{workflow_id}.json")),
            serde_json::to_vec_pretty(&json!({ "steps": steps })).expect("workflow json"),
        )
        .expect("write workflow");
        (tmp, context, reset)
    }

    #[tokio::test]
    #[serial(config)]
    async fn happy_path_stops_when_agent_says_done() {
        let (_tmp, context, _reset) = think_context();
        let _path = install_acpx_stub(&context.config.maenifold_root);
        write_stub_config(
            &context.config.maenifold_root,
            json!(["first [[one]]", "second [[two]]", "DONE [[three]]"]),
        );
        let result = run_think(&context, "codex", "Topic", ThinkOptions::default())
            .await
            .expect("think runner");
        assert_eq!(result.thoughts_completed, 3);
        assert_eq!(result.conclusion.as_deref(), Some("DONE [[three]]"));
    }

    #[tokio::test]
    #[serial(config)]
    async fn overflow_protection_stops_at_total_plus_three() {
        let (_tmp, context, _reset) = think_context();
        let _path = install_acpx_stub(&context.config.maenifold_root);
        write_stub_config(
            &context.config.maenifold_root,
            json!(vec!["keep going [[step]]"; 6]),
        );
        let result = run_think(
            &context,
            "codex",
            "Topic",
            ThinkOptions {
                total_thoughts: 1,
                parent_workflow_id: None,
            },
        )
        .await
        .expect("think runner");
        assert_eq!(result.thoughts_completed, 4);
    }

    #[tokio::test]
    #[serial(config)]
    async fn injects_step_wikilink_when_agent_response_has_none() {
        let (_tmp, context, _reset) = think_context();
        let _path = install_acpx_stub(&context.config.maenifold_root);
        write_stub_config(
            &context.config.maenifold_root,
            json!(["plain text", "DONE plain text"]),
        );
        let result = run_think(&context, "codex", "Topic", ThinkOptions::default())
            .await
            .expect("think runner");
        let (y, m, d) = date_parts_from_session_id(&result.session_id);
        let uri = format!(
            "memory://thinking/sequential/{y:04}/{m:02}/{d:02}/{}",
            result.session_id
        );
        let (_, content, _) = read_markdown(&uri, &context.config).expect("read thinking session");
        assert!(content.contains("plain text [[step]]"));
        assert!(content.contains("DONE plain text [[step]]"));
    }

    #[tokio::test]
    #[serial(config)]
    async fn run_think_seeds_parent_workflow_linkage() {
        let (_tmp, context, _reset) = workflow_context("parent-link", 1);
        let _path = install_acpx_stub(&context.config.maenifold_root);
        write_stub_config(&context.config.maenifold_root, json!(["DONE [[child]]"]));

        let start = handler::workflow(&context, &json!({ "workflowId": "parent-link" }))
            .expect("start workflow");
        let workflow_session_id = extract_workflow_session_id(&start).expect("workflow session id");

        let result = run_think(
            &context,
            "codex",
            "Topic",
            ThinkOptions {
                total_thoughts: 1,
                parent_workflow_id: Some(workflow_session_id.clone()),
            },
        )
        .await
        .expect("think runner");

        let (y, m, d) = date_parts_from_session_id(&result.session_id);
        let uri = format!(
            "memory://thinking/sequential/{y:04}/{m:02}/{d:02}/{}",
            result.session_id
        );
        let (frontmatter, _, _) = read_markdown(&uri, &context.config).expect("read thinking");
        let expected_parent = format!("[[workflow/{workflow_session_id}]]");
        assert_eq!(
            frontmatter
                .as_ref()
                .and_then(|values| values.get("parent"))
                .and_then(|value| value.as_str()),
            Some(expected_parent.as_str())
        );

        let (workflow_frontmatter, _, _) =
            read_markdown(&workflow_session_uri(&workflow_session_id), &context.config)
                .expect("read workflow");
        let related_link = format!("[[sequential/{}]]", result.session_id);
        let related = workflow_frontmatter
            .as_ref()
            .and_then(|values| values.get("related"))
            .and_then(|value| value.as_sequence())
            .expect("workflow related links");
        assert!(
            related
                .iter()
                .any(|value| value.as_str() == Some(related_link.as_str())),
            "workflow frontmatter should link the seeded sequential session"
        );
    }

    #[tokio::test]
    #[serial(config)]
    async fn workflow_runner_completes_multi_step_workflow() {
        let (_tmp, context, _reset) = workflow_context("happy", 2);
        let _path = install_acpx_stub(&context.config.maenifold_root);
        write_stub_config(
            &context.config.maenifold_root,
            json!({
                "runner-agent": {
                    "responses": [
                        { "text": "First [[response]]" },
                        { "text": "Final [[conclusion]]" }
                    ]
                }
            }),
        );

        let result = run_workflow(&context, "happy", "runner-agent", "System prompt")
            .await
            .expect("workflow runner");

        assert!(result.session_id.starts_with("workflow-"));
        assert_eq!(result.steps_completed, 2);
        assert_eq!(result.conclusion.as_deref(), Some("Final [[conclusion]]"));
        assert_eq!(result.status, WorkflowRunStatus::Completed);
    }

    #[tokio::test]
    #[serial(config)]
    async fn workflow_runner_stops_on_terminal_success() {
        let (_tmp, context, _reset) = workflow_context("single", 1);
        let _path = install_acpx_stub(&context.config.maenifold_root);
        write_stub_config(
            &context.config.maenifold_root,
            json!({ "runner-agent": { "responses": [{ "text": "Done [[conclusion]]" }] } }),
        );

        let result = run_workflow(&context, "single", "runner-agent", "Prompt")
            .await
            .expect("workflow runner");
        let state = read_stub_state(&context.config.maenifold_root);

        assert_eq!(result.steps_completed, 1);
        assert_eq!(state["counts"]["runner-agent"], 1);
        assert_eq!(result.status, WorkflowRunStatus::Completed);
    }

    #[tokio::test]
    #[serial(config)]
    async fn workflow_runner_propagates_agent_failures() {
        let (_tmp, context, _reset) = workflow_context("failure", 1);
        let _path = install_acpx_stub(&context.config.maenifold_root);
        write_stub_config(
            &context.config.maenifold_root,
            json!({ "runner-agent": { "stderr": "agent exploded", "exitCode": 2 } }),
        );

        let error = run_workflow(&context, "failure", "runner-agent", "Prompt")
            .await
            .expect_err("workflow runner should fail");

        assert!(error.to_string().contains("agent exploded"));
    }

    #[tokio::test]
    #[serial(config)]
    async fn workflow_runner_tags_response_without_wikilinks() {
        let (_tmp, context, _reset) = workflow_context("tagging", 1);
        let _path = install_acpx_stub(&context.config.maenifold_root);
        write_stub_config(
            &context.config.maenifold_root,
            json!({ "runner-agent": { "responses": [{ "text": "plain text no links" }] } }),
        );

        let result = run_workflow(&context, "tagging", "runner-agent", "Prompt")
            .await
            .expect("workflow runner");

        assert_eq!(result.steps_completed, 1);
        assert_eq!(
            result.conclusion.as_deref(),
            Some("plain text no links [[step]]"),
            "tag_response should append [[step]] to wikilink-free agent responses"
        );
    }

    #[tokio::test]
    #[serial(config)]
    async fn workflow_runner_thinks_before_agent_prompt_when_required() {
        let (_tmp, context, _reset) = workflow_context("thinking", 1);
        fs::write(context.config.assets_path.join("workflows/thinking.json"), serde_json::to_vec_pretty(&json!({ "steps": [{ "id": "step-1", "description": "Analyze [[step]]", "metadata": { "requiresSequentialThinking": true, "toolHints": { "SequentialThinking": { "estimated_thoughts": 1 } } } }] })).expect("workflow json")).expect("write workflow");
        let _path = install_acpx_stub(&context.config.maenifold_root);
        write_stub_config(
            &context.config.maenifold_root,
            json!({ "runner-agent": { "responses": [{ "text": "DONE [[thought]]" }, { "text": "Done [[conclusion]]" }] } }),
        );
        run_workflow(&context, "thinking", "runner-agent", "Prompt")
            .await
            .expect("workflow runner");
        let state = read_stub_state(&context.config.maenifold_root);
        assert!(
            state["prompts"][0]["session"]
                .as_str()
                .unwrap()
                .starts_with("think-")
        );
        assert!(
            state["prompts"][1]["session"]
                .as_str()
                .unwrap()
                .starts_with("workflow-")
        );
        assert_eq!(
            state["observations"][0]["registeredBeforePrompt"].as_bool(),
            Some(true),
            "workflow should register the submachine before driving the first thought"
        );
        assert_eq!(
            state["observations"][0]["activeSubmachineSessionId"].as_str(),
            state["prompts"][0]["session"]
                .as_str()
                .map(|session| session.trim_start_matches("think-"))
        );
        assert_eq!(
            state["observations"][0]["parentWorkflowSession"].as_str(),
            state["prompts"][1]["session"].as_str()
        );
    }

    #[tokio::test]
    #[serial(config)]
    async fn workflow_runner_skips_thinking_without_flag() {
        let (_tmp, context, _reset) = workflow_context("no-thinking", 1);
        let _path = install_acpx_stub(&context.config.maenifold_root);
        write_stub_config(
            &context.config.maenifold_root,
            json!({ "runner-agent": { "responses": [{ "text": "Done [[conclusion]]" }] } }),
        );
        run_workflow(&context, "no-thinking", "runner-agent", "Prompt")
            .await
            .expect("workflow runner");
        let state = read_stub_state(&context.config.maenifold_root);
        assert_eq!(state["prompts"].as_array().map(Vec::len), Some(1));
        assert!(
            state["prompts"][0]["session"]
                .as_str()
                .unwrap()
                .starts_with("workflow-")
        );
    }

    fn install_acpx_stub(root: &Path) -> PathGuard {
        let bin = root.join("bin");
        fs::create_dir_all(&bin).expect("bin dir");
        let stub = bin.join("acpx");
        fs::write(&stub, acpx_stub()).expect("write stub");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut p = fs::metadata(&stub).expect("metadata").permissions();
            p.set_mode(0o755);
            fs::set_permissions(&stub, p).expect("chmod");
        }
        prepend_path(&bin)
    }

    fn acpx_stub() -> &'static str {
        r#"#!/usr/bin/env python3
import json, os, re, sys
root=os.getcwd(); cfg=os.path.join(root,".runner-stub-config.json"); state=os.path.join(root,".runner-stub-state.json")
def read(path, default):
    try:
        with open(path, "r", encoding="utf-8") as h: return json.load(h)
    except Exception: return default
def read_text(path):
    try:
        with open(path, "r", encoding="utf-8") as h: return h.read()
    except Exception: return ""
def find_session_file(session_id):
    target=session_id + ".md"
    for dirpath, _, filenames in os.walk(root):
        if target in filenames: return os.path.join(dirpath, target)
    return None
def observe_prompt(session, agent):
    observation={"session":session,"agent":agent}
    if session.startswith("think-"):
        sequential_session_id=session[len("think-"):]
        observation["activeSubmachineSessionId"]=sequential_session_id
        sequential_path=find_session_file(sequential_session_id)
        if sequential_path:
            sequential_text=read_text(sequential_path)
            match=re.search(r"\[\[workflow/(workflow-[A-Za-z0-9-]+)\]\]", sequential_text)
            if match:
                workflow_session_id=match.group(1)
                observation["parentWorkflowSession"]=workflow_session_id
                workflow_path=find_session_file(workflow_session_id)
                if workflow_path:
                    workflow_text=read_text(workflow_path)
                    observation["registeredBeforePrompt"]=bool(re.search(r"activeSubmachineSessionId:\s*['\"]?" + re.escape(sequential_session_id) + r"['\"]?", workflow_text))
                    observation["workflowWaiting"]=bool(re.search(r"phase:\s*['\"]?waiting['\"]?", workflow_text))
    return observation
def write(path, data):
    with open(path, "w", encoding="utf-8") as h: json.dump(data, h)
args=sys.argv[1:]; s=read(state, {"counts":{}, "sessions":[], "prompts":[], "observations":[]})
if len(args) >= 5 and args[1:4] == ["sessions", "new", "--name"]:
    s["sessions"].append(args[4]); write(state, s); sys.exit(0)
session=args[args.index("-s")+1] if "-s" in args else ""
agent=args[args.index("-s")-1] if "-s" in args and args.index("-s") > 0 else ""
config=read(cfg, [])
if isinstance(config, list):
    key=session
    entry={"responses": config}
else:
    key=session if session in config else agent if agent in config else "*"
    entry=config.get(key, {})
    if isinstance(entry, list):
        entry={"responses": entry}
count=s["counts"].get(key, 0); s["counts"][key]=count+1; write(state, s)
s["prompts"].append({"session":session,"agent":agent}); write(state, s)
s["observations"].append(observe_prompt(session, agent)); write(state, s)
responses=entry.get("responses", [entry]); behavior=responses[count] if count < len(responses) else responses[-1]
if isinstance(behavior, str): behavior={"text": behavior}
if "stderr" in behavior: print(behavior["stderr"], file=sys.stderr)
if "error" in behavior:
    print(json.dumps({"type":"error","message":behavior["error"]}))
elif "text" in behavior:
    print(json.dumps({"type":"text","text":behavior["text"]}))
sys.exit(int(behavior.get("exitCode", 0)))
"#
    }

    fn write_stub_config(root: &Path, responses: Value) {
        fs::write(
            root.join(".runner-stub-config.json"),
            serde_json::to_vec(&responses).expect("json"),
        )
        .expect("write config");
    }

    fn read_stub_state(root: &Path) -> Value {
        serde_json::from_slice(&fs::read(root.join(".runner-stub-state.json")).expect("read state"))
            .expect("parse state")
    }

    fn prepend_path(path: &Path) -> PathGuard {
        let original = std::env::var_os("PATH");
        let updated = match &original {
            Some(existing) => std::env::join_paths(
                std::iter::once(path.to_path_buf()).chain(std::env::split_paths(existing)),
            )
            .expect("join PATH"),
            None => std::env::join_paths([path]).expect("join PATH"),
        };
        unsafe {
            std::env::set_var("PATH", &updated);
        }
        PathGuard { original }
    }

    struct PathGuard {
        original: Option<OsString>,
    }
    impl Drop for PathGuard {
        fn drop(&mut self) {
            if let Some(path) = self.original.take() {
                unsafe {
                    std::env::set_var("PATH", path);
                }
            } else {
                unsafe {
                    std::env::remove_var("PATH");
                }
            }
        }
    }
}
