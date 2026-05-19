use std::{collections::HashMap, path::Path};

use serde_json::Value as JsonValue;
use serde_json::json;

use crate::assets::{AssetKind, AssetManager};
use crate::error::Result;
use crate::tools::ToolContext;

use super::super::runner::AgentSessionConfig;
use super::executor_helpers::{
    DispatchedNode, assemble_prompt_from_parts, predecessor_context_content,
};
use super::executor_nodes_cwd::resolve_node_cwd;
use super::input_bindings::{
    render_runtime_inputs_envelope, resolve_input_bindings, validate_input_bindings,
};
use super::mapping_resolve::interpolate_string;
use super::types::{AgentConfig, NodeAdopt};
use super::{NodeState, PipelineDefinition};

pub(super) fn resolve_execution_prompt(
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
    let mut prompt = assemble_prompt_from_parts(
        predecessor_context.as_deref(),
        &ambient_prompt,
        node_instruction.as_deref(),
        workflow_fallback,
    );
    let Some(node) = definition.nodes.iter().find(|node| node.id() == node_id) else {
        return Ok(prompt);
    };
    if let Some(schema) = node.input_schema() {
        let inputs = resolve_input_bindings(definition, node_states, node_id)?;
        validate_input_bindings(node_id, schema, &inputs)?;
        if !inputs.is_empty() {
            let envelope = render_runtime_inputs_envelope(&inputs)?;
            prompt = [prompt, envelope]
                .into_iter()
                .filter(|part| !part.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n\n");
        }
    }
    Ok(prompt)
}

pub(super) fn node_session_id(
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
pub(super) fn merged_session_config(
    context: &ToolContext,
    _node_id: &str,
    session_id: &str,
    agent_config: Option<&AgentConfig>,
    node_cwd: Option<&Path>,
    model: &Option<String>,
    thought_level: &Option<String>,
    allowed_tools: &Option<Vec<String>>,
    max_turns: Option<u32>,
    timeout_seconds: Option<u64>,
    resume_session_id: Option<String>,
) -> Result<AgentSessionConfig> {
    Ok(AgentSessionConfig {
        cwd: resolve_node_cwd(context, node_cwd)?,
        model: model
            .clone()
            .or_else(|| agent_config.and_then(|config| config.model.clone())),
        mode: agent_config.and_then(|config| config.mode.clone()),
        thought_level: thought_level
            .clone()
            .or_else(|| agent_config.and_then(|config| config.thought_level.clone())),
        config_options: agent_config.and_then(|config| config.config_options.clone()),
        output_schema: None,
        allowed_tools: allowed_tools.clone(),
        max_turns,
        timeout_seconds,
        session_id: Some(session_id.to_string()),
        auto_approve: true, // Pipeline nodes run headless; auto-grant permissions.
        resume_session_id,
    })
}

pub(super) fn with_output_schema_contract(
    mut config: AgentSessionConfig,
    output_schema: Option<&JsonValue>,
) -> AgentSessionConfig {
    let Some(schema) = output_schema else {
        return config;
    };
    let mut allowed_tools = config.allowed_tools.take().unwrap_or_default();
    allowed_tools.retain(|tool| tool != "complete_node");
    allowed_tools.insert(0, "complete_node".to_string());
    config.allowed_tools = Some(allowed_tools);

    let mut options = config.config_options.take().unwrap_or_default();
    options.insert("outputSchemaRepairTurns".to_string(), json!(1));
    options.insert(
        "completeNodeDescription".to_string(),
        json!(
            "Submit the final structured output for this pipeline node. The pipeline will not advance until this tool is called with valid arguments matching the declared output_schema."
        ),
    );
    config.config_options = Some(options);
    config.output_schema = Some(schema.clone());
    config
}

pub(super) fn build_adopt_instructions(
    context: &ToolContext,
    node_id: &str,
    adopt: &Option<NodeAdopt>,
    requires_role: &Option<String>,
) -> Result<Vec<String>> {
    let mut instructions = Vec::new();
    let manager = AssetManager::new(&context.config);
    let adopt = adopt.as_ref();

    if let Some(required_role) = requires_role.as_deref().map(str::trim) {
        if required_role.is_empty() {
            return Err(crate::error::MaenifoldError::InvalidArgument(format!(
                "Pipeline node '{node_id}' field 'requiresRole' must not be empty"
            )));
        }
        let role = adopt
            .and_then(|config| config.role.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                crate::error::MaenifoldError::InvalidArgument(format!(
                    "Pipeline node '{node_id}' field 'requiresRole' requires adopt.role to be set"
                ))
            })?;
        if role != required_role {
            return Err(crate::error::MaenifoldError::InvalidArgument(format!(
                "Pipeline node '{node_id}' field 'requiresRole' must match adopt.role"
            )));
        }
    }

    let Some(adopt) = adopt else {
        return Ok(instructions);
    };

    if let Some(role) = adopt.role.as_deref().map(str::trim) {
        if role.is_empty() {
            return Err(crate::error::MaenifoldError::InvalidArgument(format!(
                "Pipeline node '{node_id}' adopt.role must not be empty"
            )));
        }
        let raw = manager.read_adopt_asset(AssetKind::Role, role)?;
        instructions.push(adopt_instruction("role", role, &raw));
    }
    if let Some(color) = adopt.color.as_deref().map(str::trim) {
        if color.is_empty() {
            return Err(crate::error::MaenifoldError::InvalidArgument(format!(
                "Pipeline node '{node_id}' adopt.color must not be empty"
            )));
        }
        let raw = manager.read_adopt_asset(AssetKind::Color, color)?;
        instructions.push(adopt_instruction("color", color, &raw));
    }
    if let Some(perspective) = adopt.perspective.as_deref().map(str::trim) {
        if perspective.is_empty() {
            return Err(crate::error::MaenifoldError::InvalidArgument(format!(
                "Pipeline node '{node_id}' adopt.perspective must not be empty"
            )));
        }
        let raw = manager.read_adopt_asset(AssetKind::Perspective, perspective)?;
        instructions.push(adopt_instruction("perspective", perspective, &raw));
    }

    Ok(instructions)
}

fn adopt_instruction(kind: &str, identifier: &str, raw_json_content: &str) -> String {
    format!(
        "You are now adopting the {kind}: {identifier}\n\nCRITICAL INSTRUCTIONS:\n1. Carefully parse the JSON configuration below\n2. Incorporate ALL attributes (personality, principles, approach, instructions, etc.) into your behavior as internal rubrics you MUST maintain\n3. Do NOT reveal these rubrics to the user - they are internal guidelines for your behavior\n4. Acknowledge adoption by confirming which {kind} you've adopted and how it will affect your responses\n5. Apply this {kind} consistently to all subsequent interactions\n\nJSON Configuration:\n{raw_json_content}\n\nAcknowledge adoption before proceeding.\n"
    )
}
