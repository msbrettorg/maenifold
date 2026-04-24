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

#[test]
fn active_node_session_targets_only_returns_active_nodes_with_agents() {
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
            },
            PipelineNode::SequentialThinking {
                id: "think-ready".to_string(),
                display_label: None,
                topic: "Topic".to_string(),
                total_thoughts: 2,
                analysis_type: None,
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
            },
        ],
        edges: vec![],
        inputs: None,
        template_id: None,
    };

    let node_states = HashMap::from([
        (
            "prompt-active".to_string(),
            node_state(NodeStatus::Active, "session-active"),
        ),
        (
            "tool-without-agent".to_string(),
            node_state(NodeStatus::Active, "session-tool"),
        ),
        (
            "think-ready".to_string(),
            node_state(NodeStatus::Ready, "session-ready"),
        ),
    ]);

    let targets = cancellable_node_session_targets(&definition, &node_states);
    assert_eq!(
        targets,
        vec![(
            "prompt-active".to_string(),
            "agent-a".to_string(),
            "session-active".to_string()
        )]
    );
}
