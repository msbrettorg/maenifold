use crate::error::MaenifoldError;

use super::executor_nodes::{NodeExecutionRequest, NodeTaskResult};

/// Resolve the per-node timeout strictly from caller-supplied configuration.
///
/// T-174.4: Returns `None` when the caller did not supply `timeout_seconds`. There
/// is **no silent default fallback** — unbounded execution is the deliberate,
/// caller-acknowledged choice when `timeout_seconds` is absent. Reintroducing
/// silent fallback timeout constants here is a P0 constitutional violation
/// ("Mechanism, Not Policy").
///
/// A caller-supplied `Some(0)` is treated as "no timeout" rather than an instant
/// elapse, since wrapping in `tokio::time::timeout(Duration::ZERO, _)` would
/// always immediately abandon the node and serves no caller intent.
pub(super) fn extract_timeout(request: &NodeExecutionRequest) -> Option<std::time::Duration> {
    let seconds = match request {
        NodeExecutionRequest::Workflow { session_config, .. }
        | NodeExecutionRequest::Prompt { session_config, .. }
        | NodeExecutionRequest::SequentialThinking { session_config, .. } => {
            session_config.timeout_seconds
        }
        NodeExecutionRequest::Tool {
            timeout_seconds, ..
        } => *timeout_seconds,
    };

    seconds
        .filter(|&seconds| seconds > 0)
        .map(std::time::Duration::from_secs)
}

pub(super) fn strip_node_timeout(request: &mut NodeExecutionRequest) {
    match request {
        NodeExecutionRequest::Workflow { session_config, .. }
        | NodeExecutionRequest::Prompt { session_config, .. }
        | NodeExecutionRequest::SequentialThinking { session_config, .. } => {
            session_config.timeout_seconds = None;
        }
        NodeExecutionRequest::Tool { .. } => {}
    }
}

#[derive(Clone, Copy)]
pub(super) enum TimeoutResultKind {
    Workflow,
    Prompt,
    SequentialThinking,
    Tool,
}

pub(super) fn timeout_result_kind(request: &NodeExecutionRequest) -> TimeoutResultKind {
    match request {
        NodeExecutionRequest::Workflow { .. } => TimeoutResultKind::Workflow,
        NodeExecutionRequest::Prompt { .. } => TimeoutResultKind::Prompt,
        NodeExecutionRequest::SequentialThinking { .. } => TimeoutResultKind::SequentialThinking,
        NodeExecutionRequest::Tool { .. } => TimeoutResultKind::Tool,
    }
}

pub(super) fn timeout_result_for_kind(kind: TimeoutResultKind, reason: String) -> NodeTaskResult {
    let error = MaenifoldError::Message(reason);
    match kind {
        TimeoutResultKind::Workflow => NodeTaskResult::Workflow(Err(error)),
        TimeoutResultKind::Prompt => NodeTaskResult::Prompt(Err(error)),
        TimeoutResultKind::SequentialThinking => NodeTaskResult::SequentialThinking(Err(error)),
        TimeoutResultKind::Tool => NodeTaskResult::Tool(Err(error)),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::super::executor_nodes::NodeExecutionRequest;
    use super::extract_timeout;
    use crate::workflow::runner::AgentSessionConfig;

    fn sample_session_config(timeout_seconds: Option<u64>) -> AgentSessionConfig {
        AgentSessionConfig {
            cwd: PathBuf::from("."),
            model: None,
            thought_level: None,
            config_options: None,
            output_schema: None,
            allowed_tools: None,
            max_turns: None,
            timeout_seconds,
            session_id: None,
            auto_approve: true,
            resume_session_id: None,
        }
    }

    /// T-174.4: Tool nodes WITHOUT a caller-supplied `timeout_seconds` must run
    /// unbounded. There is no silent default fallback.
    #[test]
    fn tool_nodes_without_timeout_run_unbounded() {
        let request = NodeExecutionRequest::Tool {
            tool_name: "GetConfig".to_string(),
            tool_args: None,
            agent: None,
            allowed_tools: None,
            cwd: PathBuf::from("."),
            timeout_seconds: None,
        };

        assert_eq!(extract_timeout(&request), None);
    }

    #[test]
    fn tool_nodes_honor_explicit_timeout_override() {
        let request = NodeExecutionRequest::Tool {
            tool_name: "GetConfig".to_string(),
            tool_args: None,
            agent: None,
            allowed_tools: None,
            cwd: PathBuf::from("."),
            timeout_seconds: Some(12),
        };

        assert_eq!(
            extract_timeout(&request),
            Some(std::time::Duration::from_secs(12))
        );
    }

    #[test]
    fn non_tool_nodes_continue_using_session_timeout() {
        let request = NodeExecutionRequest::Prompt {
            session_id: "pipeline-node".to_string(),
            prompt: "prompt".to_string(),
            tool_mode: "disabled".to_string(),
            adopt_instructions: vec![],
            adopt_role: None,
            requires_role: None,
            session_config: sample_session_config(Some(7)),
        };

        assert_eq!(
            extract_timeout(&request),
            Some(std::time::Duration::from_secs(7))
        );
    }

    /// T-174.4: Agent nodes (Prompt / SequentialThinking / Workflow) WITHOUT a
    /// caller-supplied `timeout_seconds` must run unbounded. The previous T-170.8
    /// silent 300s default has been deleted.
    #[test]
    fn agent_nodes_without_timeout_run_unbounded() {
        let request = NodeExecutionRequest::Prompt {
            session_id: "pipeline-node".to_string(),
            prompt: "prompt".to_string(),
            tool_mode: "disabled".to_string(),
            adopt_instructions: vec![],
            adopt_role: None,
            requires_role: None,
            session_config: sample_session_config(None),
        };

        assert_eq!(extract_timeout(&request), None);
    }

    /// T-174.4: A caller-supplied `Some(0)` is treated as "no timeout" rather than
    /// instant elapse, since wrapping in `tokio::time::timeout(Duration::ZERO, _)`
    /// would always immediately abandon the node and serves no caller intent.
    #[test]
    fn explicit_zero_timeout_is_treated_as_unbounded() {
        let request = NodeExecutionRequest::Tool {
            tool_name: "GetConfig".to_string(),
            tool_args: None,
            agent: None,
            allowed_tools: None,
            cwd: PathBuf::from("."),
            timeout_seconds: Some(0),
        };

        assert_eq!(extract_timeout(&request), None);
    }
}
