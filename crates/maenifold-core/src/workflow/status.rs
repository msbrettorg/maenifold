use crate::error::{MaenifoldError, Result};
use crate::markdown::{
    Frontmatter, append_h2_section, extract_wikilinks, read_markdown, write_markdown,
};
use crate::sessions::now_iso8601_roundtrip;
use crate::tools::ToolContext;
use crate::workflow::{format_workflow_step, read_workflow_definition, workflow_steps};
use serde_json::json;
use serde_yaml::Value as YamlValue;

// T-12.0: RTM NFR-1 keeps workflow completion/cancellation transitions isolated from session bootstrap helpers.

pub(crate) fn handle_completed_workflow_status(
    context: &ToolContext,
    workflow_uri: &str,
    frontmatter: Frontmatter,
    current_step: usize,
    conclusion: Option<&str>,
) -> Result<String> {
    let conclusion = match require_workflow_conclusion(conclusion, false) {
        Ok(conclusion) => conclusion,
        Err(message) => return Ok(message),
    };

    append_h2_section(workflow_uri, "Conclusion", conclusion, &context.config)?;
    finalize_workflow_status(
        context,
        workflow_uri,
        frontmatter,
        current_step,
        "completed",
        "completed",
    )?;
    Ok("✅ Workflow session completed".to_string())
}

pub(crate) fn handle_cancelled_workflow_status(
    context: &ToolContext,
    workflow_uri: &str,
    mut frontmatter: Frontmatter,
    existing_content: &str,
    current_step: usize,
) -> Result<String> {
    finalize_frontmatter_status(&mut frontmatter, current_step, "cancelled", "cancelled");
    write_markdown(
        workflow_uri,
        Some(&frontmatter),
        existing_content,
        &context.config,
    )?;
    Ok("❌ Workflow session cancelled".to_string())
}

pub(crate) fn handle_queue_exhaustion(
    context: &ToolContext,
    workflow_uri: &str,
    frontmatter: Frontmatter,
    current_step: usize,
    conclusion: Option<&str>,
) -> Result<String> {
    let conclusion = match require_workflow_conclusion(conclusion, true) {
        Ok(conclusion) => conclusion,
        Err(message) => return Ok(message),
    };

    append_h2_section(workflow_uri, "Conclusion", conclusion, &context.config)?;
    finalize_workflow_status(
        context,
        workflow_uri,
        frontmatter,
        current_step,
        "completed",
        "completed",
    )?;
    Ok("✅ All workflows in queue completed".to_string())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn transition_to_next_workflow(
    context: &ToolContext,
    workflow_uri: &str,
    mut frontmatter: Frontmatter,
    existing_content: &str,
    current_workflow_index: usize,
    queue: &[String],
) -> Result<String> {
    frontmatter.insert(
        "currentWorkflow".to_string(),
        YamlValue::Number(serde_yaml::Number::from(current_workflow_index as i64)),
    );
    frontmatter.insert(
        "currentStep".to_string(),
        YamlValue::Number(serde_yaml::Number::from(0)),
    );
    write_markdown(
        workflow_uri,
        Some(&frontmatter),
        existing_content,
        &context.config,
    )?;

    let next_workflow_id = queue.get(current_workflow_index).ok_or_else(|| {
        MaenifoldError::Message("Workflow queue transition index out of range".to_string())
    })?;
    let workflow_value = read_workflow_definition(&context.config, next_workflow_id)?;
    let next_steps = workflow_steps(&workflow_value, next_workflow_id)?;
    let next_step = next_steps.first().cloned().unwrap_or_else(|| json!({}));
    let rendered_step = format_workflow_step(&next_step)?;
    append_h2_section(
        workflow_uri,
        &format!("Step 1/{}", next_steps.len()),
        &rendered_step,
        &context.config,
    )?;

    Ok(format!("Step 1/{}\n\n{}", next_steps.len(), rendered_step))
}

fn require_workflow_conclusion(
    conclusion: Option<&str>,
    queue_exhausted: bool,
) -> std::result::Result<&str, String> {
    let conclusion = match conclusion {
        Some(conclusion) if !conclusion.is_empty() => conclusion,
        _ => {
            return Err(if queue_exhausted {
                "ERROR: All workflows complete. Call workflow tool again with conclusion parameter to finalize."
                    .to_string()
            } else {
                "ERROR: Conclusion required when completing session. Must synthesize findings with [[WikiLinks]]."
                    .to_string()
            });
        }
    };

    if extract_wikilinks(conclusion).is_empty() {
        return Err(
            "ERROR: Conclusion must include [[WikiLinks]] for knowledge graph integration."
                .to_string(),
        );
    }

    Ok(conclusion)
}

#[allow(clippy::too_many_arguments)]
fn finalize_workflow_status(
    context: &ToolContext,
    workflow_uri: &str,
    mut frontmatter: Frontmatter,
    current_step: usize,
    status: &str,
    timestamp_key: &str,
) -> Result<()> {
    finalize_frontmatter_status(&mut frontmatter, current_step, status, timestamp_key);

    let (_, content_after_update, _) = read_markdown(workflow_uri, &context.config)?;
    write_markdown(
        workflow_uri,
        Some(&frontmatter),
        &content_after_update,
        &context.config,
    )?;
    Ok(())
}

fn finalize_frontmatter_status(
    frontmatter: &mut Frontmatter,
    current_step: usize,
    status: &str,
    timestamp_key: &str,
) {
    frontmatter.insert("status".to_string(), YamlValue::String(status.to_string()));
    frontmatter.insert(
        timestamp_key.to_string(),
        YamlValue::String(now_iso8601_roundtrip()),
    );
    frontmatter.insert(
        "stepCount".to_string(),
        YamlValue::Number(serde_yaml::Number::from(current_step as i64)),
    );
}
