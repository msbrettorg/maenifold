use chrono::Utc;
use maenifold_core::markdown::read_markdown;
use maenifold_core::tools::ToolContext;
use maenifold_core::workflow::pipeline::NodeStatus;
use maenifold_core::{MaenifoldError, Result};
use jockey::acp::PREFLIGHT_ENV_VALIDATION_FAILED_PREFIX;

use crate::recovery::{
    persist_pipeline_frontmatter, pipeline_node_states, pipeline_session_uri,
    update_workflow_status,
};

pub(crate) fn promote_preflight_env_failures_to_stuck(
    context: &ToolContext,
    session_id: &str,
) -> Result<usize> {
    let pipeline_uri = pipeline_session_uri(session_id);
    let (frontmatter, pipeline_content, _) = read_markdown(&pipeline_uri, &context.config)?;
    let mut frontmatter = frontmatter.ok_or_else(|| {
        MaenifoldError::Message(format!(
            "Pipeline session {session_id} has invalid frontmatter"
        ))
    })?;
    let mut node_states = pipeline_node_states(&frontmatter)?;
    let mut promoted = 0usize;

    for state in node_states.values_mut() {
        let Some(reason) = state.failure_reason.as_deref() else {
            continue;
        };
        if !matches!(state.status, NodeStatus::Abandoned)
            || !reason
                .to_ascii_lowercase()
                .contains(PREFLIGHT_ENV_VALIDATION_FAILED_PREFIX)
        {
            continue;
        }

        state.status = NodeStatus::Stuck;
        state.completed_at = Some(Utc::now());
        state.failure_reason = Some(preflight_failure_message(reason));
        update_workflow_status(context, &state.session_id, "stuck")?;
        promoted += 1;
    }

    if promoted > 0 {
        persist_pipeline_frontmatter(
            context,
            &pipeline_uri,
            &mut frontmatter,
            &pipeline_content,
            &node_states,
        )?;
    }

    Ok(promoted)
}

fn preflight_failure_message(reason: &str) -> String {
    format!("{reason} — fix agent environment configuration and rerun the node")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recovery::{pipeline_session_uri, workflow_session_uri};
    use maenifold_core::config::Config;
    use maenifold_core::markdown::{Frontmatter, read_markdown, write_markdown};
    use maenifold_core::tools::ToolContext;
    use maenifold_core::util::json::OutputFormat;
    use maenifold_core::workflow::pipeline::NodeState;
    use serde_yml::Value as YamlValue;
    use std::collections::HashMap;

    struct ConfigGuard;

    impl Drop for ConfigGuard {
        fn drop(&mut self) {
            Config::reset_overrides();
        }
    }

    #[test]
    fn preflight_adapter_error_maps_node_to_stuck() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root = std::env::current_dir()
            .expect("current dir")
            .join("target")
            .join("pipeline-driver-tests")
            .join(format!("preflight-stuck-{unique}"));
        std::fs::create_dir_all(&root).expect("create test root");
        Config::override_root(&root);
        Config::set_database_path(root.join("memory.db"));
        let _guard = ConfigGuard;
        let config = Config::current();
        config.ensure_directories().expect("ensure directories");
        let context = ToolContext::new(config, OutputFormat::Json);
        let session_id = "pipeline-1735689600000-preflight-stuck";
        let workflow_id = "workflow-1735689600000-preflight-stuck";
        let pipeline_uri = pipeline_session_uri(session_id);
        let workflow_uri = workflow_session_uri(workflow_id);
        let node_states = HashMap::from([(
            "prompt-1".to_string(),
            NodeState {
                session_id: workflow_id.to_string(),
                status: NodeStatus::Abandoned,
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
                failure_reason: Some(
                    "preflight env validation failed for agent 'codex': API_KEY from /missing (missing)"
                        .to_string(),
                ),
                retry_after: None,
                retry_count: None,
            },
        )]);
        let mut pipeline_frontmatter = Frontmatter::new();
        pipeline_frontmatter.insert(
            "status".to_string(),
            YamlValue::String("failed".to_string()),
        );
        pipeline_frontmatter.insert(
            "nodeStates".to_string(),
            serde_yml::to_value(&node_states).expect("node states yaml"),
        );
        let mut workflow_frontmatter = Frontmatter::new();
        workflow_frontmatter.insert(
            "status".to_string(),
            YamlValue::String("abandoned".to_string()),
        );
        write_markdown(
            &pipeline_uri,
            Some(&pipeline_frontmatter),
            "# Pipeline",
            &context.config,
        )
        .expect("write pipeline");
        write_markdown(
            &workflow_uri,
            Some(&workflow_frontmatter),
            "# Node",
            &context.config,
        )
        .expect("write workflow");

        let promoted = promote_preflight_env_failures_to_stuck(&context, session_id)
            .expect("promote preflight failure");

        assert_eq!(promoted, 1);
        let (frontmatter, _, _) =
            read_markdown(&pipeline_uri, &context.config).expect("read pipeline");
        let states: HashMap<String, NodeState> = serde_yml::from_value(
            frontmatter
                .expect("pipeline frontmatter")
                .get("nodeStates")
                .cloned()
                .expect("node states"),
        )
        .expect("deserialize node states");
        assert_eq!(states["prompt-1"].status, NodeStatus::Stuck);
        assert!(
            states["prompt-1"]
                .failure_reason
                .as_deref()
                .unwrap_or_default()
                .contains("fix agent environment configuration")
        );
    }
}
