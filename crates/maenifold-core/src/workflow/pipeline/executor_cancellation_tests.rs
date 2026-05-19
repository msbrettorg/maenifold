use std::collections::HashMap;

use super::{
    NodeState, NodeStatus, PipelineDefinition, PipelineNode, cancellable_node_session_targets,
};

fn node_state(status: NodeStatus, session_id: &str) -> NodeState {
    NodeState {
        session_id: session_id.to_string(),
        status,
        attempts: 1,
        started_at: None,
        completed_at: None,
        acp_session_id: None,
        thinking_session_id: None,
        tool_provenance: None,
        input_tokens: None,
        output_tokens: None,
        model: None,
        output: None,
        failure_reason: None,
        retry_after: None,
        retry_count: None,
    }
}

fn node_state_with_acp(status: NodeStatus, session_id: &str, acp_session_id: &str) -> NodeState {
    let mut state = node_state(status, session_id);
    state.acp_session_id = Some(acp_session_id.to_string());
    state
}

#[test]
fn acp_session_targets_only_returns_prompt_and_thinking_nodes_with_acp_lineage() {
    let definition = PipelineDefinition {
        nodes: vec![
            PipelineNode::Prompt {
                id: "prompt-active".to_string(),
                display_label: None,
                prompt: "Prompt".to_string(),
                agent: "agent-a".to_string(),
                tool_mode: "auto".to_string(),
                model: None,
                thought_level: None,
                allowed_tools: None,
                max_turns: None,
                cwd: None,
                timeout: None,
                agent_config: None,
                adopt: None,
                requires_role: None,
                retry_policy: None,
                join_policy: None,
                session_ref: None,
            },
            PipelineNode::Tool {
                id: "tool-without-agent".to_string(),
                display_label: None,
                tool_name: "maenifold.GetConfig".to_string(),
                tool_args: None,
                agent: None,
                model: None,
                thought_level: None,
                allowed_tools: None,
                max_turns: None,
                cwd: None,
                timeout: None,
                agent_config: None,
                retry_policy: None,
                join_policy: None,
            },
            PipelineNode::SequentialThinking {
                id: "think-ready".to_string(),
                display_label: None,
                topic: "Topic".to_string(),
                total_thoughts: 2,
                agent: "agent-b".to_string(),
                model: None,
                thought_level: None,
                allowed_tools: None,
                max_turns: None,
                cwd: None,
                timeout: None,
                agent_config: None,
                adopt: None,
                requires_role: None,
                retry_policy: None,
                join_policy: None,
            },
        ],
        edges: vec![],
        inputs: None,
        input_schema: None,
        template_id: None,
    };

    let node_states = HashMap::from([
        (
            "prompt-active".to_string(),
            node_state_with_acp(NodeStatus::Active, "session-active", "acp-active"),
        ),
        (
            "tool-without-agent".to_string(),
            node_state(NodeStatus::Active, "session-tool"),
        ),
        (
            "think-ready".to_string(),
            node_state_with_acp(NodeStatus::Ready, "session-ready", "acp-ready"),
        ),
    ]);

    let mut targets = cancellable_node_session_targets(&definition, &node_states);
    targets.sort();
    assert_eq!(
        targets,
        vec![
            ("prompt-active".to_string(), "acp-active".to_string()),
            ("think-ready".to_string(), "acp-ready".to_string())
        ]
    );
}
