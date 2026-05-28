use crate::config::Config;
use crate::console_protocol::{ConsoleProtocolError, resolve_console_asset};
use crate::error::{MaenifoldError, Result};
use crate::mcp::tool_utils::MCP_APP_HTML_MIME_TYPE;
use base64::{Engine, prelude::BASE64_STANDARD};
use rmcp::model::{AnnotateAble, RawResource, ReadResourceResult, Resource, ResourceContents};
use std::path::Path;

pub const CONSOLE_UI_RESOURCE_URI: &str = "ui://maenifold/console";
pub const CONSOLE_ASSET_RESOURCE_PREFIX: &str = "ui://maenifold/console/assets/";

const CONSOLE_UI_MIME_TYPE: &str = MCP_APP_HTML_MIME_TYPE;

pub(super) fn list_resources() -> Vec<Resource> {
    vec![
        RawResource::new(CONSOLE_UI_RESOURCE_URI, "Console MCP App")
            .with_title("Console")
            .with_description("Interactive MCP App for the maenifold console bundle.")
            .with_mime_type(CONSOLE_UI_MIME_TYPE)
            .optional_annotate(None),
    ]
}

pub(super) fn is_console_uri(uri: &str) -> bool {
    uri == CONSOLE_UI_RESOURCE_URI || uri.starts_with(CONSOLE_ASSET_RESOURCE_PREFIX)
}

pub(super) fn handle_read_resource_with_config(
    uri: &str,
    config: &Config,
) -> Result<ReadResourceResult> {
    build_console_response(&config.assets_path.join("console"), uri)
}

fn build_console_response(assets_root: &Path, uri: &str) -> Result<ReadResourceResult> {
    if uri == CONSOLE_UI_RESOURCE_URI {
        let (bytes, _) = resolve_console_asset(assets_root, "index.html")
            .map_err(|err| map_console_error("index.html", err))?;
        let html = String::from_utf8(bytes).map_err(|err| {
            MaenifoldError::InvalidPayload(format!("console index.html is not valid UTF-8: {err}"))
        })?;
        return Ok(ReadResourceResult::new(vec![
            ResourceContents::text(rewrite_console_html_paths(html), uri.to_string())
                .with_mime_type(CONSOLE_UI_MIME_TYPE),
        ]));
    }

    if let Some(asset_name) = uri.strip_prefix(CONSOLE_ASSET_RESOURCE_PREFIX) {
        let request_path = format!("assets/{asset_name}");
        let (bytes, mime_type) = resolve_console_asset(assets_root, &request_path)
            .map_err(|err| map_console_error(&request_path, err))?;
        return Ok(ReadResourceResult::new(vec![
            ResourceContents::blob(BASE64_STANDARD.encode(bytes), uri.to_string())
                .with_mime_type(mime_type),
        ]));
    }

    Err(MaenifoldError::InvalidResourceUri(format!(
        "{uri}. Expected '{CONSOLE_UI_RESOURCE_URI}' or '{CONSOLE_ASSET_RESOURCE_PREFIX}<asset>'"
    )))
}

fn rewrite_console_html_paths(html: String) -> String {
    html.replace(
        r#"src="/assets/"#,
        &format!(r#"src="{CONSOLE_ASSET_RESOURCE_PREFIX}"#),
    )
    .replace(
        r#"href="/assets/"#,
        &format!(r#"href="{CONSOLE_ASSET_RESOURCE_PREFIX}"#),
    )
}

fn map_console_error(request_path: &str, err: ConsoleProtocolError) -> MaenifoldError {
    match err {
        ConsoleProtocolError::NotFound => {
            MaenifoldError::AssetNotFound("console".to_string(), request_path.to_string())
        }
        ConsoleProtocolError::PathTraversal => {
            MaenifoldError::PathTraversal(request_path.to_string())
        }
        ConsoleProtocolError::Io(error) => MaenifoldError::Io(error),
    }
}

#[cfg(test)]
#[path = "console_tests.rs"]
mod tests;
