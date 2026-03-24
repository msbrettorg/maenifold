use crate::error::{MaenifoldError, Result};
use crate::markdown::{extract_h2_sections, read_markdown};
use crate::sessions::workflow_session_uri;
use crate::thinking::sequential_session_uri;
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use regex::Regex;
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;
use std::collections::HashMap;

use super::session_io::{derive_pipeline_node_states, frontmatter_value, read_pipeline_session};
use super::supervisor::supervisor_status;
#[cfg(test)]
use super::test_support;
use super::{
    EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineNode, compute_pipeline_status,
    compute_ready_set,
};
use super::{executor::assemble_node_prompt, executor::predecessor_context_content};

pub(super) fn view_pipeline_session(context: &ToolContext, session_id: &str) -> Result<String> {
    let (_, frontmatter, _) = read_pipeline_session(context, session_id)?;
    let definition: PipelineDefinition = frontmatter_value(&frontmatter, "definition")?;
    let node_states =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;
    let ambient_prompt = frontmatter
        .get("prompt")
        .and_then(YamlValue::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let result = build_pipeline_view_result(
        context,
        session_id,
        &definition,
        &node_states,
        ambient_prompt,
    )?;

    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}

pub(super) fn node_ids_with_status(
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    expected_status: NodeStatus,
) -> Result<Vec<String>> {
    definition
        .nodes
        .iter()
        .map(|node| {
            let state = node_states.get(node.id()).ok_or_else(|| {
                MaenifoldError::Message(format!(
                    "ERROR: Pipeline session missing node state for '{}'",
                    node.id()
                ))
            })?;
            Ok((node.id().to_string(), state.status == expected_status))
        })
        .filter_map(|entry| match entry {
            Ok((node_id, true)) => Some(Ok(node_id)),
            Ok((_, false)) => None,
            Err(error) => Some(Err(error)),
        })
        .collect()
}

pub(super) fn build_pipeline_view_result(
    context: &ToolContext,
    session_id: &str,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    ambient_prompt: Option<&str>,
) -> Result<Value> {
    let ready = compute_ready_set(definition, node_states);
    let status = compute_pipeline_status(node_states);
    let active = node_ids_with_status(definition, node_states, NodeStatus::Active)?;
    let completed = node_ids_with_status(definition, node_states, NodeStatus::Completed)?;
    let nodes = pipeline_nodes_json(
        context,
        definition,
        node_states,
        ambient_prompt.unwrap_or(""),
    )?;
    let edges = pipeline_edges_json(definition);
    let mermaid = render_pipeline_mermaid(definition, node_states)?;
    let running = supervisor_status(session_id).is_some();

    Ok(json!({
        "pipelineId": session_id,
        "status": status,
        "nodes": nodes,
        "edges": edges,
        "ready": ready,
        "active": active,
        "completed": completed,
        "mermaid": mermaid,
        "running": running,
    }))
}

fn pipeline_nodes_json(
    context: &ToolContext,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    ambient_prompt: &str,
) -> Result<Vec<Value>> {
    definition
        .nodes
        .iter()
        .map(|node| {
            let state = node_states.get(node.id()).ok_or_else(|| {
                MaenifoldError::Message(format!(
                    "ERROR: Pipeline session missing node state for '{}'",
                    node.id()
                ))
            })?;
            let mut node_json = json!({
                "nodeId": node.id(),
                "sessionId": state.session_id,
                "status": state.status,
                "attempts": state.attempts,
                "output": state.output,
                "type": node.node_type_name(),
            });
            add_pipeline_node_view_fields(&mut node_json, node, state, context);
            let assembled_prompt = assembled_prompt_for_view(
                context,
                definition,
                node_states,
                node,
                state,
                ambient_prompt,
            )?;
            if let Some(node_object) = node_json.as_object_mut() {
                node_object.insert(
                    "assembled_prompt".to_string(),
                    assembled_prompt.map(Value::String).unwrap_or(Value::Null),
                );

                let workflow_steps = if matches!(node, PipelineNode::Workflow { .. })
                    && matches!(state.status, NodeStatus::Active | NodeStatus::Completed)
                {
                    parse_workflow_steps(context, &state.session_id)
                } else {
                    None
                };
                node_object.insert(
                    "workflow_steps".to_string(),
                    workflow_steps.map(Value::Array).unwrap_or(Value::Null),
                );

                // T-64.6: RTM FR-23 — thinking trace for SequentialThinking nodes.
                let thinking_trace = if matches!(node, PipelineNode::SequentialThinking { .. })
                    && matches!(state.status, NodeStatus::Active | NodeStatus::Completed)
                {
                    parse_thinking_trace(context, &state.session_id)
                        .map(Value::Array)
                        .unwrap_or(Value::Null)
                } else {
                    Value::Null
                };
                node_object.insert("thinking_trace".to_string(), thinking_trace);
            }
            Ok(node_json)
        })
        .collect()
}

fn assembled_prompt_for_view(
    context: &ToolContext,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    node: &PipelineNode,
    state: &NodeState,
    ambient_prompt: &str,
) -> Result<Option<String>> {
    if !matches!(state.status, NodeStatus::Active | NodeStatus::Completed) {
        return Ok(None);
    }

    if matches!(node, PipelineNode::Tool { .. }) {
        return Ok(None);
    }

    let predecessor_context =
        match predecessor_context_content(context, definition, node_states, node.id()) {
            Ok(context) => context,
            Err(_) => return Ok(None),
        };
    let has_predecessor_context = predecessor_context
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some();
    if !has_predecessor_context {
        return Ok(None);
    }

    Ok(Some(assemble_node_prompt(
        context,
        definition,
        node_states,
        node,
        ambient_prompt,
    )?))
}

fn pipeline_edges_json(definition: &PipelineDefinition) -> Vec<Value> {
    definition
        .edges
        .iter()
        .map(|edge| {
            json!({
                "source": edge.source,
                "target": edge.target,
                "type": edge_type_name(&edge.edge_type),
            })
        })
        .collect()
}

fn render_pipeline_mermaid(
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
) -> Result<String> {
    let mut lines = vec!["graph LR".to_string()];

    for node in &definition.nodes {
        let state = node_states.get(node.id()).ok_or_else(|| {
            MaenifoldError::Message(format!(
                "ERROR: Pipeline session missing node state for '{}'",
                node.id()
            ))
        })?;
        lines.push(render_pipeline_mermaid_node(node, &state.status));
    }

    for edge in &definition.edges {
        let arrow = match edge.edge_type {
            EdgeType::Always => "-.->",
            EdgeType::Success => "-->",
        };
        lines.push(format!("  {} {} {}", edge.source, arrow, edge.target));
    }

    Ok(lines.join("\n"))
}

fn add_pipeline_node_view_fields(
    node_json: &mut Value,
    node: &PipelineNode,
    _state: &NodeState,
    _context: &ToolContext,
) {
    let additional_fields = match node {
        PipelineNode::Workflow {
            workflow_id,
            acp_client,
            prompt,
            ..
        } => json!({
            "workflowId": workflow_id,
            "agent": acp_client,
            "prompt": prompt,
        }),
        PipelineNode::Prompt { prompt, agent, .. } => json!({
            "prompt": prompt,
            "agent": agent,
        }),
        PipelineNode::SequentialThinking {
            topic,
            total_thoughts,
            analysis_type,
            agent,
            ..
        } => json!({
            "topic": topic,
            "agent": agent,
            "totalThoughts": total_thoughts,
            "analysisType": analysis_type,
        }),
        PipelineNode::Tool {
            tool_name,
            tool_args,
            agent,
            ..
        } => json!({
            "agent": agent,
            "toolName": tool_name,
            "toolArgs": tool_args,
        }),
    };

    if let (Some(node_object), Some(additional_object)) =
        (node_json.as_object_mut(), additional_fields.as_object())
    {
        node_object.extend(additional_object.clone());
    }
}

fn render_pipeline_mermaid_node(node: &PipelineNode, status: &NodeStatus) -> String {
    let label = mermaid_node_label(node, status);
    let wrapped_label = match node {
        PipelineNode::Workflow { .. } => format!("[\"{label}\"]"),
        PipelineNode::Prompt { .. } => format!("(\"{label}\")"),
        PipelineNode::SequentialThinking { .. } => format!("{{{{\"{label}\"}}}}"),
        PipelineNode::Tool { .. } => format!(">{label}]"),
    };

    format!("  {}{}", node.id(), wrapped_label)
}

fn mermaid_node_label(node: &PipelineNode, status: &NodeStatus) -> String {
    let node_label = match node {
        PipelineNode::Workflow { .. } => format!(
            "wf: {}",
            node.workflow_id()
                .expect("workflow nodes always have workflow ids")
        ),
        PipelineNode::Prompt { .. } => {
            format!(
                "prompt: {}",
                truncated_prompt_label(node.prompt().unwrap_or_default())
            )
        }
        PipelineNode::SequentialThinking { topic, .. } => format!("think: {topic}"),
        PipelineNode::Tool { tool_name, .. } => format!("tool: {tool_name}"),
    };

    format!("{} {}", status_emoji(status), node_label)
}

fn truncated_prompt_label(prompt: &str) -> String {
    let preview: String = prompt.chars().take(40).collect();
    format!("{preview}...")
}

fn status_emoji(status: &NodeStatus) -> &'static str {
    match status {
        NodeStatus::Pending => "⏳",
        NodeStatus::Ready => "🟡",
        NodeStatus::Active => "🔄",
        NodeStatus::Completed => "✅",
        NodeStatus::Abandoned => "❌",
        NodeStatus::Stuck => "🚫",
    }
}

fn edge_type_name(edge_type: &EdgeType) -> &'static str {
    match edge_type {
        EdgeType::Success => "success",
        EdgeType::Always => "always",
    }
}

fn parse_workflow_steps(context: &ToolContext, session_id: &str) -> Option<Vec<Value>> {
    let uri = workflow_session_uri(session_id);
    let (frontmatter, content, _checksum) = read_markdown(&uri, &context.config).ok()?;
    let workflow_status = frontmatter
        .as_ref()
        .and_then(|frontmatter| frontmatter.get("status"))
        .and_then(YamlValue::as_str)
        .unwrap_or_default();
    let steps: Vec<(String, String)> = extract_h2_sections(&content)
        .into_iter()
        .filter(|(heading, _)| workflow_step_progress(heading).is_some())
        .collect();

    if steps.is_empty() {
        return None;
    }

    let last_idx = steps.len() - 1;
    Some(
        steps
            .into_iter()
            .enumerate()
            .map(|(index, (heading, body))| {
                let status = if heading.contains('❌') || body.contains('❌') {
                    "failed"
                } else if workflow_status.eq_ignore_ascii_case("completed")
                    || workflow_status.eq_ignore_ascii_case("complete")
                    || heading.contains('✅')
                    || body.contains('✅')
                    || index < last_idx
                {
                    "complete"
                } else {
                    "active"
                };

                json!({
                    "step_name": workflow_step_name(&heading, &body),
                    "status": status,
                    "output": (!body.trim().is_empty()).then_some(body),
                })
            })
            .collect(),
    )
}

fn workflow_step_progress(heading: &str) -> Option<(u32, u32)> {
    let step = heading.trim().strip_prefix("Step ")?;
    let (current, remainder) = step.split_once('/')?;
    let total = remainder
        .split(|ch: char| ch == ':' || ch.is_whitespace())
        .find(|part| !part.is_empty())?;
    Some((current.parse().ok()?, total.parse().ok()?))
}

fn workflow_step_name(heading: &str, body: &str) -> String {
    heading
        .split_once(':')
        .map(|(_, name)| name.trim())
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            serde_json::from_str::<Value>(body).ok().and_then(|step| {
                step.get("name")
                    .and_then(Value::as_str)
                    .or_else(|| step.get("description").and_then(Value::as_str))
                    .or_else(|| step.get("id").and_then(Value::as_str))
                    .map(|value| value.lines().next().unwrap_or(value).trim().to_string())
            })
        })
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| heading.trim().to_string())
}

// T-64.6: RTM FR-23 — Parse thinking trace from a sequential thinking session markdown file.
fn parse_thinking_trace(context: &ToolContext, session_id: &str) -> Option<Vec<Value>> {
    let uri = sequential_session_uri(session_id);
    let (_, content, _) = read_markdown(&uri, &context.config).ok()?;
    let sections = extract_h2_sections(&content);
    let re = Regex::new(
        r"^Thought (\d+)/(\d+) \[([^\]]*)\](?:\s*\(Revises:\s*(\d+)\)|\s*\(Branch:\s*([^)]+)\))?$",
    )
    .ok()?;

    Some(
        sections
            .iter()
            .filter(|(heading, _)| !heading.starts_with("Conclusion"))
            .filter_map(|(heading, body)| {
                let caps = re.captures(heading)?;
                let thought_number: u64 = caps.get(1)?.as_str().parse().ok()?;
                let total_thoughts: u64 = caps.get(2)?.as_str().parse().ok()?;
                let agent = caps.get(3).map(|m| m.as_str().to_string());
                let revises_thought: Option<u64> =
                    caps.get(4).and_then(|m| m.as_str().parse().ok());
                let branch_id = caps.get(5).map(|m| m.as_str().to_string());
                Some(json!({
                    "thought_number": thought_number,
                    "total_thoughts": total_thoughts,
                    "text": body,
                    "is_revision": revises_thought.is_some(),
                    "revises_thought": revises_thought,
                    "branch_id": branch_id,
                    "agent": agent,
                }))
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::super::handler::pipeline;
    use super::*;
    use crate::markdown::append_h2_section;
    use serde_json::Value;
    use serial_test::serial;
    use std::collections::HashMap;

    use super::test_support::*;

    #[test]
    #[serial(config)]
    fn pipeline_view_returns_structured_json_and_mermaid() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let session_id = "pipeline-1735689600000";
        let definition = seed_pipeline_view_session(&context, session_id, "pipeline");

        let output = pipeline(&context, &json!({ "sessionId": session_id, "view": true }))
            .expect("view pipeline");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");

        assert_eq!(
            data.get("pipelineId").and_then(Value::as_str),
            Some(session_id)
        );
        assert_eq!(data.get("status").and_then(Value::as_str), Some("active"));
        assert_eq!(
            data.get("ready").and_then(Value::as_array),
            Some(&vec![Value::String("child".to_string())])
        );
        assert_eq!(
            data.get("active").and_then(Value::as_array),
            Some(&vec![Value::String("cleanup".to_string())])
        );
        assert_eq!(
            data.get("completed").and_then(Value::as_array),
            Some(&vec![Value::String("root".to_string())])
        );

        let nodes = data
            .get("nodes")
            .and_then(Value::as_array)
            .expect("nodes array");
        assert_eq!(nodes.len(), definition.nodes.len());
        assert_eq!(nodes[0]["nodeId"], "root");
        assert_eq!(nodes[0]["type"], "workflow");
        assert_eq!(nodes[0]["workflowId"], "test-workflow-a");
        assert_eq!(nodes[0]["sessionId"], "workflow-1735689600000-root");
        assert_eq!(nodes[0]["status"], "completed");
        assert_eq!(nodes[0]["attempts"], 1);
        assert_eq!(nodes[1]["status"], "active");
        assert_eq!(nodes[2]["status"], "pending");

        let edges = data
            .get("edges")
            .and_then(Value::as_array)
            .expect("edges array");
        assert_eq!(edges.len(), definition.edges.len());
        assert_eq!(edges[0]["type"], "success");
        assert_eq!(edges[1]["type"], "always");

        let mermaid = data
            .get("mermaid")
            .and_then(Value::as_str)
            .expect("mermaid string");
        assert!(mermaid.starts_with("graph LR\n"));
        assert!(mermaid.contains("root[\"✅ wf: test-workflow-a\"]"));
        assert!(mermaid.contains("cleanup[\"🔄 wf: test-workflow-b\"]"));
        assert!(mermaid.contains("child[\"⏳ wf: test-workflow-b\"]"));
        assert!(mermaid.contains("root --> child"));
        assert!(mermaid.contains("root -.-> cleanup"));
        assert_eq!(data.get("running").and_then(Value::as_bool), Some(false));
        assert!(data.get("pid").is_none() || data.get("pid") == Some(&Value::Null));
        assert!(data.get("attempts").is_none() || data.get("attempts") == Some(&Value::Null));
    }

    #[test]
    #[serial(config)]
    fn pipeline_view_renders_mixed_node_types() {
        let definition = PipelineDefinition {
            nodes: vec![
                PipelineNode::Workflow {
                    id: "workflow-node".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                },
                PipelineNode::Prompt {
                    id: "prompt-node".to_string(),
                    prompt: "Summarize the pipeline status and propose next steps for the operator"
                        .to_string(),
                    agent: "agent-prompt".to_string(),
                    tool_mode: "default".to_string(),
                },
                PipelineNode::SequentialThinking {
                    id: "thinking-node".to_string(),
                    topic: "pipeline triage".to_string(),
                    total_thoughts: 3,
                    analysis_type: Some("diagnostic".to_string()),
                    agent: "agent-think".to_string(),
                },
                PipelineNode::Tool {
                    id: "tool-node".to_string(),
                    tool_name: "ReadMemory".to_string(),
                    tool_args: Some(json!({"identifier": "memory://build/test"})),
                    agent: None,
                },
            ],
            edges: vec![
                super::super::PipelineEdge {
                    source: "workflow-node".to_string(),
                    target: "prompt-node".to_string(),
                    edge_type: EdgeType::Success,
                    mapping: None,
                },
                super::super::PipelineEdge {
                    source: "prompt-node".to_string(),
                    target: "thinking-node".to_string(),
                    edge_type: EdgeType::Always,
                    mapping: None,
                },
                super::super::PipelineEdge {
                    source: "thinking-node".to_string(),
                    target: "tool-node".to_string(),
                    edge_type: EdgeType::Success,
                    mapping: None,
                },
            ],
        };
        let node_states = HashMap::from([
            (
                "workflow-node".to_string(),
                NodeState {
                    session_id: "workflow-session".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    output: None,
                },
            ),
            (
                "prompt-node".to_string(),
                NodeState {
                    session_id: "prompt-session".to_string(),
                    status: NodeStatus::Active,
                    attempts: 2,
                    output: Some("prompt output".to_string()),
                },
            ),
            (
                "thinking-node".to_string(),
                NodeState {
                    session_id: "thinking-session".to_string(),
                    status: NodeStatus::Ready,
                    attempts: 0,
                    output: None,
                },
            ),
            (
                "tool-node".to_string(),
                NodeState {
                    session_id: "tool-session".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    output: None,
                },
            ),
        ]);

        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let result = build_pipeline_view_result(
            &context,
            "pipeline-mixed-nodes",
            &definition,
            &node_states,
            Some("stay focused"),
        )
        .expect("build pipeline view");
        let nodes = result
            .get("nodes")
            .and_then(Value::as_array)
            .expect("nodes array");

        assert_eq!(nodes[0]["type"], "workflow");
        assert_eq!(nodes[0]["workflowId"], "test-workflow-a");
        assert_eq!(nodes[0]["agent"], Value::Null);
        assert_eq!(nodes[0]["prompt"], Value::Null);
        assert!(nodes[0].get("topic").is_none());
        assert_eq!(nodes[0]["status"], "completed");
        assert_eq!(nodes[0]["attempts"], 1);
        assert_eq!(nodes[0]["output"], Value::Null);
        assert_eq!(nodes[0]["workflow_steps"], Value::Null);

        assert_eq!(nodes[1]["type"], "prompt");
        assert_eq!(nodes[1]["agent"], "agent-prompt");
        assert_eq!(
            nodes[1]["prompt"],
            "Summarize the pipeline status and propose next steps for the operator"
        );
        assert!(nodes[1].get("workflowId").is_none());
        assert!(nodes[1].get("topic").is_none());
        assert_eq!(nodes[1]["status"], "active");
        assert_eq!(nodes[1]["attempts"], 2);
        assert_eq!(nodes[1]["output"], "prompt output");
        assert_eq!(nodes[1]["assembled_prompt"], Value::Null);
        assert_eq!(nodes[1]["workflow_steps"], Value::Null);

        assert_eq!(nodes[2]["type"], "sequentialThinking");
        assert_eq!(nodes[2]["agent"], "agent-think");
        assert_eq!(nodes[2]["topic"], "pipeline triage");
        assert_eq!(nodes[2]["totalThoughts"], 3);
        assert_eq!(nodes[2]["analysisType"], "diagnostic");
        assert!(nodes[2].get("workflowId").is_none());
        assert_eq!(nodes[2]["status"], "ready");
        assert_eq!(nodes[2]["attempts"], 0);
        assert_eq!(nodes[2]["output"], Value::Null);
        assert_eq!(nodes[2]["assembled_prompt"], Value::Null);
        assert_eq!(nodes[2]["workflow_steps"], Value::Null);

        assert_eq!(nodes[3]["type"], "tool");
        assert_eq!(nodes[3]["agent"], Value::Null);
        assert_eq!(nodes[3]["toolName"], "ReadMemory");
        assert_eq!(
            nodes[3]["toolArgs"],
            json!({"identifier": "memory://build/test"})
        );
        assert!(nodes[3].get("workflowId").is_none());
        assert_eq!(nodes[3]["status"], "pending");
        assert_eq!(nodes[3]["attempts"], 0);
        assert_eq!(nodes[3]["output"], Value::Null);
        assert_eq!(nodes[3]["assembled_prompt"], Value::Null);
        assert_eq!(nodes[3]["workflow_steps"], Value::Null);

        let mermaid = result
            .get("mermaid")
            .and_then(Value::as_str)
            .expect("mermaid string");
        assert!(mermaid.contains("workflow-node[\"✅ wf: test-workflow-a\"]"));
        assert!(
            mermaid.contains(
                "prompt-node(\"🔄 prompt: Summarize the pipeline status and propos...\")"
            )
        );
        assert!(mermaid.contains("thinking-node{{\"🟡 think: pipeline triage\"}}"));
        assert!(mermaid.contains("tool-node>⏳ tool: ReadMemory]"));
        assert!(mermaid.contains("workflow-node --> prompt-node"));
        assert!(mermaid.contains("prompt-node -.-> thinking-node"));
        assert!(mermaid.contains("thinking-node --> tool-node"));
    }

    #[test]
    #[serial(config)]
    fn pipeline_view_includes_assembled_prompt_for_active_and_completed_acp_nodes() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let definition = PipelineDefinition {
            nodes: vec![
                PipelineNode::Workflow {
                    id: "workflow-node".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: Some("agent-workflow".to_string()),
                    prompt: Some("Summarize the upstream output".to_string()),
                },
                PipelineNode::Prompt {
                    id: "prompt-node".to_string(),
                    prompt: "Draft operator guidance".to_string(),
                    agent: "agent-prompt".to_string(),
                    tool_mode: "default".to_string(),
                },
                PipelineNode::SequentialThinking {
                    id: "thinking-node".to_string(),
                    topic: "diagnose what happened".to_string(),
                    total_thoughts: 4,
                    analysis_type: Some("diagnostic".to_string()),
                    agent: "agent-think".to_string(),
                },
                PipelineNode::Tool {
                    id: "tool-node".to_string(),
                    tool_name: "ReadMemory".to_string(),
                    tool_args: None,
                    agent: None,
                },
            ],
            edges: vec![
                super::super::PipelineEdge {
                    source: "workflow-node".to_string(),
                    target: "prompt-node".to_string(),
                    edge_type: EdgeType::Success,
                    mapping: None,
                },
                super::super::PipelineEdge {
                    source: "prompt-node".to_string(),
                    target: "thinking-node".to_string(),
                    edge_type: EdgeType::Success,
                    mapping: None,
                },
                super::super::PipelineEdge {
                    source: "thinking-node".to_string(),
                    target: "tool-node".to_string(),
                    edge_type: EdgeType::Success,
                    mapping: None,
                },
            ],
        };
        let node_states = HashMap::from([
            (
                "workflow-node".to_string(),
                NodeState {
                    session_id: "workflow-session".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    output: Some("workflow conclusion".to_string()),
                },
            ),
            (
                "prompt-node".to_string(),
                NodeState {
                    session_id: "prompt-session".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    output: Some("prompt answer".to_string()),
                },
            ),
            (
                "thinking-node".to_string(),
                NodeState {
                    session_id: "thinking-session".to_string(),
                    status: NodeStatus::Active,
                    attempts: 1,
                    output: None,
                },
            ),
            (
                "tool-node".to_string(),
                NodeState {
                    session_id: "tool-session".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    output: Some("tool output".to_string()),
                },
            ),
        ]);

        let result = build_pipeline_view_result(
            &context,
            "pipeline-assembled-prompt",
            &definition,
            &node_states,
            Some("Keep the response concise"),
        )
        .expect("build pipeline view");
        let nodes = result
            .get("nodes")
            .and_then(Value::as_array)
            .expect("nodes array");

        assert_eq!(nodes[0]["assembled_prompt"], Value::Null);
        assert_eq!(
            nodes[1]["assembled_prompt"],
            "### From workflow-node\n\nworkflow conclusion\n\nKeep the response concise\n\nDraft operator guidance"
        );
        assert_eq!(
            nodes[2]["assembled_prompt"],
            "### From prompt-node\n\nprompt answer\n\nKeep the response concise\n\ndiagnose what happened"
        );
        assert_eq!(nodes[3]["assembled_prompt"], Value::Null);
    }

    #[test]
    #[serial(config)]
    fn pipeline_view_returns_workflow_steps_for_active_and_completed_nodes() {
        use crate::sessions::workflow_session_uri;

        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);

        // Build a three-node pipeline: completed → active → pending
        let definition = PipelineDefinition {
            nodes: vec![
                PipelineNode::Workflow {
                    id: "node-done".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                },
                PipelineNode::Workflow {
                    id: "node-active".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: None,
                    prompt: None,
                },
                PipelineNode::Workflow {
                    id: "node-pending".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                },
            ],
            edges: vec![
                super::super::PipelineEdge {
                    source: "node-done".to_string(),
                    target: "node-active".to_string(),
                    edge_type: EdgeType::Success,
                    mapping: None,
                },
                super::super::PipelineEdge {
                    source: "node-active".to_string(),
                    target: "node-pending".to_string(),
                    edge_type: EdgeType::Success,
                    mapping: None,
                },
            ],
        };

        let node_states = HashMap::from([
            (
                "node-done".to_string(),
                NodeState {
                    session_id: "workflow-1735689600000-done".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    output: Some("all done".to_string()),
                },
            ),
            (
                "node-active".to_string(),
                NodeState {
                    session_id: "workflow-1735689600000-active".to_string(),
                    status: NodeStatus::Active,
                    attempts: 1,
                    output: None,
                },
            ),
            (
                "node-pending".to_string(),
                NodeState {
                    session_id: "workflow-1735689600000-pending".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    output: None,
                },
            ),
        ]);

        // Seed pipeline + workflow sessions via shared helper
        seed_pipeline_session_with_workflows(
            &context,
            "pipeline-wf-steps",
            definition.clone(),
            node_states.clone(),
            8,
        );

        let completed_uri = workflow_session_uri("workflow-1735689600000-done");
        append_h2_section(
            &completed_uri,
            "Step 1/2",
            "{\n      \"id\": \"research\",\n      \"name\": \"Research evidence\",\n      \"description\": \"Collect the supporting material\"\n    }",
            &context.config,
        )
        .expect("append completed step 1");
        append_h2_section(
            &completed_uri,
            "Step 2/2",
            "{\n      \"id\": \"analysis\",\n      \"name\": \"Analyze findings\",\n      \"description\": \"Synthesize the research into recommendations\"\n    }",
            &context.config,
        )
        .expect("append completed step 2");
        append_h2_section(
            &completed_uri,
            "Conclusion",
            "Final summary of the workflow.",
            &context.config,
        )
        .expect("append conclusion");

        let active_uri = workflow_session_uri("workflow-1735689600000-active");
        append_h2_section(
            &active_uri,
            "Step 1/3",
            "{\n      \"id\": \"gather\",\n      \"name\": \"Gather data\",\n      \"description\": \"Collect metrics from all sources\"\n    }",
            &context.config,
        )
        .expect("append active step 1");
        append_h2_section(
            &active_uri,
            "Step 2/3",
            "{\n      \"id\": \"validate\",\n      \"name\": \"Validate input\",\n      \"description\": \"❌ Validation failed for dataset B\"\n    }",
            &context.config,
        )
        .expect("append active step 2");
        append_h2_section(
            &active_uri,
            "Step 3/3",
            "{\n      \"id\": \"transform\",\n      \"name\": \"Transform data\",\n      \"description\": \"Apply the normalized transformation\"\n    }",
            &context.config,
        )
        .expect("append active step 3");

        // Build the view
        let result = build_pipeline_view_result(
            &context,
            "pipeline-wf-steps",
            &definition,
            &node_states,
            None,
        )
        .expect("build pipeline view");
        let nodes = result
            .get("nodes")
            .and_then(Value::as_array)
            .expect("nodes array");

        // --- Completed node: workflow_steps present and fully complete ---
        let done_node = nodes
            .iter()
            .find(|n| n["nodeId"] == "node-done")
            .expect("find node-done");
        let done_steps = done_node["workflow_steps"]
            .as_array()
            .expect("workflow_steps array for completed node");
        assert_eq!(done_steps.len(), 2);
        assert_eq!(done_steps[0]["step_name"], "Research evidence");
        assert_eq!(done_steps[0]["status"], "complete");
        assert!(
            done_steps[0]["output"]
                .as_str()
                .unwrap()
                .contains("\"research\"")
        );
        assert_eq!(done_steps[1]["step_name"], "Analyze findings");
        assert_eq!(done_steps[1]["status"], "complete");

        // --- Active node: workflow_steps present with failed + active state ---
        let active_node = nodes
            .iter()
            .find(|n| n["nodeId"] == "node-active")
            .expect("find node-active");
        let active_steps = active_node["workflow_steps"]
            .as_array()
            .expect("workflow_steps array for active node");
        assert_eq!(active_steps.len(), 3);
        assert_eq!(active_steps[0]["step_name"], "Gather data");
        assert_eq!(active_steps[0]["status"], "complete");
        assert_eq!(active_steps[1]["step_name"], "Validate input");
        assert_eq!(active_steps[1]["status"], "failed");
        assert_eq!(active_steps[2]["step_name"], "Transform data");
        assert_eq!(active_steps[2]["status"], "active");
        assert!(
            active_steps[2]["output"]
                .as_str()
                .unwrap()
                .contains("\"transform\"")
        );

        let pending_node = nodes
            .iter()
            .find(|n| n["nodeId"] == "node-pending")
            .expect("find node-pending");
        assert_eq!(pending_node["workflow_steps"], Value::Null);
    }

    #[test]
    #[serial(config)]
    fn pipeline_view_includes_thinking_trace_for_st_nodes() {
        use crate::markdown::append_h2_section;
        use crate::thinking::sequential_session_uri;

        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        // session_id with parseable timestamp (2025-01-01 00:00:00 UTC).
        let completed_sid = "st-1735689600000-think";
        let pending_sid = "st-1735689600000-pending";

        let definition = PipelineDefinition {
            nodes: vec![
                PipelineNode::SequentialThinking {
                    id: "think-done".to_string(),
                    topic: "architecture review".to_string(),
                    total_thoughts: 3,
                    analysis_type: Some("architecture".to_string()),
                    agent: "agent-think".to_string(),
                },
                PipelineNode::SequentialThinking {
                    id: "think-pending".to_string(),
                    topic: "pending analysis".to_string(),
                    total_thoughts: 2,
                    analysis_type: None,
                    agent: "agent-think".to_string(),
                },
            ],
            edges: vec![super::super::PipelineEdge {
                source: "think-done".to_string(),
                target: "think-pending".to_string(),
                edge_type: EdgeType::Success,
                mapping: None,
            }],
        };

        let node_states = HashMap::from([
            (
                "think-done".to_string(),
                NodeState {
                    session_id: completed_sid.to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    output: Some("conclusion text".to_string()),
                },
            ),
            (
                "think-pending".to_string(),
                NodeState {
                    session_id: pending_sid.to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    output: None,
                },
            ),
        ]);

        // Create a mock sequential thinking session file.
        let uri = sequential_session_uri(completed_sid);
        append_h2_section(
            &uri,
            "Thought 1/3 [agent-think]",
            "First thought about [[architecture]].\n\n*2025-01-01 00:00:01 UTC*\n",
            &context.config,
        )
        .expect("write thought 1");
        append_h2_section(
            &uri,
            "Thought 2/3 [agent-think] (Revises: 1)",
            "Revised first thought.\n\n*2025-01-01 00:00:02 UTC*\n",
            &context.config,
        )
        .expect("write thought 2");
        append_h2_section(
            &uri,
            "Thought 3/3 [agent-think] (Branch: alt-path)",
            "Branched thought.\n\n*2025-01-01 00:00:03 UTC*\n",
            &context.config,
        )
        .expect("write thought 3");
        append_h2_section(&uri, "Conclusion", "Final conclusion.", &context.config)
            .expect("write conclusion");

        let result = build_pipeline_view_result(
            &context,
            "pipeline-thinking-trace",
            &definition,
            &node_states,
            None,
        )
        .expect("build pipeline view");

        let nodes = result
            .get("nodes")
            .and_then(Value::as_array)
            .expect("nodes array");

        // Completed ST node should have thinking_trace array.
        let completed_node = &nodes[0];
        let trace = completed_node
            .get("thinking_trace")
            .expect("thinking_trace present");
        let trace_array = trace.as_array().expect("thinking_trace is array");
        assert_eq!(trace_array.len(), 3, "Conclusion section must be excluded");

        // First thought — plain.
        assert_eq!(trace_array[0]["thought_number"], 1);
        assert_eq!(trace_array[0]["total_thoughts"], 3);
        assert_eq!(trace_array[0]["agent"], "agent-think");
        assert_eq!(trace_array[0]["is_revision"], false);
        assert_eq!(trace_array[0]["revises_thought"], Value::Null);
        assert_eq!(trace_array[0]["branch_id"], Value::Null);
        assert!(
            trace_array[0]["text"]
                .as_str()
                .unwrap()
                .contains("architecture")
        );

        // Second thought — revision.
        assert_eq!(trace_array[1]["thought_number"], 2);
        assert_eq!(trace_array[1]["is_revision"], true);
        assert_eq!(trace_array[1]["revises_thought"], 1);
        assert_eq!(trace_array[1]["branch_id"], Value::Null);

        // Third thought — branch.
        assert_eq!(trace_array[2]["thought_number"], 3);
        assert_eq!(trace_array[2]["is_revision"], false);
        assert_eq!(trace_array[2]["revises_thought"], Value::Null);
        assert_eq!(trace_array[2]["branch_id"], "alt-path");

        // Pending ST node should have null thinking_trace.
        let pending_node = &nodes[1];
        assert_eq!(
            pending_node.get("thinking_trace"),
            Some(&Value::Null),
            "pending node must have null thinking_trace"
        );
    }
}
