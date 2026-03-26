use std::collections::HashMap;
use std::sync::Arc;

use serde::Deserialize;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::acp_config::sync_acpx_config;
use crate::acpx_driver::AcpxDriver;
use crate::error::{MaenifoldError, Result};
use crate::markdown::read_markdown;
use crate::registry::ToolRegistry;
use crate::sessions::workflow_session_uri;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;

use super::super::runner::{self, WorkflowRunStatus};
use super::dispatch::dispatch_pipeline;
use super::session_io::{derive_pipeline_node_states, frontmatter_value, read_pipeline_session};
use super::supervisor::supervisor_wait_if_paused;
use super::update::update_pipeline_node_status;
use super::{EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineNode};

const DEFAULT_MAX_CONCURRENCY: usize = 8;
const MAX_CONCLUSION_CHARS: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExecutionResult {
    pub completed: usize,
    pub abandoned: usize,
    pub status: String,
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
struct DispatchResult {
    dispatched: Vec<DispatchedNode>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DispatchedNode {
    node_id: String,
    #[allow(dead_code)]
    session_id: Option<String>,
}

pub(super) async fn execute_pipeline(
    context: &ToolContext,
    session_id: &str,
    prompt: &str,
    cancel: CancellationToken,
) -> Result<ExecutionResult> {
    sync_acpx_config(&context.config.maenifold_root);
    let json_context = ToolContext::new(context.config.clone(), OutputFormat::Json);
    let (definition, max_concurrency) = load_execution_config(&json_context, session_id)?;
    let nodes_by_id = definition
        .nodes
        .iter()
        .cloned()
        .map(|node| (node.id().to_string(), node))
        .collect::<HashMap<_, _>>();
    let semaphore = Arc::new(Semaphore::new(max_concurrency));
    let mut join_set = JoinSet::new();
    let mut completed = 0usize;
    let mut abandoned = 0usize;
    let prompt = prompt.to_string();

    loop {
        if cancel.is_cancelled() {
            break;
        }

        supervisor_wait_if_paused(session_id, &cancel).await;
        if cancel.is_cancelled() {
            break;
        }

        let dispatched = parse_dispatch_result(dispatch_pipeline(&json_context, session_id)?)?;
        let node_states = if dispatched.is_empty() {
            None
        } else {
            Some(load_pipeline_node_states(&json_context, session_id)?)
        };
        for dispatch in &dispatched {
            let permit = semaphore.clone().acquire_owned().await.map_err(|_| {
                MaenifoldError::Message(
                    "ERROR: Pipeline executor semaphore unexpectedly closed".to_string(),
                )
            })?;
            let node = nodes_by_id.get(&dispatch.node_id).ok_or_else(|| {
                MaenifoldError::Message(format!(
                    "ERROR: Pipeline definition missing node '{}'",
                    dispatch.node_id
                ))
            })?;
            let node_id = dispatch.node_id.clone();
            let is_tool_node = matches!(node, PipelineNode::Tool { .. });
            let agent = match node.acp_client() {
                Some(client) => Some(client.to_string()),
                None if is_tool_node => None,
                None => {
                    drop(permit);
                    update_pipeline_node_status(
                        &json_context,
                        session_id,
                        &node_id,
                        "abandoned",
                        None,
                    )?;
                    abandoned += 1;
                    continue;
                }
            };
            let execution = build_node_execution_request(
                &json_context,
                &definition,
                node_states.as_ref().expect("loaded for dispatched nodes"),
                node,
                dispatch,
                &prompt,
            )?;
            let ctx = ToolContext::new(context.config.clone(), context.output_format);
            join_set.spawn(async move {
                let _permit = permit;
                let result = match execution {
                    NodeExecutionRequest::Workflow {
                        workflow_id,
                        prompt,
                    } => {
                        let agent = agent.as_deref().expect("agent required for workflow nodes");
                        NodeTaskResult::Workflow(
                            runner::run_workflow(&ctx, &workflow_id, agent, &prompt).await,
                        )
                    }
                    NodeExecutionRequest::Prompt {
                        session_id,
                        prompt,
                        tool_mode,
                    } => {
                        let agent = agent.as_deref().expect("agent required for prompt nodes");
                        NodeTaskResult::Prompt(
                            run_prompt_node(&ctx, agent, &session_id, &prompt, &tool_mode).await,
                        )
                    }
                    NodeExecutionRequest::SequentialThinking {
                        topic,
                        total_thoughts,
                    } => {
                        let agent = agent.as_deref().expect("agent required for thinking nodes");
                        NodeTaskResult::SequentialThinking(
                            runner::run_think(
                                &ctx,
                                agent,
                                &topic,
                                runner::ThinkOptions {
                                    total_thoughts,
                                    parent_workflow_id: None,
                                },
                            )
                            .await,
                        )
                    }
                    NodeExecutionRequest::Tool {
                        tool_name,
                        tool_args,
                    } => NodeTaskResult::Tool(
                        run_tool_node(&ctx, &tool_name, tool_args.as_ref()).await,
                    ),
                };
                (node_id, result)
            });
        }

        if dispatched.is_empty() && join_set.is_empty() {
            break;
        }

        let Some(joined) = (tokio::select! {
            biased;
            _ = cancel.cancelled() => None,
            joined = join_set.join_next(), if !join_set.is_empty() => joined,
        }) else {
            break;
        };

        let (node_id, result) = joined.map_err(|error| {
            MaenifoldError::Message(format!("ERROR: Node task failed: {error}"))
        })?;

        match classify_node_result(result) {
            NodeOutcome::Completed(conclusion) => {
                update_pipeline_node_status(
                    &json_context,
                    session_id,
                    &node_id,
                    "completed",
                    Some(&conclusion),
                )?;
                completed += 1;
            }
            NodeOutcome::Abandoned => {
                update_pipeline_node_status(
                    &json_context,
                    session_id,
                    &node_id,
                    "abandoned",
                    None,
                )?;
                abandoned += 1;
            }
        }
    }

    let cancelled = cancel.is_cancelled();
    if cancelled {
        join_set.abort_all();
        drop(join_set);
        abandoned += abandon_active_nodes(&json_context, session_id)?;
    }

    Ok(ExecutionResult {
        completed,
        abandoned,
        status: derive_execution_status(&json_context, session_id, cancelled)?,
    })
}

fn load_pipeline_node_states(
    context: &ToolContext,
    session_id: &str,
) -> Result<HashMap<String, NodeState>> {
    let (_, frontmatter, _) = read_pipeline_session(context, session_id)?;
    derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)
}

#[derive(Debug)]
enum NodeExecutionRequest {
    Workflow {
        workflow_id: String,
        prompt: String,
    },
    Prompt {
        session_id: String,
        prompt: String,
        tool_mode: String,
    },
    SequentialThinking {
        topic: String,
        total_thoughts: u32,
    },
    Tool {
        tool_name: String,
        tool_args: Option<JsonValue>,
    },
}

/// Resolve a dot-path expression against a JSON value.
///
/// Supports `a.b`, `results.0.title`, and `results[0].title` syntax.
/// Returns [`JsonValue::Null`] for any unresolved segment.
fn resolve_dot_path(value: &JsonValue, path: &str) -> JsonValue {
    let segments = match parse_dot_path(path) {
        Some(segments) => segments,
        None => return JsonValue::Null,
    };

    let mut current = value;
    for segment in segments {
        current = match current {
            JsonValue::Object(map) => match map.get(segment.as_str()) {
                Some(value) => value,
                None => return JsonValue::Null,
            },
            JsonValue::Array(items) => {
                match segment
                    .parse::<usize>()
                    .ok()
                    .and_then(|index| items.get(index))
                {
                    Some(value) => value,
                    None => return JsonValue::Null,
                }
            }
            _ => return JsonValue::Null,
        };
    }

    current.clone()
}

fn parse_dot_path(path: &str) -> Option<Vec<String>> {
    if path.is_empty() {
        return None;
    }

    let mut segments = Vec::new();
    let mut chars = path.chars().peekable();
    let mut current = String::new();
    let mut allow_empty_dot = false;

    while let Some(ch) = chars.next() {
        match ch {
            '.' => {
                if !current.is_empty() {
                    segments.push(std::mem::take(&mut current));
                    allow_empty_dot = false;
                } else if allow_empty_dot {
                    allow_empty_dot = false;
                } else {
                    return None;
                }
            }
            '[' => {
                if !current.is_empty() {
                    segments.push(std::mem::take(&mut current));
                }

                let mut index = String::new();
                let mut closed = false;
                for next in chars.by_ref() {
                    if next == ']' {
                        closed = true;
                        break;
                    }
                    index.push(next);
                }

                if !closed || index.is_empty() {
                    return None;
                }

                segments.push(index);
                allow_empty_dot = true;
            }
            ']' => return None,
            other => {
                current.push(other);
                allow_empty_dot = false;
            }
        }
    }

    if !current.is_empty() {
        segments.push(current);
    }

    Some(segments)
}

/// Apply edge mappings for a target tool node.
///
/// Collects incoming edges with `mapping` from completed source nodes, resolves
/// each dot-path against the source node's output JSON, and merges the resolved
/// values into `static_args`. Mapped values override static values.
///
/// Returns `static_args` unchanged when no incoming edges carry mappings.
fn mapped_tool_args(
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    target_node_id: &str,
    static_args: Option<JsonValue>,
) -> Option<JsonValue> {
    let mut merged_args = match static_args.clone() {
        Some(JsonValue::Object(map)) => map,
        _ => serde_json::Map::new(),
    };
    let mut applied_mapping = false;

    for edge in definition
        .edges
        .iter()
        .filter(|edge| edge.target == target_node_id)
    {
        let Some(mapping) = edge.mapping.as_ref() else {
            continue;
        };
        let Some(source_state) = node_states.get(&edge.source) else {
            continue;
        };
        if source_state.status != NodeStatus::Completed {
            continue;
        }

        let source_output = source_state
            .output
            .as_deref()
            .and_then(|value| serde_json::from_str::<JsonValue>(value).ok())
            .unwrap_or(JsonValue::Null);

        for (target_key, source_path) in mapping {
            applied_mapping = true;
            merged_args.insert(
                target_key.clone(),
                resolve_dot_path(&source_output, source_path),
            );
        }
    }

    if applied_mapping {
        Some(JsonValue::Object(merged_args))
    } else {
        static_args
    }
}

fn build_node_execution_request(
    context: &ToolContext,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    node: &PipelineNode,
    dispatch: &DispatchedNode,
    ambient_prompt: &str,
) -> Result<NodeExecutionRequest> {
    Ok(match node {
        PipelineNode::Workflow { workflow_id, .. } => {
            let assembled_prompt =
                assemble_node_prompt(context, definition, node_states, node, ambient_prompt)?;
            NodeExecutionRequest::Workflow {
                workflow_id: workflow_id.clone(),
                prompt: assembled_prompt,
            }
        }
        PipelineNode::Prompt { tool_mode, .. } => NodeExecutionRequest::Prompt {
            session_id: dispatch.session_id.clone().unwrap_or_else(|| {
                node_states
                    .get(node.id())
                    .map(|state| state.session_id.clone())
                    .unwrap_or_else(|| format!("pipeline-{}", node.id()))
            }),
            prompt: assemble_node_prompt(context, definition, node_states, node, ambient_prompt)?,
            tool_mode: tool_mode.clone(),
        },
        PipelineNode::SequentialThinking { total_thoughts, .. } => {
            NodeExecutionRequest::SequentialThinking {
                topic: assemble_node_prompt(
                    context,
                    definition,
                    node_states,
                    node,
                    ambient_prompt,
                )?,
                total_thoughts: *total_thoughts,
            }
        }
        PipelineNode::Tool {
            tool_name,
            tool_args,
            ..
        } => NodeExecutionRequest::Tool {
            tool_name: tool_name.clone(),
            tool_args: mapped_tool_args(definition, node_states, node.id(), tool_args.clone()),
        },
    })
}

pub(super) fn assemble_node_prompt(
    context: &ToolContext,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    node: &PipelineNode,
    ambient_prompt: &str,
) -> Result<String> {
    let predecessor_context =
        predecessor_context_content(context, definition, node_states, node.id())?;
    let ambient_prompt = ambient_prompt.trim();
    let node_instruction = match node {
        PipelineNode::Workflow { prompt, .. } => prompt.as_deref(),
        PipelineNode::Prompt { prompt, .. } => Some(prompt.as_str()),
        PipelineNode::SequentialThinking { topic, .. } => Some(topic.as_str()),
        PipelineNode::Tool { .. } => None,
    }
    .map(str::trim)
    .filter(|value| !value.is_empty());

    let workflow_fallback =
        matches!(node, PipelineNode::Workflow { .. }) && node_instruction.is_none();
    let mut parts = Vec::new();

    if let Some(context) = predecessor_context
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        parts.push(context.to_string());
    }

    if !ambient_prompt.is_empty() {
        parts.push(ambient_prompt.to_string());
    }

    if !workflow_fallback {
        if let Some(instruction) = node_instruction {
            parts.push(instruction.to_string());
        }
    }

    Ok(parts.join("\n\n"))
}

fn load_execution_config(
    context: &ToolContext,
    session_id: &str,
) -> Result<(PipelineDefinition, usize)> {
    let (_, frontmatter, _) = read_pipeline_session(context, session_id)?;
    let definition: PipelineDefinition = frontmatter_value(&frontmatter, "definition")?;
    let max_concurrency = frontmatter
        .get("maxConcurrency")
        .and_then(YamlValue::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(DEFAULT_MAX_CONCURRENCY);
    Ok((definition, max_concurrency))
}

fn parse_dispatch_result(raw: String) -> Result<Vec<DispatchedNode>> {
    let response: Envelope<DispatchResult> = serde_json::from_str(&raw)?;
    if response.success {
        Ok(response.data.dispatched)
    } else {
        Err(MaenifoldError::Message(
            response
                .error
                .map(|error| error.message)
                .unwrap_or_else(|| "ERROR: Pipeline dispatch failed".to_string()),
        ))
    }
}

enum NodeOutcome {
    Completed(String),
    Abandoned,
}

enum NodeTaskResult {
    Workflow(Result<runner::WorkflowResult>),
    Prompt(Result<String>),
    SequentialThinking(Result<runner::ThinkResult>),
    Tool(Result<String>),
}

fn classify_node_result(result: NodeTaskResult) -> NodeOutcome {
    match result {
        NodeTaskResult::Workflow(result) => classify_workflow_result(result),
        NodeTaskResult::Prompt(result) => classify_prompt_result(result),
        NodeTaskResult::SequentialThinking(result) => classify_think_result(result),
        NodeTaskResult::Tool(result) => classify_prompt_result(result),
    }
}

fn classify_workflow_result(result: Result<runner::WorkflowResult>) -> NodeOutcome {
    match result {
        Ok(wf) => match wf.status {
            WorkflowRunStatus::Completed => {
                let conclusion = wf.conclusion.unwrap_or_default();
                NodeOutcome::Completed(truncate_chars(&conclusion, MAX_CONCLUSION_CHARS))
            }
            WorkflowRunStatus::Failed(_) | WorkflowRunStatus::Cancelled => NodeOutcome::Abandoned,
        },
        Err(_) => NodeOutcome::Abandoned,
    }
}

fn classify_prompt_result(result: Result<String>) -> NodeOutcome {
    match result {
        Ok(response) if !response.trim().is_empty() => NodeOutcome::Completed(response),
        Ok(_) | Err(_) => NodeOutcome::Abandoned,
    }
}

fn classify_think_result(result: Result<runner::ThinkResult>) -> NodeOutcome {
    match result {
        Ok(think) => think
            .conclusion
            .map(|conclusion| conclusion.trim().to_string())
            .filter(|conclusion| !conclusion.is_empty())
            .map(NodeOutcome::Completed)
            .unwrap_or(NodeOutcome::Abandoned),
        Err(_) => NodeOutcome::Abandoned,
    }
}

async fn run_prompt_node(
    context: &ToolContext,
    agent: &str,
    session_id: &str,
    prompt: &str,
    _tool_mode: &str,
) -> Result<String> {
    let driver = AcpxDriver::new(agent.to_string(), 0, context.config.maenifold_root.clone())
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    driver
        .create_session(session_id)
        .await
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;

    let response = driver
        .prompt(session_id, prompt)
        .await
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    if let Some(error) = response.error.filter(|value| !value.trim().is_empty()) {
        return Err(MaenifoldError::Message(error));
    }

    Ok(response.text.trim().to_string())
}

async fn run_tool_node(
    _context: &ToolContext,
    tool_name: &str,
    tool_args: Option<&JsonValue>,
) -> Result<String> {
    let arguments = tool_args
        .cloned()
        .unwrap_or_else(|| JsonValue::Object(Default::default()));

    ToolRegistry::global()
        .try_invoke(tool_name, &arguments, OutputFormat::Json)
        .map_err(|error| {
            if matches!(
                error,
                MaenifoldError::UnknownTool(_)
                    | MaenifoldError::InvalidPayload(_)
                    | MaenifoldError::InvalidArgument(_)
                    | MaenifoldError::InvalidResourceUri(_)
                    | MaenifoldError::AssetNotFound(_, _)
                    | MaenifoldError::PathTraversal(_)
                    | MaenifoldError::MissingProperty(_)
            ) {
                error
            } else {
                MaenifoldError::Message(format!("Error invoking {tool_name}: {error}"))
            }
        })
}

pub(super) fn predecessor_context_content(
    context: &ToolContext,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    downstream_node_id: &str,
) -> Result<Option<String>> {
    let mut sections = Vec::new();

    for edge in definition.edges.iter().filter(|edge| {
        edge.target == downstream_node_id
            && matches!(edge.edge_type, EdgeType::Success | EdgeType::Always)
    }) {
        let Some(upstream_state) = node_states.get(&edge.source) else {
            continue;
        };
        if upstream_state.status != NodeStatus::Completed {
            continue;
        }

        let Some(content) =
            predecessor_context_source(context, definition, &edge.source, upstream_state)?
        else {
            continue;
        };

        sections.push(format!("### From {}\n\n{}", edge.source, content));
    }

    if sections.is_empty() {
        Ok(None)
    } else {
        Ok(Some(sections.join("\n\n")))
    }
}

fn predecessor_context_source(
    context: &ToolContext,
    definition: &PipelineDefinition,
    upstream_node_id: &str,
    upstream_state: &NodeState,
) -> Result<Option<String>> {
    if let Some(output) = upstream_state
        .output
        .as_deref()
        .map(str::trim)
        .filter(|output| !output.is_empty())
    {
        return Ok(Some(output.to_string()));
    }

    let Some(upstream_node) = definition
        .nodes
        .iter()
        .find(|node| node.id() == upstream_node_id)
    else {
        return Ok(None);
    };

    if upstream_node.workflow_id().is_none() {
        return Ok(None);
    }

    let (frontmatter, _, _) = read_markdown(
        &workflow_session_uri(&upstream_state.session_id),
        &context.config,
    )?;
    Ok(frontmatter
        .as_ref()
        .and_then(|frontmatter| frontmatter.get("last"))
        .and_then(YamlValue::as_str)
        .map(str::trim)
        .filter(|last| !last.is_empty())
        .map(ToOwned::to_owned))
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

fn abandon_active_nodes(context: &ToolContext, session_id: &str) -> Result<usize> {
    let (_, frontmatter, _) = read_pipeline_session(context, session_id)?;
    let node_states =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;
    let active_nodes = node_states
        .iter()
        .filter(|(_, state)| matches!(state.status, NodeStatus::Active))
        .map(|(node_id, _)| node_id.clone())
        .collect::<Vec<_>>();

    for node_id in &active_nodes {
        update_pipeline_node_status(context, session_id, node_id, "abandoned", None)?;
    }

    Ok(active_nodes.len())
}

fn derive_execution_status(
    context: &ToolContext,
    session_id: &str,
    cancelled: bool,
) -> Result<String> {
    if cancelled {
        return Ok("cancelled".to_string());
    }

    let (_, frontmatter, _) = read_pipeline_session(context, session_id)?;
    let node_states =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;

    if node_states
        .values()
        .all(|state| matches!(state.status, NodeStatus::Completed))
    {
        Ok("completed".to_string())
    } else if node_states
        .values()
        .any(|state| matches!(state.status, NodeStatus::Stuck))
    {
        Ok("stuck".to_string())
    } else if node_states
        .values()
        .any(|state| matches!(state.status, NodeStatus::Abandoned))
    {
        Ok("abandoned".to_string())
    } else {
        Ok("active".to_string())
    }
}

#[cfg(test)]
mod pipeline_executor_tests {
    use super::*;
    use crate::markdown::{Frontmatter, read_markdown, write_markdown};
    use crate::sessions::{pipeline_session_uri, workflow_session_uri};
    use crate::util::json::OutputFormat;
    use crate::workflow::initial_workflow_frontmatter;
    use crate::workflow::pipeline::test_support::{
        pipeline_test_context, seed_pipeline_session_with_workflows,
    };
    use crate::workflow::pipeline::{EdgeType, NodeState, PipelineEdge, PipelineNode};
    use serde_json::{Value, json};
    use serde_yaml::Value as YamlValue;
    use serial_test::serial;
    use std::collections::HashMap;
    use std::ffi::OsString;
    use std::fs;
    use std::path::Path;
    use std::time::Duration;
    use tokio::time::{Instant, sleep};

    #[test]
    fn resolve_dot_path_handles_nested_object_and_arrays() {
        let value = json!({
            "results": [{"title": "First"}],
            "data": {"content": "Body"}
        });

        assert_eq!(resolve_dot_path(&value, "results.0.title"), json!("First"));
        assert_eq!(resolve_dot_path(&value, "results[0].title"), json!("First"));
        assert_eq!(resolve_dot_path(&value, "data.content"), json!("Body"));
    }

    #[test]
    #[serial(config)]
    fn build_tool_request_applies_mapped_args_from_completed_source() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let tool_args = build_tool_request_args(
            &context,
            PipelineDefinition {
                nodes: vec![
                    source_tool_node("source"),
                    target_tool_node(Some(json!({"static": "keep"}))),
                ],
                edges: vec![PipelineEdge {
                    source: "source".to_string(),
                    target: "tool".to_string(),
                    edge_type: EdgeType::Success,
                    mapping: Some(HashMap::from([
                        ("title".to_string(), "results.0.title".to_string()),
                        ("count".to_string(), "meta.count".to_string()),
                    ])),
                }],
            },
            HashMap::from([
                completed_node_state(
                    "source",
                    "pipeline-executor-mapped-source",
                    json!({
                        "results": [{"title": "Resolved"}],
                        "meta": {"count": 2}
                    }),
                ),
                pending_node_state("tool", "pipeline-executor-mapped-tool"),
            ]),
        );

        assert_eq!(
            tool_args,
            Some(json!({
                "static": "keep",
                "title": "Resolved",
                "count": 2
            }))
        );
    }

    #[test]
    #[serial(config)]
    fn build_tool_request_keeps_static_args_when_edge_has_no_mapping() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let tool_args = build_tool_request_args(
            &context,
            PipelineDefinition {
                nodes: vec![
                    source_tool_node("source"),
                    target_tool_node(Some(json!({"query": "static"}))),
                ],
                edges: vec![PipelineEdge {
                    source: "source".to_string(),
                    target: "tool".to_string(),
                    edge_type: EdgeType::Success,
                    mapping: None,
                }],
            },
            HashMap::from([
                completed_node_state(
                    "source",
                    "pipeline-executor-static-source",
                    json!({"query": "mapped"}),
                ),
                pending_node_state("tool", "pipeline-executor-static-tool"),
            ]),
        );

        assert_eq!(tool_args, Some(json!({"query": "static"})));
    }

    #[test]
    #[serial(config)]
    fn build_tool_request_sets_null_for_unresolved_paths() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let tool_args = build_tool_request_args(
            &context,
            PipelineDefinition {
                nodes: vec![source_tool_node("source"), target_tool_node(None)],
                edges: vec![PipelineEdge {
                    source: "source".to_string(),
                    target: "tool".to_string(),
                    edge_type: EdgeType::Success,
                    mapping: Some(HashMap::from([(
                        "missing".to_string(),
                        "results[9].title".to_string(),
                    )])),
                }],
            },
            HashMap::from([
                completed_node_state(
                    "source",
                    "pipeline-executor-null-source",
                    json!({"results": [{"title": "Resolved"}]}),
                ),
                pending_node_state("tool", "pipeline-executor-null-tool"),
            ]),
        );

        assert_eq!(tool_args, Some(json!({"missing": Value::Null})));
    }

    #[test]
    #[serial(config)]
    fn build_tool_request_mapping_overrides_static_values() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let tool_args = build_tool_request_args(
            &context,
            PipelineDefinition {
                nodes: vec![
                    source_tool_node("source"),
                    target_tool_node(Some(json!({"title": "Static", "mode": "keep"}))),
                ],
                edges: vec![PipelineEdge {
                    source: "source".to_string(),
                    target: "tool".to_string(),
                    edge_type: EdgeType::Success,
                    mapping: Some(HashMap::from([(
                        "title".to_string(),
                        "results[0].title".to_string(),
                    )])),
                }],
            },
            HashMap::from([
                completed_node_state(
                    "source",
                    "pipeline-executor-override-source",
                    json!({"results": [{"title": "Resolved"}]}),
                ),
                pending_node_state("tool", "pipeline-executor-override-tool"),
            ]),
        );

        assert_eq!(
            tool_args,
            Some(json!({
                "title": "Resolved",
                "mode": "keep"
            }))
        );
    }

    fn build_tool_request_args(
        context: &ToolContext,
        definition: PipelineDefinition,
        node_states: HashMap<String, NodeState>,
    ) -> Option<JsonValue> {
        let tool_node = definition
            .nodes
            .iter()
            .find(|node| node.id() == "tool")
            .expect("tool node");

        match build_node_execution_request(
            context,
            &definition,
            &node_states,
            tool_node,
            &DispatchedNode {
                node_id: "tool".to_string(),
                session_id: None,
            },
            "Ambient context",
        )
        .expect("build node execution request")
        {
            NodeExecutionRequest::Tool { tool_args, .. } => tool_args,
            other => panic!("expected tool execution request, got {other:?}"),
        }
    }

    fn source_tool_node(id: &str) -> PipelineNode {
        PipelineNode::Tool {
            id: id.to_string(),
            tool_name: "GetConfig".to_string(),
            tool_args: None,
            agent: None,
        }
    }

    fn target_tool_node(tool_args: Option<JsonValue>) -> PipelineNode {
        PipelineNode::Tool {
            id: "tool".to_string(),
            tool_name: "GetConfig".to_string(),
            tool_args,
            agent: None,
        }
    }

    fn completed_node_state(id: &str, session_id: &str, output: JsonValue) -> (String, NodeState) {
        (
            id.to_string(),
            NodeState {
                session_id: session_id.to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                output: Some(output.to_string()),
            },
        )
    }

    #[tokio::test]
    #[serial(config)]
    async fn executor_completes_single_node_pipeline() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let _path_guard = install_acpx_stub(&context.config.maenifold_root);
        let pipeline_id = "pipeline-executor-single";
        let workflow_session_id = "workflow-executor-single-root";

        write_workflow_definition(&context.config.assets_path, "test-workflow-a");

        write_stub_config(
            &context.config.maenifold_root,
            json!({
                "test-agent-root": { "text": "Single node completed [[step]]" }
            }),
        );

        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![PipelineNode::Workflow {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: Some("test-agent-root".to_string()),
                    prompt: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "root".to_string(),
                NodeState {
                    session_id: workflow_session_id.to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    output: None,
                },
            )]),
            8,
        );

        let result = execute_pipeline(
            &context,
            pipeline_id,
            "Execute the workflow",
            CancellationToken::new(),
        )
        .await
        .expect("executor should succeed");

        assert_eq!(
            result,
            ExecutionResult {
                completed: 1,
                abandoned: 0,
                status: "completed".to_string(),
            }
        );

        let node_states = read_pipeline_node_states(&context, pipeline_id);
        assert_eq!(node_states["root"].status, NodeStatus::Completed);
        assert_eq!(
            node_states["root"].output.as_deref(),
            Some("Single node completed [[step]]")
        );
        assert_eq!(
            read_workflow_status(&context, workflow_session_id),
            Some("completed".to_string())
        );
        assert_eq!(
            read_workflow_conclusion(&context, workflow_session_id),
            Some("Single node completed [[step]]".to_string())
        );
        let prompts = read_stub_prompts(&context.config.maenifold_root, "test-agent-root");
        assert_eq!(prompts.len(), 1);
        assert!(prompts[0].contains("Execute the workflow"));
        assert!(prompts[0].contains("Step 1/1"));
    }

    #[tokio::test]
    #[serial(config)]
    async fn executor_assembles_workflow_prompt_with_predecessor_context() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let _path_guard = install_acpx_stub(&context.config.maenifold_root);
        let pipeline_id = "pipeline-executor-workflow-prompt";

        write_workflow_definition(&context.config.assets_path, "test-workflow-a");
        write_workflow_definition(&context.config.assets_path, "test-workflow-b");

        write_stub_config(
            &context.config.maenifold_root,
            json!({
                "test-agent-a": { "text": "Upstream conclusion [[step]]" },
                "test-agent-b": { "text": "Downstream conclusion [[step]]" }
            }),
        );

        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![
                    PipelineNode::Workflow {
                        id: "a".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: Some("test-agent-a".to_string()),
                        prompt: None,
                    },
                    PipelineNode::Workflow {
                        id: "b".to_string(),
                        workflow_id: "test-workflow-b".to_string(),
                        acp_client: Some("test-agent-b".to_string()),
                        prompt: Some("Node-specific instruction".to_string()),
                    },
                ],
                edges: vec![PipelineEdge {
                    source: "a".to_string(),
                    target: "b".to_string(),
                    edge_type: EdgeType::Success,
                    mapping: None,
                }],
            },
            HashMap::from([
                pending_node_state("a", "workflow-executor-workflow-prompt-a"),
                pending_node_state("b", "workflow-executor-workflow-prompt-b"),
            ]),
            2,
        );

        let result = execute_pipeline(
            &context,
            pipeline_id,
            "Ambient context",
            CancellationToken::new(),
        )
        .await
        .expect("executor should succeed");

        assert_eq!(result.completed, 2);
        assert_eq!(result.abandoned, 0);
        assert_eq!(result.status, "completed");

        let prompts = read_stub_prompts(&context.config.maenifold_root, "test-agent-b");
        assert_eq!(prompts.len(), 1);
        assert!(prompts[0].contains("### From a\n\nUpstream conclusion [[step]]"));
        assert!(prompts[0].contains("Ambient context\n\nNode-specific instruction"));
        assert!(prompts[0].contains("Step 1/1"));
    }

    #[tokio::test]
    #[serial(config)]
    async fn executor_runs_prompt_and_thinking_nodes_and_captures_output() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let _path_guard = install_acpx_stub(&context.config.maenifold_root);
        let pipeline_id = "pipeline-executor-mixed";

        write_workflow_definition(&context.config.assets_path, "test-workflow-a");

        write_stub_config(
            &context.config.maenifold_root,
            json!({
                "test-agent-a": { "text": "Workflow output [[step]]" },
                "prompt-agent": { "text": "Prompt output [[note]]" },
                "think-agent": { "text": "DONE Thinking output [[idea]]" }
            }),
        );

        seed_pipeline_session_with_nodes(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![
                    PipelineNode::Workflow {
                        id: "a".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: Some("test-agent-a".to_string()),
                        prompt: None,
                    },
                    PipelineNode::Prompt {
                        id: "prompt".to_string(),
                        prompt: "Draft next action".to_string(),
                        agent: "prompt-agent".to_string(),
                        tool_mode: "default".to_string(),
                    },
                    PipelineNode::SequentialThinking {
                        id: "think".to_string(),
                        topic: "Plan follow-up".to_string(),
                        total_thoughts: 2,
                        analysis_type: Some("architecture".to_string()),
                        agent: "think-agent".to_string(),
                    },
                ],
                edges: vec![
                    PipelineEdge {
                        source: "a".to_string(),
                        target: "prompt".to_string(),
                        edge_type: EdgeType::Success,
                        mapping: None,
                    },
                    PipelineEdge {
                        source: "prompt".to_string(),
                        target: "think".to_string(),
                        edge_type: EdgeType::Success,
                        mapping: None,
                    },
                ],
            },
            HashMap::from([
                pending_node_state("a", "workflow-executor-mixed-a"),
                pending_node_state("prompt", "workflow-executor-mixed-prompt"),
                pending_node_state("think", "workflow-executor-mixed-think"),
            ]),
            3,
        );

        let result = execute_pipeline(
            &context,
            pipeline_id,
            "Ambient context",
            CancellationToken::new(),
        )
        .await
        .expect("executor should succeed");

        assert_eq!(
            result,
            ExecutionResult {
                completed: 3,
                abandoned: 0,
                status: "completed".to_string(),
            }
        );

        let node_states = read_pipeline_node_states(&context, pipeline_id);
        assert_eq!(
            node_states["a"].output.as_deref(),
            Some("Workflow output [[step]]")
        );
        assert_eq!(
            node_states["prompt"].output.as_deref(),
            Some("Prompt output [[note]]")
        );
        assert_eq!(
            node_states["think"].output.as_deref(),
            Some("DONE Thinking output [[idea]]")
        );

        let prompt_prompts = read_stub_prompts(&context.config.maenifold_root, "prompt-agent");
        assert_eq!(prompt_prompts.len(), 1);
        assert_eq!(
            prompt_prompts[0],
            "### From a\n\nWorkflow output [[step]]\n\nAmbient context\n\nDraft next action"
        );

        let think_prompts = read_stub_prompts(&context.config.maenifold_root, "think-agent");
        assert_eq!(think_prompts.len(), 1);
        assert!(think_prompts[0].starts_with(
            "Thought 1/~2 on: ### From prompt\n\nPrompt output [[note]]\n\nAmbient context\n\nPlan follow-up."
        ));
        assert!(think_prompts[0].contains("Include [[WikiLinks]]. Say DONE when complete."));
    }

    #[tokio::test]
    #[serial(config)]
    async fn executor_runs_tool_nodes_and_captures_tool_output() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-executor-tool";

        seed_pipeline_session_with_nodes(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![PipelineNode::Tool {
                    id: "tool".to_string(),
                    tool_name: "GetConfig".to_string(),
                    tool_args: Some(json!({})),
                    agent: None,
                }],
                edges: vec![],
            },
            HashMap::from([pending_node_state("tool", "workflow-executor-tool-node")]),
            1,
        );

        let result = execute_pipeline(
            &context,
            pipeline_id,
            "Ambient context should not affect tool execution",
            CancellationToken::new(),
        )
        .await
        .expect("executor should run tool node");

        assert_eq!(
            result,
            ExecutionResult {
                completed: 1,
                abandoned: 0,
                status: "completed".to_string(),
            }
        );

        let node_states = read_pipeline_node_states(&context, pipeline_id);
        let output = node_states["tool"]
            .output
            .as_deref()
            .expect("tool node output");
        let envelope: Value = serde_json::from_str(output).expect("tool output JSON");
        assert_eq!(envelope["success"].as_bool(), Some(true));
        assert!(
            envelope["data"]
                .as_str()
                .is_some_and(|text| text.contains("Maenifold Configuration:")),
            "unexpected tool output: {output}"
        );
    }

    #[tokio::test]
    #[serial(config)]
    async fn executor_handles_node_failure() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let _path_guard = install_acpx_stub(&context.config.maenifold_root);
        let pipeline_id = "pipeline-executor-failure";
        let workflow_session_id = "workflow-executor-failure-root";

        write_workflow_definition(&context.config.assets_path, "test-workflow-a");

        write_stub_config(
            &context.config.maenifold_root,
            json!({
                "failing-agent": { "error": "simulated failure" }
            }),
        );

        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![PipelineNode::Workflow {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: Some("failing-agent".to_string()),
                    prompt: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "root".to_string(),
                NodeState {
                    session_id: workflow_session_id.to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    output: None,
                },
            )]),
            8,
        );

        let result = execute_pipeline(
            &context,
            pipeline_id,
            "Execute the workflow",
            CancellationToken::new(),
        )
        .await
        .expect("executor should handle failures");

        assert_eq!(
            result,
            ExecutionResult {
                completed: 0,
                abandoned: 1,
                status: "abandoned".to_string(),
            }
        );

        let node_states = read_pipeline_node_states(&context, pipeline_id);
        assert_eq!(node_states["root"].status, NodeStatus::Abandoned);
        assert_eq!(
            read_workflow_status(&context, workflow_session_id),
            Some("abandoned".to_string())
        );
    }

    #[tokio::test]
    #[serial(config)]
    async fn executor_respects_cancellation() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let _path_guard = install_acpx_stub(&context.config.maenifold_root);
        let pipeline_id = "pipeline-executor-cancel";

        write_workflow_definition(&context.config.assets_path, "test-workflow-a");
        write_workflow_definition(&context.config.assets_path, "test-workflow-b");

        write_stub_config(
            &context.config.maenifold_root,
            json!({
                "test-agent-a": { "delay": 5.0, "text": "A [[step]]" },
                "test-agent-b": { "delay": 5.0, "text": "B [[step]]" }
            }),
        );

        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![
                    PipelineNode::Workflow {
                        id: "a".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: Some("test-agent-a".to_string()),
                        prompt: None,
                    },
                    PipelineNode::Workflow {
                        id: "b".to_string(),
                        workflow_id: "test-workflow-b".to_string(),
                        acp_client: Some("test-agent-b".to_string()),
                        prompt: None,
                    },
                ],
                edges: vec![],
            },
            HashMap::from([
                (
                    "a".to_string(),
                    NodeState {
                        session_id: "workflow-executor-cancel-a".to_string(),
                        status: NodeStatus::Pending,
                        attempts: 0,
                        output: None,
                    },
                ),
                (
                    "b".to_string(),
                    NodeState {
                        session_id: "workflow-executor-cancel-b".to_string(),
                        status: NodeStatus::Pending,
                        attempts: 0,
                        output: None,
                    },
                ),
            ]),
            2,
        );

        wait_for_stub_max(&context.config.maenifold_root, 0).await;

        let context = Arc::new(context);
        let cancel = CancellationToken::new();
        let cancel_for_task = cancel.clone();
        let task_context = context.clone();
        let handle = tokio::spawn(async move {
            execute_pipeline(
                &task_context,
                pipeline_id,
                "Execute the workflow",
                cancel_for_task,
            )
            .await
        });

        wait_for_stub_max(&context.config.maenifold_root, 2).await;
        cancel.cancel();

        let result = handle
            .await
            .expect("join executor task")
            .expect("executor should exit cleanly");

        assert_eq!(result.completed, 0);
        assert_eq!(result.abandoned, 2);
        assert_eq!(result.status, "cancelled");

        let node_states = read_pipeline_node_states(&context, pipeline_id);
        assert!(matches!(node_states["a"].status, NodeStatus::Abandoned));
        assert!(matches!(node_states["b"].status, NodeStatus::Abandoned));
    }

    #[tokio::test]
    #[serial(config)]
    async fn executor_respects_max_concurrency() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let _path_guard = install_acpx_stub(&context.config.maenifold_root);
        let pipeline_id = "pipeline-executor-concurrency";

        write_workflow_definition(&context.config.assets_path, "test-workflow-a");
        write_workflow_definition(&context.config.assets_path, "test-workflow-b");

        write_stub_config(
            &context.config.maenifold_root,
            json!({
                "test-agent-a": { "delay": 0.2, "text": "A [[step]]" },
                "test-agent-b": { "delay": 0.2, "text": "B [[step]]" },
                "test-agent-c": { "delay": 0.2, "text": "C [[step]]" },
                "test-agent-d": { "delay": 0.2, "text": "D [[step]]" }
            }),
        );

        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![
                    pipeline_node("a", "test-workflow-a"),
                    pipeline_node("b", "test-workflow-b"),
                    pipeline_node("c", "test-workflow-a"),
                    pipeline_node("d", "test-workflow-b"),
                ],
                edges: vec![],
            },
            HashMap::from([
                pending_node_state("a", "workflow-executor-concurrency-a"),
                pending_node_state("b", "workflow-executor-concurrency-b"),
                pending_node_state("c", "workflow-executor-concurrency-c"),
                pending_node_state("d", "workflow-executor-concurrency-d"),
            ]),
            2,
        );

        let result = execute_pipeline(
            &context,
            pipeline_id,
            "Execute the workflow",
            CancellationToken::new(),
        )
        .await
        .expect("executor should succeed");

        assert_eq!(result.completed, 4);
        assert_eq!(result.abandoned, 0);
        assert_eq!(result.status, "completed");
        assert_eq!(read_stub_max(&context.config.maenifold_root), 2);
    }

    #[tokio::test]
    #[serial(config)]
    async fn executor_cascades_through_dag() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let _path_guard = install_acpx_stub(&context.config.maenifold_root);
        let pipeline_id = "pipeline-executor-dag";

        write_workflow_definition(&context.config.assets_path, "test-workflow-a");
        write_workflow_definition(&context.config.assets_path, "test-workflow-b");

        write_stub_config(
            &context.config.maenifold_root,
            json!({
                "test-agent-a": { "text": "A complete [[step]]" },
                "test-agent-b": { "text": "B complete [[step]]" },
                "test-agent-c": { "text": "C complete [[step]]" }
            }),
        );

        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![
                    pipeline_node("a", "test-workflow-a"),
                    pipeline_node("b", "test-workflow-b"),
                    pipeline_node("c", "test-workflow-a"),
                ],
                edges: vec![
                    PipelineEdge {
                        source: "a".to_string(),
                        target: "b".to_string(),
                        edge_type: EdgeType::Success,
                        mapping: None,
                    },
                    PipelineEdge {
                        source: "b".to_string(),
                        target: "c".to_string(),
                        edge_type: EdgeType::Success,
                        mapping: None,
                    },
                ],
            },
            HashMap::from([
                pending_node_state("a", "workflow-executor-dag-a"),
                pending_node_state("b", "workflow-executor-dag-b"),
                pending_node_state("c", "workflow-executor-dag-c"),
            ]),
            3,
        );

        let result = execute_pipeline(
            &context,
            pipeline_id,
            "Execute the workflow",
            CancellationToken::new(),
        )
        .await
        .expect("executor should succeed");

        assert_eq!(result.completed, 3);
        assert_eq!(result.abandoned, 0);
        assert_eq!(result.status, "completed");

        let starts = read_stub_event_order(&context.config.maenifold_root);
        assert_eq!(
            starts,
            vec![
                "test-agent-a".to_string(),
                "test-agent-b".to_string(),
                "test-agent-c".to_string(),
            ]
        );

        let node_states = read_pipeline_node_states(&context, pipeline_id);
        assert!(
            node_states
                .values()
                .all(|state| matches!(state.status, NodeStatus::Completed))
        );
    }

    fn pipeline_node(id: &str, workflow_id: &str) -> PipelineNode {
        PipelineNode::Workflow {
            id: id.to_string(),
            workflow_id: workflow_id.to_string(),
            acp_client: Some(format!("test-agent-{id}")),
            prompt: None,
        }
    }

    fn write_workflow_definition(assets_path: &Path, workflow_id: &str) {
        let workflows_dir = assets_path.join("workflows");
        fs::create_dir_all(&workflows_dir).expect("workflows dir");
        let definition = json!({
            "steps": [{
                "id": "step-1",
                "name": "Step 1",
                "description": "Execute [[task]]"
            }]
        });
        fs::write(
            workflows_dir.join(format!("{workflow_id}.json")),
            serde_json::to_vec_pretty(&definition).expect("workflow json"),
        )
        .expect("write workflow definition");
    }

    fn pending_node_state(id: &str, session_id: &str) -> (String, NodeState) {
        (
            id.to_string(),
            NodeState {
                session_id: session_id.to_string(),
                status: NodeStatus::Pending,
                attempts: 0,
                output: None,
            },
        )
    }

    fn node_status_name(status: &NodeStatus) -> &'static str {
        match status {
            NodeStatus::Pending => "pending",
            NodeStatus::Ready => "ready",
            NodeStatus::Active => "active",
            NodeStatus::Completed => "completed",
            NodeStatus::Abandoned => "abandoned",
            NodeStatus::Stuck => "stuck",
        }
    }

    fn seed_pipeline_session_with_nodes(
        context: &ToolContext,
        pipeline_id: &str,
        definition: PipelineDefinition,
        node_states: HashMap<String, NodeState>,
        max_concurrency: u64,
    ) {
        let mut frontmatter = Frontmatter::new();
        frontmatter.insert(
            "title".to_string(),
            YamlValue::String(format!("Pipeline Session {pipeline_id}")),
        );
        frontmatter.insert(
            "permalink".to_string(),
            YamlValue::String(pipeline_id.to_string()),
        );
        frontmatter.insert(
            "type".to_string(),
            YamlValue::String("pipeline".to_string()),
        );
        frontmatter.insert(
            "status".to_string(),
            YamlValue::String("active".to_string()),
        );
        frontmatter.insert(
            "definition".to_string(),
            serde_yaml::to_value(&definition).expect("serialize definition"),
        );
        frontmatter.insert(
            "nodeStates".to_string(),
            serde_yaml::to_value(&node_states).expect("serialize node states"),
        );
        frontmatter.insert(
            "maxConcurrency".to_string(),
            serde_yaml::to_value(max_concurrency).expect("serialize concurrency"),
        );

        write_markdown(
            &pipeline_session_uri(pipeline_id),
            Some(&frontmatter),
            "# Pipeline Session\n",
            &context.config,
        )
        .expect("write pipeline session");

        for node in &definition.nodes {
            let state = node_states.get(node.id()).expect("node state");
            let workflow_ref = node
                .workflow_id()
                .map(str::to_string)
                .unwrap_or_else(|| format!("pipeline-{}-{}", node.node_type_name(), node.id()));
            let mut workflow_frontmatter = initial_workflow_frontmatter(
                &state.session_id,
                std::slice::from_ref(&workflow_ref),
                node_status_name(&state.status),
            );
            workflow_frontmatter.insert(
                "pipelineId".to_string(),
                YamlValue::String(pipeline_id.to_string()),
            );
            workflow_frontmatter.insert(
                "nodeId".to_string(),
                YamlValue::String(node.id().to_string()),
            );
            workflow_frontmatter.insert("workflowId".to_string(), YamlValue::String(workflow_ref));

            write_markdown(
                &workflow_session_uri(&state.session_id),
                Some(&workflow_frontmatter),
                "# Workflow Session\n",
                &context.config,
            )
            .expect("write node session");
        }
    }

    fn install_acpx_stub(root: &Path) -> PathGuard {
        let bin_dir = root.join("bin");
        fs::create_dir_all(&bin_dir).expect("create bin dir");
        let stub_path = bin_dir.join("acpx");
        fs::write(&stub_path, acpx_stub_script()).expect("write acpx stub");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mut permissions = fs::metadata(&stub_path).expect("metadata").permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&stub_path, permissions).expect("chmod");
        }

        prepend_path(&bin_dir)
    }

    fn acpx_stub_script() -> &'static str {
        r#"#!/usr/bin/env python3
import fcntl
import json
import os
import sys
import time

ROOT = os.getcwd()
CONFIG_PATH = os.path.join(ROOT, ".executor-stub-config.json")
STATE_PATH = os.path.join(ROOT, ".executor-stub-state.json")
LOCK_PATH = os.path.join(ROOT, ".executor-stub.lock")

def read_json(path, default):
    try:
        with open(path, "r", encoding="utf-8") as handle:
            return json.load(handle)
    except Exception:
        return default

def write_state(state):
    with open(STATE_PATH, "w", encoding="utf-8") as handle:
        json.dump(state, handle)

def mutate_state(mutator):
    with open(LOCK_PATH, "a+", encoding="utf-8") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        state = read_json(STATE_PATH, {"current": 0, "max": 0, "events": []})
        mutator(state)
        write_state(state)
        fcntl.flock(lock, fcntl.LOCK_UN)

args = sys.argv[1:]
prompt = args[-1] if args else ""
session = ""
agent = ""
if "-s" in args:
    index = args.index("-s")
    if index + 1 < len(args):
        session = args[index + 1]
    if index > 0:
        agent = args[index - 1]

config = read_json(CONFIG_PATH, {})
behavior = config.get(session) or config.get(agent) or config.get("*") or {}

mutate_state(lambda state: (
    state.__setitem__("current", state.get("current", 0) + 1),
    state.__setitem__("max", max(state.get("max", 0), state.get("current", 0))),
    state.setdefault("events", []).append({"event": "start", "session": session, "agent": agent, "prompt": prompt})
))

try:
    delay = float(behavior.get("delay", 0))
    if delay > 0:
        time.sleep(delay)

    if "stderr" in behavior:
        print(behavior["stderr"], file=sys.stderr)

    if "error" in behavior:
        print(json.dumps({"type": "error", "message": behavior["error"]}))
    else:
        print(json.dumps({"type": "text", "text": behavior.get("text", "")}))

    sys.exit(int(behavior.get("exitCode", 0)))
finally:
    mutate_state(lambda state: (
        state.__setitem__("current", max(0, state.get("current", 0) - 1)),
        state.setdefault("events", []).append({"event": "end", "session": session})
    ))
"#
    }

    fn write_stub_config(root: &Path, config: Value) {
        fs::write(
            root.join(".executor-stub-config.json"),
            serde_json::to_vec_pretty(&config).expect("serialize stub config"),
        )
        .expect("write stub config");
    }

    async fn wait_for_stub_max(root: &Path, expected: u64) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if read_stub_max(root) >= expected {
                return;
            }
            sleep(Duration::from_millis(25)).await;
        }
        panic!("stub max concurrency never reached {expected}");
    }

    fn read_stub_state(root: &Path) -> Value {
        let state_path = root.join(".executor-stub-state.json");
        if !state_path.exists() {
            return json!({
                "current": 0,
                "max": 0,
                "events": []
            });
        }

        serde_json::from_slice(&fs::read(&state_path).expect("read stub state"))
            .expect("parse stub state")
    }

    fn read_stub_max(root: &Path) -> u64 {
        read_stub_state(root)
            .get("max")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    }

    fn read_stub_event_order(root: &Path) -> Vec<String> {
        read_stub_state(root)
            .get("events")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|event| event.get("event").and_then(Value::as_str) == Some("start"))
            .filter_map(|event| {
                event
                    .get("agent")
                    .and_then(Value::as_str)
                    .filter(|agent| !agent.is_empty())
                    .map(str::to_string)
            })
            .collect()
    }

    fn read_stub_prompts(root: &Path, agent: &str) -> Vec<String> {
        read_stub_state(root)
            .get("events")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|event| event.get("event").and_then(Value::as_str) == Some("start"))
            .filter(|event| event.get("agent").and_then(Value::as_str) == Some(agent))
            .filter_map(|event| {
                event
                    .get("prompt")
                    .and_then(Value::as_str)
                    .filter(|prompt| !prompt.is_empty())
                    .map(str::to_string)
            })
            .collect()
    }

    fn read_pipeline_node_states(
        context: &ToolContext,
        pipeline_id: &str,
    ) -> HashMap<String, NodeState> {
        let (frontmatter, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline");
        let frontmatter = frontmatter.expect("pipeline frontmatter");
        frontmatter_value(&frontmatter, "nodeStates").expect("node states")
    }

    fn read_workflow_status(context: &ToolContext, session_id: &str) -> Option<String> {
        let (frontmatter, _, _) = read_markdown(&workflow_session_uri(session_id), &context.config)
            .expect("read workflow");
        frontmatter
            .expect("workflow frontmatter")
            .get("status")
            .and_then(YamlValue::as_str)
            .map(str::to_string)
    }

    fn read_workflow_conclusion(context: &ToolContext, session_id: &str) -> Option<String> {
        let (frontmatter, _, _) = read_markdown(&workflow_session_uri(session_id), &context.config)
            .expect("read workflow");
        frontmatter
            .expect("workflow frontmatter")
            .get("conclusion")
            .and_then(YamlValue::as_str)
            .map(str::to_string)
    }

    fn prepend_path(path: &Path) -> PathGuard {
        let original = std::env::var_os("PATH");
        let updated = match &original {
            Some(existing) => {
                let mut parts = vec![path.to_path_buf()];
                parts.extend(std::env::split_paths(existing));
                std::env::join_paths(parts).expect("join PATH")
            }
            None => std::env::join_paths([path]).expect("join PATH"),
        };

        // SAFETY: tests serialize PATH mutation with #[serial(config)] and restore it on drop.
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
            match &self.original {
                Some(path) => {
                    // SAFETY: this restores PATH to its pre-test value.
                    unsafe {
                        std::env::set_var("PATH", path);
                    }
                }
                None => {
                    // SAFETY: this restores the pre-test absence of PATH.
                    unsafe {
                        std::env::remove_var("PATH");
                    }
                }
            }
        }
    }
}
