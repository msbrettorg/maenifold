use crate::error::Result;
use crate::markdown::{Frontmatter, write_markdown};
use crate::sessions::{date_parts_from_session_id, pipeline_session_uri};
use crate::tools::ToolContext;
use serde_json::{Map as JsonMap, Value as JsonValue};
use serde_yml::Value as YamlValue;
use std::collections::BTreeMap;
use std::fs;

use super::PipelineDefinition;
use super::resource_notifications::emit_pipeline_resource_updated;

#[cfg(test)]
pub(super) use super::session_delete_files::delete_file_verified_with;

pub(super) fn write_pipeline_session(
    context: &ToolContext,
    pipeline_id: &str,
    definition: &PipelineDefinition,
    node_states: &BTreeMap<String, YamlValue>,
    max_concurrency: u64,
    prompt: Option<&str>,
    name: Option<&str>,
    description: Option<&str>,
) -> Result<()> {
    let uri = pipeline_session_uri(pipeline_id);
    let (year, month, day) = date_parts_from_session_id(pipeline_id);
    fs::create_dir_all(
        context
            .config
            .memory_path
            .join("pipelines")
            .join(format!("{year:04}"))
            .join(format!("{month:02}"))
            .join(format!("{day:02}")),
    )?;
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
    if let Some(name) = name.map(str::trim).filter(|value| !value.is_empty()) {
        frontmatter.insert("name".to_string(), YamlValue::String(name.to_string()));
    }
    if let Some(description) = description.map(str::trim).filter(|value| !value.is_empty()) {
        frontmatter.insert(
            "description".to_string(),
            YamlValue::String(description.to_string()),
        );
    }
    frontmatter.insert("definition".to_string(), serde_yml::to_value(definition)?);
    if let Some(inputs) = definition
        .inputs
        .as_ref()
        .filter(|inputs| !inputs.is_empty())
    {
        frontmatter.insert("inputs".to_string(), serde_yml::to_value(inputs)?);
    }
    frontmatter.insert(
        "nodeStates".to_string(),
        YamlValue::Mapping(
            node_states
                .iter()
                .map(|(key, value)| (YamlValue::String(key.clone()), value.clone()))
                .collect(),
        ),
    );
    frontmatter.insert(
        "maxConcurrency".to_string(),
        serde_yml::to_value(max_concurrency)?,
    );
    frontmatter.insert("first".to_string(), serde_yml::to_value(0u64)?);
    frontmatter.insert("last".to_string(), serde_yml::to_value(0u64)?);
    // T-93.4: cursor tracks completed dispatch cycles for crash-recovery continuity.
    frontmatter.insert("cursor".to_string(), serde_yml::to_value(0u64)?);
    if let Some(prompt) = prompt.map(str::trim).filter(|prompt| !prompt.is_empty()) {
        frontmatter.insert("prompt".to_string(), YamlValue::String(prompt.to_string()));
    }

    write_markdown(
        &uri,
        Some(&frontmatter),
        "# Pipeline Session\n",
        &context.config,
    )?;
    Ok(())
}

pub(super) fn persist_pipeline_runtime_context(
    context: &ToolContext,
    session_id: &str,
    prompt_override: Option<&str>,
    input_overrides: Option<&JsonMap<String, JsonValue>>,
) -> Result<String> {
    let (uri, mut frontmatter, pipeline_content) =
        super::session_read::read_pipeline_session(context, session_id)?;
    let definition: PipelineDefinition =
        super::session_read::frontmatter_value(&frontmatter, "definition")?;
    let mut inputs = super::session_read::optional_frontmatter_value::<JsonMap<String, JsonValue>>(
        &frontmatter,
        "inputs",
    )?
    .or(definition.inputs.clone())
    .unwrap_or_default();
    if let Some(overrides) = input_overrides {
        inputs.extend(overrides.clone());
    }
    if inputs.is_empty() {
        frontmatter.remove("inputs");
    } else {
        frontmatter.insert("inputs".to_string(), serde_yml::to_value(&inputs)?);
    }

    let prompt = prompt_override
        .map(str::trim)
        .filter(|prompt| !prompt.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            frontmatter
                .get("prompt")
                .and_then(YamlValue::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_default();
    if prompt.is_empty() {
        frontmatter.remove("prompt");
    } else {
        frontmatter.insert("prompt".to_string(), YamlValue::String(prompt.clone()));
    }

    write_markdown(&uri, Some(&frontmatter), &pipeline_content, &context.config)?;
    emit_pipeline_resource_updated(session_id);
    Ok(prompt)
}
