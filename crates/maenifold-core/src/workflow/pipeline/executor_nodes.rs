use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value as JsonValue;

use crate::acp_native::{AgentPool, SessionOpts};
use crate::error::Result;
use crate::tools::ToolContext;

use super::super::runner;
use super::executor_helpers::{
    DispatchedNode, assemble_prompt_from_parts, predecessor_context_content,
};
use super::executor_mapping::{interpolate_string, mapped_tool_args, pipeline_inputs_scope};
use super::executor_run::{run_prompt_node, run_tool_node};
use super::types::AgentConfig;
use super::{NodeState, PipelineDefinition, PipelineNode};

#[cfg(test)]
pub(super) use super::executor_run::FailurePhase;
pub(super) use super::executor_run::{NodeOutcome, NodeTaskResult, classify_node_result};

#[derive(Debug)]
pub(super) enum NodeExecutionRequest {
    Workflow {
        workflow_id: String,
        existing_session_id: Option<String>,
        prompt: String,
        session_opts: SessionOpts,
    },
    Prompt {
        session_id: String,
        prompt: String,
        tool_mode: String,
        session_opts: SessionOpts,
    },
    SequentialThinking {
        node_session_id: String,
        topic: String,
        prompt_context: String,
        total_thoughts: u32,
        session_opts: SessionOpts,
    },
    Tool {
        tool_name: String,
        tool_args: Option<JsonValue>,
        agent: Option<String>,
        allowed_tools: Option<Vec<String>>,
        timeout_seconds: Option<u64>,
    },
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_node_execution_request(
    context: &ToolContext,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    node: &PipelineNode,
    dispatch: &DispatchedNode,
    ambient_prompt: &str,
    inputs: &serde_json::Map<String, JsonValue>,
) -> Result<NodeExecutionRequest> {
    let scope = pipeline_inputs_scope(inputs);
    Ok(match node {
        PipelineNode::Workflow {
            workflow_id,
            prompt,
            timeout,
            agent_config,
            ..
        } => NodeExecutionRequest::Workflow {
            workflow_id: workflow_id.clone(),
            existing_session_id: Some(node_session_id(node_states, node.id(), dispatch)),
            prompt: resolve_execution_prompt(
                context,
                definition,
                node_states,
                node.id(),
                ambient_prompt,
                prompt.as_deref(),
                true,
                &scope,
            )?,
            session_opts: merged_session_opts(
                context,
                node.id(),
                agent_config.as_ref(),
                &None,
                &None,
                &None,
                None,
                *timeout,
            ),
        },
        PipelineNode::Prompt {
            prompt,
            tool_mode,
            agent_config,
            model,
            thought_level,
            allowed_tools,
            max_turns,
            timeout,
            ..
        } => NodeExecutionRequest::Prompt {
            session_id: node_session_id(node_states, node.id(), dispatch),
            prompt: resolve_execution_prompt(
                context,
                definition,
                node_states,
                node.id(),
                ambient_prompt,
                Some(prompt.as_str()),
                false,
                &scope,
            )?,
            tool_mode: tool_mode.clone(),
            session_opts: merged_session_opts(
                context,
                node.id(),
                agent_config.as_ref(),
                model,
                thought_level,
                allowed_tools,
                *max_turns,
                *timeout,
            ),
        },
        PipelineNode::SequentialThinking {
            topic,
            total_thoughts,
            agent_config,
            model,
            thought_level,
            allowed_tools,
            max_turns,
            timeout,
            ..
        } => NodeExecutionRequest::SequentialThinking {
            node_session_id: node_session_id(node_states, node.id(), dispatch),
            topic: interpolate_string(topic.trim(), &scope)?,
            prompt_context: resolve_execution_prompt(
                context,
                definition,
                node_states,
                node.id(),
                ambient_prompt,
                Some(topic.as_str()),
                false,
                &scope,
            )?,
            total_thoughts: *total_thoughts,
            session_opts: merged_session_opts(
                context,
                node.id(),
                agent_config.as_ref(),
                model,
                thought_level,
                allowed_tools,
                *max_turns,
                *timeout,
            ),
        },
        PipelineNode::Tool {
            tool_name,
            tool_args,
            agent,
            allowed_tools,
            timeout,
            ..
        } => NodeExecutionRequest::Tool {
            tool_name: tool_name.clone(),
            tool_args: mapped_tool_args(
                definition,
                node_states,
                node.id(),
                tool_args.clone(),
                &scope,
            )?,
            agent: agent.clone(),
            allowed_tools: allowed_tools.clone(),
            timeout_seconds: *timeout,
        },
    })
}

fn resolve_execution_prompt(
    context: &ToolContext,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    node_id: &str,
    ambient_prompt: &str,
    node_instruction: Option<&str>,
    workflow_fallback: bool,
    scope: &JsonValue,
) -> Result<String> {
    let predecessor_context =
        predecessor_context_content(context, definition, node_states, node_id)?;
    let ambient_prompt = interpolate_string(ambient_prompt.trim(), scope)?;
    let node_instruction = node_instruction
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| interpolate_string(value, scope))
        .transpose()?;
    Ok(assemble_prompt_from_parts(
        predecessor_context.as_deref(),
        &ambient_prompt,
        node_instruction.as_deref(),
        workflow_fallback,
    ))
}

fn node_session_id(
    node_states: &HashMap<String, NodeState>,
    node_id: &str,
    dispatch: &DispatchedNode,
) -> String {
    dispatch.session_id.clone().unwrap_or_else(|| {
        node_states
            .get(node_id)
            .map(|state| state.session_id.clone())
            .unwrap_or_else(|| format!("pipeline-{node_id}"))
    })
}

#[allow(clippy::too_many_arguments)]
fn merged_session_opts(
    context: &ToolContext,
    _node_id: &str,
    agent_config: Option<&AgentConfig>,
    model: &Option<String>,
    thought_level: &Option<String>,
    allowed_tools: &Option<Vec<String>>,
    max_turns: Option<u32>,
    timeout_seconds: Option<u64>,
) -> SessionOpts {
    SessionOpts {
        session_id: None,
        cwd: agent_config
            .and_then(|config| config.cwd.clone())
            .unwrap_or_else(|| context.config.maenifold_root.clone()),
        auto_grant_permissions: true,
        model: model
            .clone()
            .or_else(|| agent_config.and_then(|config| config.model.clone())),
        mode: agent_config.and_then(|config| config.mode.clone()),
        thought_level: thought_level
            .clone()
            .or_else(|| agent_config.and_then(|config| config.thought_level.clone())),
        config_options: agent_config.and_then(|config| config.config_options.clone()),
        allowed_tools: allowed_tools.clone(),
        max_turns,
        timeout_seconds,
        mcp_servers: vec![],
    }
}

pub(super) async fn execute_node_request(
    context: ToolContext,
    node_id: String,
    execution: NodeExecutionRequest,
    agent: Option<String>,
    pool: Arc<AgentPool>,
) -> (String, NodeTaskResult) {
    let result = match execution {
        NodeExecutionRequest::Workflow {
            workflow_id,
            existing_session_id,
            prompt,
            session_opts,
        } => match agent.as_deref() {
            Some(agent) => NodeTaskResult::Workflow(
                runner::run_workflow_with_pool_session_opts(
                    &context,
                    &workflow_id,
                    existing_session_id,
                    agent,
                    &prompt,
                    session_opts,
                    pool.clone(),
                    true,
                )
                .await,
            ),
            None => {
                NodeTaskResult::Workflow(Err(missing_agent_configuration(&node_id, "workflow")))
            }
        },
        NodeExecutionRequest::Prompt {
            session_id,
            prompt,
            tool_mode,
            session_opts,
        } => match agent.as_deref() {
            Some(agent) => NodeTaskResult::Prompt(
                run_prompt_node(
                    &context,
                    pool.as_ref(),
                    agent,
                    &session_id,
                    &prompt,
                    &tool_mode,
                    session_opts,
                    true,
                )
                .await,
            ),
            None => NodeTaskResult::Prompt(Err(missing_agent_configuration(&node_id, "prompt"))),
        },
        NodeExecutionRequest::SequentialThinking {
            node_session_id,
            topic,
            prompt_context,
            total_thoughts,
            session_opts,
        } => match agent.as_deref() {
            Some(agent) => NodeTaskResult::SequentialThinking(
                runner::run_think_with_pool(
                    &context,
                    agent,
                    &topic,
                    &prompt_context,
                    runner::ThinkOptions {
                        total_thoughts,
                        parent_workflow_id: Some(node_session_id),
                        model: None,
                    },
                    session_opts,
                    pool,
                    None,
                    true,
                )
                .await,
            ),
            None => NodeTaskResult::SequentialThinking(Err(missing_agent_configuration(
                &node_id,
                "sequentialThinking",
            ))),
        },
        NodeExecutionRequest::Tool {
            tool_name,
            tool_args,
            agent,
            allowed_tools,
            ..
        } => NodeTaskResult::Tool(
            run_tool_node(
                &context,
                &tool_name,
                tool_args.as_ref(),
                agent.as_deref(),
                allowed_tools.as_deref(),
            )
            .await,
        ),
    };

    (node_id, result)
}

fn missing_agent_configuration(node_id: &str, node_type: &str) -> crate::error::MaenifoldError {
    crate::error::MaenifoldError::Message(format!(
        "ERROR: Pipeline node '{node_id}' of type '{node_type}' is missing required agent configuration"
    ))
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, fs, time::Duration};

    use serde_json::json;
    use serial_test::serial;
    use tokio_util::sync::CancellationToken;

    use crate::error::MaenifoldError;
    use crate::util::json::OutputFormat;

    use super::super::executor::execute_pipeline;
    use super::super::executor_helpers::DispatchedNode;
    use super::super::session_read::{frontmatter_value, read_pipeline_session};
    use super::super::test_support::{
        pipeline_test_context, seed_pipeline_session_with_node_sessions,
    };
    use super::super::types::NodeStatus;
    use super::super::{NodeState, PipelineDefinition, PipelineNode};
    use super::*;

    fn read_raw_node_states(
        context: &crate::tools::ToolContext,
        session_id: &str,
    ) -> HashMap<String, NodeState> {
        let (_, frontmatter, _) =
            read_pipeline_session(context, session_id).expect("read pipeline");
        frontmatter_value(&frontmatter, "nodeStates").expect("node states")
    }

    fn write_sleep_agent_config(context: &crate::tools::ToolContext, agent_name: &str) {
        fs::write(
            context.config.maenifold_root.join("maenifold.json"),
            serde_json::to_vec_pretty(&json!({
                "agents": {
                    agent_name: {
                        "command": "/bin/sleep 5"
                    }
                }
            }))
            .expect("serialize agents config"),
        )
        .expect("write agents config");
    }

    #[test]
    #[serial]
    fn timeout_seconds_is_extracted_from_prompt_node() {
        let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
        let definition = PipelineDefinition {
            nodes: vec![],
            edges: vec![],
            inputs: None,
            template_id: None,
        };
        let node_states = HashMap::new();
        let node = PipelineNode::Prompt {
            id: "prompt".to_string(),
            display_label: None,
            prompt: "Prompt".to_string(),
            agent: "agent-prompt".to_string(),
            tool_mode: "auto".to_string(),
            model: None,
            thought_level: None,
            allowed_tools: None,
            max_turns: None,
            timeout: Some(30),
            agent_config: None,
        };
        let dispatch = DispatchedNode {
            node_id: node.id().to_string(),
            session_id: Some("workflow-1700000000000-prompt".to_string()),
        };

        let request = build_node_execution_request(
            &context,
            &definition,
            &node_states,
            &node,
            &dispatch,
            "",
            &serde_json::Map::new(),
        )
        .expect("build request");

        match request {
            NodeExecutionRequest::Prompt { session_opts, .. } => {
                assert_eq!(session_opts.timeout_seconds, Some(30));
            }
            NodeExecutionRequest::Workflow { .. }
            | NodeExecutionRequest::SequentialThinking { .. }
            | NodeExecutionRequest::Tool { .. } => {
                panic!("expected prompt node execution request")
            }
        }
    }

    #[test]
    #[serial]
    fn timeout_seconds_is_none_when_not_configured() {
        let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
        let definition = PipelineDefinition {
            nodes: vec![],
            edges: vec![],
            inputs: None,
            template_id: None,
        };
        let node_states = HashMap::new();
        let node = PipelineNode::Prompt {
            id: "prompt".to_string(),
            display_label: None,
            prompt: "Prompt".to_string(),
            agent: "agent-prompt".to_string(),
            tool_mode: "auto".to_string(),
            model: None,
            thought_level: None,
            allowed_tools: None,
            max_turns: None,
            timeout: None,
            agent_config: None,
        };
        let dispatch = DispatchedNode {
            node_id: node.id().to_string(),
            session_id: Some("workflow-1700000000000-prompt".to_string()),
        };

        let request = build_node_execution_request(
            &context,
            &definition,
            &node_states,
            &node,
            &dispatch,
            "",
            &serde_json::Map::new(),
        )
        .expect("build request");

        match request {
            NodeExecutionRequest::Prompt { session_opts, .. } => {
                assert_eq!(session_opts.timeout_seconds, None);
            }
            NodeExecutionRequest::Workflow { .. }
            | NodeExecutionRequest::SequentialThinking { .. }
            | NodeExecutionRequest::Tool { .. } => {
                panic!("expected prompt node execution request")
            }
        }
    }

    #[test]
    #[serial]
    fn timeout_seconds_is_extracted_from_tool_node() {
        let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
        let definition = PipelineDefinition {
            nodes: vec![],
            edges: vec![],
            inputs: None,
            template_id: None,
        };
        let node_states = HashMap::new();
        let node = PipelineNode::Tool {
            id: "tool".to_string(),
            display_label: None,
            tool_name: "GetConfig".to_string(),
            tool_args: None,
            agent: None,
            model: None,
            thought_level: None,
            allowed_tools: None,
            max_turns: None,
            timeout: Some(15),
            agent_config: None,
        };
        let dispatch = DispatchedNode {
            node_id: node.id().to_string(),
            session_id: Some("pipeline-1700000000000-tool".to_string()),
        };

        let request = build_node_execution_request(
            &context,
            &definition,
            &node_states,
            &node,
            &dispatch,
            "",
            &serde_json::Map::new(),
        )
        .expect("build request");

        match request {
            NodeExecutionRequest::Tool {
                timeout_seconds, ..
            } => {
                assert_eq!(timeout_seconds, Some(15));
            }
            NodeExecutionRequest::Workflow { .. }
            | NodeExecutionRequest::Prompt { .. }
            | NodeExecutionRequest::SequentialThinking { .. } => {
                panic!("expected tool node execution request")
            }
        }
    }

    #[tokio::test]
    #[serial]
    async fn execute_node_request_respects_timeout() {
        let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
        write_sleep_agent_config(&context, "sleep-agent");

        let session_id = "pipeline-1735689600000-node-timeout";
        let definition = PipelineDefinition {
            nodes: vec![PipelineNode::Prompt {
                id: "prompt-timeout".to_string(),
                display_label: None,
                prompt: "Block until cancelled".to_string(),
                agent: "sleep-agent".to_string(),
                tool_mode: "auto".to_string(),
                model: None,
                thought_level: None,
                allowed_tools: None,
                max_turns: None,
                timeout: Some(1),
                agent_config: None,
            }],
            edges: vec![],
            inputs: None,
            template_id: None,
        };
        let node_states = HashMap::from([(
            "prompt-timeout".to_string(),
            NodeState {
                session_id: "workflow-1735689600001-prompt-timeout".to_string(),
                status: NodeStatus::Pending,
                attempts: 0,
                acp_session_id: None,
                thinking_session_id: None,
                tool_provenance: None,
                output: None,
                failure_reason: None,
                retry_after: None,
                retry_count: None,
            },
        )]);
        seed_pipeline_session_with_node_sessions(&context, session_id, definition, node_states, 1);

        let result = tokio::time::timeout(
            Duration::from_secs(2),
            execute_pipeline(&context, session_id, "", CancellationToken::new()),
        )
        .await
        .expect("pipeline execution should complete within the configured node timeout")
        .expect("pipeline execution should return a result");

        assert_eq!(result.completed, 0);
        assert_eq!(result.abandoned, 1);
        assert_eq!(result.status, "abandoned");

        let node_states = read_raw_node_states(&context, session_id);
        let state = &node_states["prompt-timeout"];
        assert_eq!(state.status, NodeStatus::Abandoned);
        let reason = state
            .failure_reason
            .as_deref()
            .expect("timeout should persist a failure reason")
            .to_ascii_lowercase();
        assert!(
            reason.contains("timeout") || reason.contains("timed out"),
            "unexpected timeout reason: {reason}"
        );
    }

    #[test]
    fn node_timeout_produces_abandoned_outcome() {
        let outcome = classify_node_result(
            "prompt-timeout",
            NodeTaskResult::Prompt(Err(MaenifoldError::Message(
                "node timed out after 1s".to_string(),
            ))),
        );

        match outcome {
            NodeOutcome::Abandoned { reason, .. } => {
                let reason = reason.to_ascii_lowercase();
                assert!(
                    reason.contains("timeout") || reason.contains("timed out"),
                    "unexpected timeout reason: {reason}"
                );
            }
            NodeOutcome::Completed { .. } => {
                panic!("timeout errors should abandon the node")
            }
        }
    }

    #[test]
    #[serial]
    fn build_node_execution_request_errors_when_tool_mapping_fails() {
        let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
        let definition = PipelineDefinition {
            nodes: vec![PipelineNode::Tool {
                id: "target".to_string(),
                display_label: None,
                tool_name: "noop".to_string(),
                tool_args: None,
                agent: None,
                model: None,
                thought_level: None,
                allowed_tools: None,
                max_turns: None,
                timeout: None,
                agent_config: None,
            }],
            edges: vec![super::super::types::PipelineEdge {
                source: "source".to_string(),
                target: "target".to_string(),
                edge_type: super::super::types::EdgeType::Success,
                mapping: Some(HashMap::from([(
                    "payload".to_string(),
                    "result.value".to_string(),
                )])),
            }],
            inputs: None,
            template_id: None,
        };
        let node_states = HashMap::from([(
            "source".to_string(),
            NodeState {
                session_id: "pipeline-source".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                acp_session_id: None,
                thinking_session_id: None,
                tool_provenance: None,
                output: Some("not-json".to_string()),
                failure_reason: None,
                retry_after: None,
                retry_count: None,
            },
        )]);
        let node = definition.nodes.first().expect("target node");
        let dispatch = DispatchedNode {
            node_id: "target".to_string(),
            session_id: Some("pipeline-target".to_string()),
        };

        let error = build_node_execution_request(
            &context,
            &definition,
            &node_states,
            node,
            &dispatch,
            "",
            &serde_json::Map::new(),
        )
        .expect_err("invalid mapped tool args should abort dispatch");

        assert!(
            error
                .to_string()
                .contains("could not parse node 'source' output as JSON")
        );
    }

    #[test]
    #[serial]
    fn build_node_execution_request_interpolates_inputs_in_prompt_and_tool_args() {
        let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
        let definition = PipelineDefinition {
            nodes: vec![],
            edges: vec![],
            inputs: None,
            template_id: None,
        };
        let node_states = HashMap::new();
        let inputs = serde_json::Map::from_iter([("name".to_string(), json!("world"))]);

        let prompt_node = PipelineNode::Prompt {
            id: "prompt".to_string(),
            display_label: None,
            prompt: "Hello {{inputs.name}}".to_string(),
            agent: "agent-prompt".to_string(),
            tool_mode: "auto".to_string(),
            model: None,
            thought_level: None,
            allowed_tools: None,
            max_turns: None,
            timeout: None,
            agent_config: None,
        };
        let dispatch = DispatchedNode {
            node_id: prompt_node.id().to_string(),
            session_id: Some("workflow-1700000000000-prompt".to_string()),
        };
        let prompt_request = build_node_execution_request(
            &context,
            &definition,
            &node_states,
            &prompt_node,
            &dispatch,
            "",
            &inputs,
        )
        .expect("build prompt request");
        match prompt_request {
            NodeExecutionRequest::Prompt { prompt, .. } => assert_eq!(prompt, "Hello world"),
            other => panic!("expected prompt request, got {other:?}"),
        }

        let tool_node = PipelineNode::Tool {
            id: "tool".to_string(),
            display_label: None,
            tool_name: "GetConfig".to_string(),
            tool_args: Some(json!({
                "message": "Hello {{inputs.name}}",
                "typed": { "$ref": "inputs.name" }
            })),
            agent: None,
            model: None,
            thought_level: None,
            allowed_tools: None,
            max_turns: None,
            timeout: None,
            agent_config: None,
        };
        let tool_dispatch = DispatchedNode {
            node_id: tool_node.id().to_string(),
            session_id: Some("workflow-1700000000000-tool".to_string()),
        };
        let tool_request = build_node_execution_request(
            &context,
            &definition,
            &node_states,
            &tool_node,
            &tool_dispatch,
            "",
            &inputs,
        )
        .expect("build tool request");
        match tool_request {
            NodeExecutionRequest::Tool { tool_args, .. } => assert_eq!(
                tool_args,
                Some(json!({ "message": "Hello world", "typed": "world" }))
            ),
            other => panic!("expected tool request, got {other:?}"),
        }
    }

    #[test]
    #[serial]
    fn build_node_execution_request_does_not_interpolate_workflow_id_or_tool_name() {
        let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
        let definition = PipelineDefinition {
            nodes: vec![],
            edges: vec![],
            inputs: None,
            template_id: None,
        };
        let node_states = HashMap::new();
        let inputs = serde_json::Map::from_iter([
            ("workflow".to_string(), json!("resolved.workflow")),
            ("tool".to_string(), json!("GetConfig")),
            ("name".to_string(), json!("world")),
        ]);

        let workflow_node = PipelineNode::Workflow {
            id: "workflow".to_string(),
            display_label: None,
            workflow_id: "{{inputs.workflow}}".to_string(),
            prompt: Some("Hello {{inputs.name}}".to_string()),
            timeout: None,
            acp_client: None,
            agent_config: None,
        };
        let workflow_dispatch = DispatchedNode {
            node_id: workflow_node.id().to_string(),
            session_id: Some("workflow-1700000000000-workflow".to_string()),
        };
        let workflow_request = build_node_execution_request(
            &context,
            &definition,
            &node_states,
            &workflow_node,
            &workflow_dispatch,
            "",
            &inputs,
        )
        .expect("build workflow request");
        match workflow_request {
            NodeExecutionRequest::Workflow { workflow_id, .. } => {
                assert_eq!(workflow_id, "{{inputs.workflow}}");
            }
            other => panic!("expected workflow request, got {other:?}"),
        }

        let tool_node = PipelineNode::Tool {
            id: "tool".to_string(),
            display_label: None,
            tool_name: "{{inputs.tool}}".to_string(),
            tool_args: Some(json!({ "message": "Hello {{inputs.name}}" })),
            agent: None,
            model: None,
            thought_level: None,
            allowed_tools: None,
            max_turns: None,
            timeout: None,
            agent_config: None,
        };
        let tool_dispatch = DispatchedNode {
            node_id: tool_node.id().to_string(),
            session_id: Some("workflow-1700000000000-tool".to_string()),
        };
        let tool_request = build_node_execution_request(
            &context,
            &definition,
            &node_states,
            &tool_node,
            &tool_dispatch,
            "",
            &inputs,
        )
        .expect("build tool request");
        match tool_request {
            NodeExecutionRequest::Tool {
                tool_name,
                tool_args,
                ..
            } => {
                assert_eq!(tool_name, "{{inputs.tool}}");
                assert_eq!(tool_args, Some(json!({ "message": "Hello world" })));
            }
            other => panic!("expected tool request, got {other:?}"),
        }
    }

    #[test]
    #[serial]
    fn build_node_execution_request_interpolates_sequential_thinking_topic() {
        let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
        let definition = PipelineDefinition {
            nodes: vec![],
            edges: vec![],
            inputs: None,
            template_id: None,
        };
        let node_states = HashMap::new();
        let inputs = serde_json::Map::from_iter([("name".to_string(), json!("world"))]);
        let node = PipelineNode::SequentialThinking {
            id: "think".to_string(),
            display_label: None,
            topic: "Plan for {{inputs.name}}".to_string(),
            total_thoughts: 3,
            analysis_type: None,
            agent: "agent-think".to_string(),
            model: None,
            thought_level: None,
            allowed_tools: None,
            max_turns: None,
            timeout: None,
            agent_config: None,
        };
        let dispatch = DispatchedNode {
            node_id: node.id().to_string(),
            session_id: Some("workflow-1700000000000-think".to_string()),
        };

        let request = build_node_execution_request(
            &context,
            &definition,
            &node_states,
            &node,
            &dispatch,
            "",
            &inputs,
        )
        .expect("build sequential thinking request");

        match request {
            NodeExecutionRequest::SequentialThinking {
                topic,
                prompt_context,
                ..
            } => {
                assert_eq!(topic, "Plan for world");
                assert_eq!(prompt_context, "Plan for world");
            }
            other => panic!("expected sequential thinking request, got {other:?}"),
        }
    }

    #[tokio::test]
    #[serial]
    async fn run_tool_node_preserves_local_tool_registry_path_for_non_namespaced_tools() {
        let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);

        let output = run_tool_node(&context, "GetConfig", None, None, None)
            .await
            .expect("local tool should execute through ToolRegistry");

        assert!(output.response.contains("Maenifold Configuration"));
    }

    #[tokio::test]
    #[serial]
    async fn run_tool_node_preserves_local_tool_registry_path_for_maenifold_namespace() {
        let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);

        let output = run_tool_node(&context, "maenifold.GetConfig", None, None, None)
            .await
            .expect("maenifold namespaced tool should execute through ToolRegistry");

        assert!(output.response.contains("Maenifold Configuration"));
    }

    #[tokio::test]
    #[serial]
    async fn run_tool_node_rejects_external_tool_not_in_explicit_allowlist() {
        let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);

        let error = run_tool_node(
            &context,
            "server.echo",
            Some(&json!({ "value": 1 })),
            Some("test-agent"),
            Some(&["server.other".to_string()]),
        )
        .await
        .expect_err("external MCP tool not in explicit list should be rejected");

        assert!(matches!(error, MaenifoldError::AgentSetup(_)));
        assert!(
            error
                .to_string()
                .contains("not in the explicit allowed_tools")
        );
    }

    #[tokio::test]
    #[serial]
    async fn run_tool_node_allows_external_tools_by_default_when_no_allowlist() {
        let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);

        let error = run_tool_node(
            &context,
            "server.echo",
            Some(&json!({ "value": 1 })),
            None,
            None,
        )
        .await
        .expect_err("should fail at server resolution, not at tool permission");

        assert!(
            error.to_string().contains("not found"),
            "with no allowlist restriction, the error should be about missing server config, not permissions: {error}"
        );
    }
}
