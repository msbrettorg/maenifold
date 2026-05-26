use super::context::{ToolContext, load_help_doc, render, required_string};
use crate::assets::{AssetKind, AssetManager};
use crate::error::Result;
use crate::mcp::resources;
use crate::util::json::{OutputFormat, success};
use rmcp::model::ResourceContents;
use serde_json::Value;

pub fn list_assets(context: &ToolContext, payload: &Value) -> Result<String> {
    if payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let help = load_help_doc(&context.config.assets_path, "ListAssets")
            .unwrap_or_else(|| "ERROR: Help file not found for ListAssets".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }

    let asset_manager = AssetManager::new(&context.config);
    let kind = payload
        .get("type")
        .and_then(Value::as_str)
        .map(AssetKind::parse)
        .transpose()?;
    let value = asset_manager.list_assets(kind)?;
    Ok(match context.output_format {
        // T-1.6: RTM NFR-1 - CLI JSON mode wraps rendered text in the C# envelope.
        OutputFormat::Json => success(render(OutputFormat::Markdown, value)),
        OutputFormat::Markdown => render(OutputFormat::Markdown, value),
    })
}

pub fn read_mcp_resource(context: &ToolContext, payload: &Value) -> Result<String> {
    if payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let help = load_help_doc(&context.config.assets_path, "ReadMcpResource")
            .unwrap_or_else(|| "ERROR: Help file not found for ReadMcpResource".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }

    let uri = required_string(payload, "uri")?;
    let value = resource_text(resources::handle_read_resource_with_config(
        uri,
        &context.config,
    )?);
    Ok(match context.output_format {
        OutputFormat::Json => success(value),
        OutputFormat::Markdown => value,
    })
}

fn resource_text(result: rmcp::model::ReadResourceResult) -> String {
    result
        .contents
        .into_iter()
        .find_map(|content| match content {
            ResourceContents::TextResourceContents { text, .. } => Some(text),
            ResourceContents::BlobResourceContents { blob, .. } => Some(blob),
        })
        .unwrap_or_default()
}
