use crate::error::{MaenifoldError, Result};
use crate::markdown::{append_h2_section, read_markdown, write_markdown};
use crate::sessions::workflow_session_uri;
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use serde_yaml::Value as YamlValue;
use std::collections::{BTreeSet, HashMap};

use super::session_io::{
    NodeFile, allocate_node_session_id, derive_pipeline_node_states, frontmatter_value,
    persist_pipeline_state, read_pipeline_session, update_workflow_node_frontmatter,
    write_workflow_node_session,
};
#[cfg(test)]
use super::test_support;
use super::view::build_pipeline_view_result;
use super::{EdgeType, NodeState, NodeStatus, PipelineDefinition};

pub(super) fn update_pipeline_node_status(
    context: &ToolContext,
    session_id: &str,
    node_id: &str,
    status: &str,
    conclusion: Option<&str>,
) -> Result<String> {
    let (uri, mut frontmatter, pipeline_content) = read_pipeline_session(context, session_id)?;
    let definition: PipelineDefinition = frontmatter_value(&frontmatter, "definition")?;
    let mut node_states =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;
    definition
        .nodes
        .iter()
        .find(|node| node.id == node_id)
        .ok_or_else(|| {
            MaenifoldError::Message(format!(
                "ERROR: Pipeline definition missing node '{node_id}'"
            ))
        })?;
    let workflow_session_id = {
        let state = node_states.get(node_id).ok_or_else(|| {
            MaenifoldError::Message(format!(
                "ERROR: Pipeline session missing node state for '{node_id}'"
            ))
        })?;
        if state.status != NodeStatus::Active {
            return Err(MaenifoldError::Message(format!(
                "ERROR: Node '{node_id}' must be active to transition to {status}; current status is {:?}",
                state.status
            )));
        }
        state.session_id.clone()
    };

    match status {
        "completed" => {
            let conclusion = conclusion
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| {
                    MaenifoldError::Message(
                        "ERROR: Conclusion required when marking pipeline node completed"
                            .to_string(),
                    )
                })?;
            update_workflow_node_frontmatter(
                context,
                &workflow_session_id,
                "completed",
                Some(conclusion),
            )?;
            let state = node_states.get_mut(node_id).expect("checked above");
            state.status = NodeStatus::Completed;
            inject_predecessor_context(context, &definition, &node_states, node_id)?;
        }
        "abandoned" => {
            update_workflow_node_frontmatter(context, &workflow_session_id, "abandoned", None)?;
            let state = node_states.get_mut(node_id).expect("checked above");
            state.status = NodeStatus::Abandoned;
        }
        _ => {
            return Err(MaenifoldError::InvalidArgument(format!(
                "Unsupported pipeline node status '{status}'"
            )));
        }
    }

    persist_pipeline_state(
        &mut frontmatter,
        &node_states,
        &uri,
        &pipeline_content,
        context,
    )?;

    let result = build_pipeline_view_result(session_id, &definition, &node_states)?;
    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}

pub(super) fn restart_pipeline_node(
    context: &ToolContext,
    session_id: &str,
    node_id: &str,
) -> Result<String> {
    let (uri, mut frontmatter, pipeline_content) = read_pipeline_session(context, session_id)?;
    let definition: PipelineDefinition = frontmatter_value(&frontmatter, "definition")?;
    let mut node_states =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;
    let max_attempts = frontmatter
        .get("maxAttempts")
        .and_then(YamlValue::as_u64)
        .unwrap_or(3) as u32;
    let node = definition
        .nodes
        .iter()
        .find(|node| node.id == node_id)
        .ok_or_else(|| {
            MaenifoldError::Message(format!(
                "ERROR: Pipeline definition missing node '{node_id}'"
            ))
        })?;

    {
        let state = node_states.get_mut(node_id).ok_or_else(|| {
            MaenifoldError::Message(format!(
                "ERROR: Pipeline session missing node state for '{node_id}'"
            ))
        })?;

        if !matches!(state.status, NodeStatus::Abandoned | NodeStatus::Stuck) {
            return Err(MaenifoldError::Message(format!(
                "Node {node_id} is {} — only abandoned or stuck nodes can be restarted",
                node_status_name(&state.status)
            )));
        }

        if state.attempts >= max_attempts {
            state.status = NodeStatus::Stuck;
            persist_pipeline_state(
                &mut frontmatter,
                &node_states,
                &uri,
                &pipeline_content,
                context,
            )?;
            return Err(MaenifoldError::Message(format!(
                "Node {node_id} has reached max attempts ({max_attempts})"
            )));
        }

        state.session_id = allocate_node_session_id(context, node_id)?;
        state.status = NodeStatus::Pending;
        state.attempts += 1;
    }

    let state = node_states.get(node_id).expect("node state checked above");
    write_workflow_node_session(
        context,
        session_id,
        &NodeFile {
            session_id: state.session_id.clone(),
            node_id: node.id.clone(),
            workflow_id: node.workflow_id.clone(),
        },
    )?;
    inject_predecessor_context_for_node(context, &definition, &node_states, node_id)?;
    persist_pipeline_state(
        &mut frontmatter,
        &node_states,
        &uri,
        &pipeline_content,
        context,
    )?;

    let result = build_pipeline_view_result(session_id, &definition, &node_states)?;
    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
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

fn inject_predecessor_context(
    context: &ToolContext,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    completed_node_id: &str,
) -> Result<()> {
    let downstream_node_ids: BTreeSet<&str> = definition
        .edges
        .iter()
        .filter(|edge| {
            edge.source == completed_node_id
                && matches!(edge.edge_type, EdgeType::Success | EdgeType::Always)
        })
        .map(|edge| edge.target.as_str())
        .collect();

    for downstream_node_id in downstream_node_ids {
        let Some(content) =
            predecessor_context_content(context, definition, node_states, downstream_node_id)?
        else {
            continue;
        };

        let downstream_state = node_states.get(downstream_node_id).ok_or_else(|| {
            MaenifoldError::Message(format!(
                "ERROR: Pipeline session missing node state for '{downstream_node_id}'"
            ))
        })?;
        upsert_h2_section(
            context,
            &workflow_session_uri(&downstream_state.session_id),
            "Predecessor Context",
            &content,
        )?;
    }

    Ok(())
}

fn inject_predecessor_context_for_node(
    context: &ToolContext,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    node_id: &str,
) -> Result<()> {
    let Some(content) = predecessor_context_content(context, definition, node_states, node_id)?
    else {
        return Ok(());
    };
    let state = node_states.get(node_id).ok_or_else(|| {
        MaenifoldError::Message(format!(
            "ERROR: Pipeline session missing node state for '{node_id}'"
        ))
    })?;
    upsert_h2_section(
        context,
        &workflow_session_uri(&state.session_id),
        "Predecessor Context",
        &content,
    )
}

fn predecessor_context_content(
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

        let (frontmatter, _, _) = read_markdown(
            &workflow_session_uri(&upstream_state.session_id),
            &context.config,
        )?;
        let Some(last) = frontmatter
            .as_ref()
            .and_then(|frontmatter| frontmatter.get("last"))
            .and_then(YamlValue::as_str)
            .map(str::trim)
            .filter(|last| !last.is_empty())
        else {
            continue;
        };

        sections.push(format!("### From {}\n\n{}", edge.source, last));
    }

    if sections.is_empty() {
        Ok(None)
    } else {
        Ok(Some(sections.join("\n\n")))
    }
}

fn upsert_h2_section(context: &ToolContext, uri: &str, heading: &str, content: &str) -> Result<()> {
    let (frontmatter, existing_content, _) = read_markdown(uri, &context.config)?;
    let frontmatter = frontmatter.unwrap_or_default();

    if has_h2_section(&existing_content, heading) {
        let updated_content = replace_h2_section(&existing_content, heading, content)?;
        write_markdown(uri, Some(&frontmatter), &updated_content, &context.config)
    } else {
        append_h2_section(uri, heading, content, &context.config)
    }
}

fn has_h2_section(existing_content: &str, heading: &str) -> bool {
    existing_content.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("## ") && trimmed.trim_start_matches('#').trim() == heading
    })
}

fn replace_h2_section(existing_content: &str, heading: &str, new_content: &str) -> Result<String> {
    let mut line_start = 0usize;
    let mut section_start = None;
    let mut section_end = existing_content.len();

    for line in existing_content.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("# ") || trimmed.starts_with("## ") {
            let heading_text = trimmed.trim_end().trim_start_matches('#').trim();
            if section_start.is_some() {
                section_end = line_start;
                break;
            }
            if heading_text == heading {
                section_start = Some(line_start);
            }
        }
        line_start += line.len();
    }

    let section_start = section_start.ok_or_else(|| {
        MaenifoldError::Message(format!(
            "Section not found: '{heading}'. Cannot replace a section that doesn't exist."
        ))
    })?;

    let replacement = format!("## {heading}\n\n{}\n", new_content.trim_end());
    Ok(format!(
        "{}{}{}",
        &existing_content[..section_start],
        replacement,
        &existing_content[section_end..]
    ))
}

#[cfg(test)]
mod tests {
    use super::super::handler::pipeline;
    use super::*;
    use crate::markdown::{append_h2_section, file_exists, read_markdown, write_markdown};
    use crate::sessions::{pipeline_session_uri, workflow_session_uri};
    use serde_json::{Value, json};
    use serial_test::serial;

    use super::test_support::*;

    #[test]
    fn pipeline_status_completed_updates_node_and_returns_pipeline_view() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600500";
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600500-root".to_string(),
                    status: NodeStatus::Active,
                    attempts: 1,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600500-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);

        let root_uri = workflow_session_uri("workflow-1735689600500-root");
        let (root_fm, root_content, _) =
            read_markdown(&root_uri, &context.config).expect("read root session");
        let mut root_fm = root_fm.expect("root frontmatter");
        root_fm.insert(
            "last".to_string(),
            YamlValue::String("Step 1 Response: Root [[context]] summary".to_string()),
        );
        write_markdown(&root_uri, Some(&root_fm), &root_content, &context.config)
            .expect("update root session");

        let output = pipeline(
            &context,
            &json!({
                "sessionId": pipeline_id,
                "nodeId": "root",
                "status": "completed",
                "conclusion": "Root [[context]] conclusion"
            }),
        )
        .expect("complete pipeline node");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");

        assert_eq!(
            data.get("pipelineId").and_then(Value::as_str),
            Some(pipeline_id)
        );
        assert_eq!(data.get("status").and_then(Value::as_str), Some("active"));
        assert_eq!(
            data.get("ready").and_then(Value::as_array),
            Some(&vec![Value::String("child".to_string())])
        );
        assert_eq!(
            data.get("completed").and_then(Value::as_array),
            Some(&vec![Value::String("root".to_string())])
        );
        assert_eq!(
            data.get("active").and_then(Value::as_array),
            Some(&Vec::<Value>::new())
        );

        let (pipeline_fm, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline session");
        let pipeline_fm = pipeline_fm.expect("pipeline frontmatter");
        let updated_states: HashMap<String, NodeState> =
            frontmatter_value(&pipeline_fm, "nodeStates").expect("node states");
        assert_eq!(updated_states["root"].status, NodeStatus::Completed);
        assert_eq!(updated_states["child"].status, NodeStatus::Pending);

        let (root_fm, _, _) = read_markdown(&root_uri, &context.config).expect("read root session");
        let root_fm = root_fm.expect("root frontmatter");
        assert_eq!(
            root_fm.get("status").and_then(YamlValue::as_str),
            Some("completed")
        );
        assert_eq!(
            root_fm.get("conclusion").and_then(YamlValue::as_str),
            Some("Root [[context]] conclusion")
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_status_rejects_unsupported_status_values() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600550";
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600550-root".to_string(),
                    status: NodeStatus::Active,
                    attempts: 1,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600550-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);

        let error =
            update_pipeline_node_status(&context, pipeline_id, "root", "invalid_status", None)
                .expect_err("unsupported status should fail");

        assert!(
            error
                .to_string()
                .contains("Unsupported pipeline node status"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn pipeline_status_abandoned_enables_always_edge_ready_set() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600600";
        let definition = PipelineDefinition {
            nodes: vec![
                crate::workflow::pipeline::PipelineNode {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                },
                crate::workflow::pipeline::PipelineNode {
                    id: "cleanup".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: None,
                },
            ],
            edges: vec![crate::workflow::pipeline::PipelineEdge {
                source: "root".to_string(),
                target: "cleanup".to_string(),
                edge_type: EdgeType::Always,
            }],
        };
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600600-root".to_string(),
                    status: NodeStatus::Active,
                    attempts: 0,
                },
            ),
            (
                "cleanup".to_string(),
                NodeState {
                    session_id: "workflow-1735689600600-cleanup".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
        ]);
        seed_pipeline_session_with_workflows(&context, pipeline_id, definition, node_states, 8);

        let output = pipeline(
            &context,
            &json!({
                "sessionId": pipeline_id,
                "nodeId": "root",
                "status": "abandoned"
            }),
        )
        .expect("abandon pipeline node");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");

        assert_eq!(
            data.get("ready").and_then(Value::as_array),
            Some(&vec![Value::String("cleanup".to_string())])
        );
        assert_eq!(
            data.get("completed").and_then(Value::as_array),
            Some(&Vec::<Value>::new())
        );

        let cleanup_nodes = data
            .get("nodes")
            .and_then(Value::as_array)
            .expect("nodes array");
        assert_eq!(cleanup_nodes[0]["status"], "abandoned");
        assert_eq!(cleanup_nodes[1]["status"], "pending");

        let (root_fm, _, _) = read_markdown(
            &workflow_session_uri("workflow-1735689600600-root"),
            &context.config,
        )
        .expect("read root session");
        let root_fm = root_fm.expect("root frontmatter");
        assert_eq!(
            root_fm.get("status").and_then(YamlValue::as_str),
            Some("abandoned")
        );
    }

    #[test]
    fn pipeline_status_rejects_non_active_transitions() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600700";
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600700-root".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600700-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);

        let error = pipeline(
            &context,
            &json!({
                "sessionId": pipeline_id,
                "nodeId": "root",
                "status": "completed",
                "conclusion": "Should fail [[context]]"
            }),
        )
        .expect_err("non-active transition should fail");

        assert!(
            error
                .to_string()
                .contains("must be active to transition to completed"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn predecessor_context_replaces_existing_section_with_all_completed_upstreams() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600800";
        let definition = PipelineDefinition {
            nodes: vec![
                crate::workflow::pipeline::PipelineNode {
                    id: "root-a".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                },
                crate::workflow::pipeline::PipelineNode {
                    id: "root-b".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: None,
                },
                crate::workflow::pipeline::PipelineNode {
                    id: "child".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: None,
                },
            ],
            edges: vec![
                crate::workflow::pipeline::PipelineEdge {
                    source: "root-a".to_string(),
                    target: "child".to_string(),
                    edge_type: EdgeType::Success,
                },
                crate::workflow::pipeline::PipelineEdge {
                    source: "root-b".to_string(),
                    target: "child".to_string(),
                    edge_type: EdgeType::Always,
                },
            ],
        };
        let node_states = HashMap::from([
            (
                "root-a".to_string(),
                NodeState {
                    session_id: "workflow-1735689600800-root-a".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                },
            ),
            (
                "root-b".to_string(),
                NodeState {
                    session_id: "workflow-1735689600800-root-b".to_string(),
                    status: NodeStatus::Active,
                    attempts: 1,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600800-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
        ]);
        seed_pipeline_session_with_workflows(&context, pipeline_id, definition, node_states, 8);

        for (session_id, last_value) in [
            (
                "workflow-1735689600800-root-a",
                "Step 1 Response: Alpha [[context]]",
            ),
            (
                "workflow-1735689600800-root-b",
                "Step 1 Response: Beta [[context]]",
            ),
        ] {
            let uri = workflow_session_uri(session_id);
            let (frontmatter, content, _) =
                read_markdown(&uri, &context.config).expect("read upstream session");
            let mut frontmatter = frontmatter.expect("upstream frontmatter");
            frontmatter.insert(
                "last".to_string(),
                YamlValue::String(last_value.to_string()),
            );
            write_markdown(&uri, Some(&frontmatter), &content, &context.config)
                .expect("write upstream last");
        }

        let child_uri = workflow_session_uri("workflow-1735689600800-child");
        append_h2_section(
            &child_uri,
            "Predecessor Context",
            "### From root-a\n\nOld [[context]] only",
            &context.config,
        )
        .expect("seed predecessor context");
        append_h2_section(
            &child_uri,
            "Step 1/1",
            "Downstream [[context]] step",
            &context.config,
        )
        .expect("seed downstream step");

        pipeline(
            &context,
            &json!({
                "sessionId": pipeline_id,
                "nodeId": "root-b",
                "status": "completed",
                "conclusion": "Beta [[context]] conclusion"
            }),
        )
        .expect("complete second upstream");

        let (_, child_content, _) =
            read_markdown(&child_uri, &context.config).expect("read child session");
        assert_eq!(child_content.matches("## Predecessor Context").count(), 1);
        assert!(child_content.contains("### From root-a"));
        assert!(child_content.contains("Step 1 Response: Alpha [[context]]"));
        assert!(child_content.contains("### From root-b"));
        assert!(child_content.contains("Step 1 Response: Beta [[context]]"));
        assert!(
            !child_content.contains("Old [[context]] only"),
            "stale content should be gone"
        );
        assert!(child_content.contains("## Step 1/1"));
    }

    #[test]
    #[serial(config)]
    fn predecessor_context_returns_none_when_completed_upstream_has_no_last_field() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600850";
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600850-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600850-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
        ]);
        let definition = seed_dispatch_pipeline(&context, pipeline_id, node_states.clone(), 8);

        let root_uri = workflow_session_uri("workflow-1735689600850-root");
        let (root_frontmatter, root_content, _) =
            read_markdown(&root_uri, &context.config).expect("read root workflow");
        let mut root_frontmatter = root_frontmatter.expect("root frontmatter");
        root_frontmatter.remove("last");
        write_markdown(
            &root_uri,
            Some(&root_frontmatter),
            &root_content,
            &context.config,
        )
        .expect("rewrite root workflow without last");

        let content = predecessor_context_content(&context, &definition, &node_states, "child")
            .expect("predecessor context lookup");

        assert_eq!(content, None);
    }

    #[test]
    fn pipeline_replace_h2_section_missing_heading_errors() {
        let content = "## Existing Section\n\nSome content\n";
        let result = replace_h2_section(content, "Nonexistent", "new content");

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Section not found"));
    }

    #[test]
    fn pipeline_restart_recreates_abandoned_node_session() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600900";
        let original_child_session = "workflow-1735689600900-child";
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600900-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: original_child_session.to_string(),
                    status: NodeStatus::Abandoned,
                    attempts: 1,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);

        let root_uri = workflow_session_uri("workflow-1735689600900-root");
        let (root_frontmatter, root_content, _) =
            read_markdown(&root_uri, &context.config).expect("read root workflow");
        let mut root_frontmatter = root_frontmatter.expect("root frontmatter");
        root_frontmatter.insert(
            "status".to_string(),
            YamlValue::String("completed".to_string()),
        );
        root_frontmatter.insert(
            "last".to_string(),
            YamlValue::String("Root finished [[handoff]]".to_string()),
        );
        write_markdown(
            &root_uri,
            Some(&root_frontmatter),
            &root_content,
            &context.config,
        )
        .expect("write root workflow");

        let child_uri = workflow_session_uri(original_child_session);
        let (child_frontmatter, child_content, _) =
            read_markdown(&child_uri, &context.config).expect("read child workflow");
        let mut child_frontmatter = child_frontmatter.expect("child frontmatter");
        child_frontmatter.insert(
            "status".to_string(),
            YamlValue::String("abandoned".to_string()),
        );
        write_markdown(
            &child_uri,
            Some(&child_frontmatter),
            &child_content,
            &context.config,
        )
        .expect("write child workflow");

        let output = pipeline(
            &context,
            &json!({
                "sessionId": pipeline_id,
                "nodeId": "child",
                "restart": true
            }),
        )
        .expect("restart node");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");

        let nodes = data
            .get("nodes")
            .and_then(Value::as_array)
            .expect("nodes array");
        let child = nodes
            .iter()
            .find(|node| node.get("nodeId") == Some(&Value::String("child".to_string())))
            .expect("child node");
        let restarted_session = child
            .get("sessionId")
            .and_then(Value::as_str)
            .expect("new session id");
        assert_ne!(restarted_session, original_child_session);
        assert_eq!(child.get("status").and_then(Value::as_str), Some("pending"));
        assert_eq!(child.get("attempts").and_then(Value::as_u64), Some(2));
        assert_eq!(
            data.get("ready").and_then(Value::as_array),
            Some(&vec![Value::String("child".to_string())])
        );

        let (pipeline_frontmatter, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline");
        let pipeline_frontmatter = pipeline_frontmatter.expect("pipeline frontmatter");
        let updated_states: HashMap<String, NodeState> =
            frontmatter_value(&pipeline_frontmatter, "nodeStates").expect("node states");
        assert_eq!(updated_states["child"].session_id, restarted_session);
        assert_eq!(updated_states["child"].status, NodeStatus::Pending);
        assert_eq!(updated_states["child"].attempts, 2);

        assert!(
            file_exists(&child_uri, &context.config).expect("old session exists"),
            "old session file must remain as audit trail"
        );
        let (new_child_frontmatter, new_child_content, _) =
            read_markdown(&workflow_session_uri(restarted_session), &context.config)
                .expect("read restarted workflow");
        let new_child_frontmatter = new_child_frontmatter.expect("new child frontmatter");
        assert_eq!(
            new_child_frontmatter
                .get("status")
                .and_then(YamlValue::as_str),
            Some("pending")
        );
        assert!(
            new_child_content.contains("## Predecessor Context"),
            "restarted node should retain predecessor context"
        );
        assert!(new_child_content.contains("Root finished [[handoff]]"));
    }

    #[test]
    #[serial(config)]
    fn pipeline_restart_recreates_stuck_node_session() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600950";
        let original_child_session = "workflow-1735689600950-child";
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600950-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: original_child_session.to_string(),
                    status: NodeStatus::Stuck,
                    attempts: 2,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);

        let output = restart_pipeline_node(&context, pipeline_id, "child").expect("restart node");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");
        let nodes = data
            .get("nodes")
            .and_then(Value::as_array)
            .expect("nodes array");
        let child = nodes
            .iter()
            .find(|node| node.get("nodeId") == Some(&Value::String("child".to_string())))
            .expect("child node");
        let restarted_session = child
            .get("sessionId")
            .and_then(Value::as_str)
            .expect("new session id");

        assert_ne!(restarted_session, original_child_session);
        assert_eq!(child.get("status").and_then(Value::as_str), Some("pending"));
        assert_eq!(child.get("attempts").and_then(Value::as_u64), Some(3));

        let (pipeline_frontmatter, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline");
        let pipeline_frontmatter = pipeline_frontmatter.expect("pipeline frontmatter");
        let updated_states: HashMap<String, NodeState> =
            frontmatter_value(&pipeline_frontmatter, "nodeStates").expect("node states");
        assert_eq!(updated_states["child"].status, NodeStatus::Pending);
        assert_eq!(updated_states["child"].attempts, 3);
        assert_eq!(updated_states["child"].session_id, restarted_session);
    }

    #[test]
    fn pipeline_restart_rejects_when_max_attempts_reached() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689601000";
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689601000-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689601000-child".to_string(),
                    status: NodeStatus::Abandoned,
                    attempts: 3,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);
        update_workflow_node_frontmatter(
            &context,
            "workflow-1735689601000-child",
            "abandoned",
            None,
        )
        .expect("mark child abandoned");

        let error = pipeline(
            &context,
            &json!({
                "sessionId": pipeline_id,
                "nodeId": "child",
                "restart": true
            }),
        )
        .expect_err("restart must fail at max attempts");

        assert_eq!(error.to_string(), "Node child has reached max attempts (3)");

        let (pipeline_frontmatter, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline");
        let pipeline_frontmatter = pipeline_frontmatter.expect("pipeline frontmatter");
        let updated_states: HashMap<String, NodeState> =
            frontmatter_value(&pipeline_frontmatter, "nodeStates").expect("node states");
        assert_eq!(updated_states["child"].status, NodeStatus::Stuck);
        assert_eq!(
            updated_states["child"].session_id,
            "workflow-1735689601000-child"
        );
        assert_eq!(updated_states["child"].attempts, 3);
    }
}
