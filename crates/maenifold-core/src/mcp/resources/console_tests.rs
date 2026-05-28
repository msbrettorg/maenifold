use super::*;
use crate::mcp::resources::handle_list_resources;
use base64::{Engine, prelude::BASE64_STANDARD};
use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn write_console_fixture() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().expect("temp dir");
    let console_root = temp_dir.path().join("console");
    let assets_dir = console_root.join("assets");

    fs::create_dir_all(&assets_dir).expect("create assets dir");
    fs::write(
        console_root.join("index.html"),
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <title>console</title>
    <script type="module" crossorigin src="/assets/index-abc.js"></script>
    <link rel="stylesheet" crossorigin href="/assets/index-abc.css">
  </head>
  <body>
    <div id="root"></div>
  </body>
</html>
"#,
    )
    .expect("write index");
    fs::write(
        assets_dir.join("index-abc.js"),
        "console.log('console bundle');\n",
    )
    .expect("write js");
    fs::write(
        assets_dir.join("index-abc.css"),
        "body { color: rgb(1, 2, 3); }\n",
    )
    .expect("write css");

    (temp_dir, console_root)
}

#[test]
fn console_resource_returns_index_html_with_inlined_or_linked_bundle() {
    let (_temp_dir, console_root) = write_console_fixture();

    let result = super::build_console_response(&console_root, CONSOLE_UI_RESOURCE_URI)
        .expect("console resource should read");
    let Some(ResourceContents::TextResourceContents {
        text, mime_type, ..
    }) = result.contents.first()
    else {
        panic!("console resource should be text");
    };

    assert!(text.contains(r#"<div id="root"></div>"#));
    assert!(text.contains(CONSOLE_ASSET_RESOURCE_PREFIX));
    assert_eq!(
        mime_type.as_deref(),
        Some(crate::mcp::tool_utils::MCP_APP_HTML_MIME_TYPE)
    );
}

#[test]
fn console_resource_advertises_text_html_mime() {
    let resource = super::list_resources()
        .into_iter()
        .find(|resource| resource.uri == CONSOLE_UI_RESOURCE_URI)
        .expect("console resource should be listed");

    assert_eq!(
        resource.mime_type.as_deref(),
        Some(crate::mcp::tool_utils::MCP_APP_HTML_MIME_TYPE)
    );
    assert!(
        resource
            .mime_type
            .as_deref()
            .is_some_and(|mime_type| mime_type.starts_with("text/html"))
    );
}

#[test]
fn console_asset_resource_serves_js_bundle() {
    let (_temp_dir, console_root) = write_console_fixture();

    let result = super::build_console_response(
        &console_root,
        &format!("{CONSOLE_ASSET_RESOURCE_PREFIX}index-abc.js"),
    )
    .expect("console js asset should read");
    let Some(ResourceContents::BlobResourceContents {
        blob, mime_type, ..
    }) = result.contents.first()
    else {
        panic!("console js asset should be blob");
    };

    let bytes = BASE64_STANDARD.decode(blob).expect("decode js bundle");
    assert_eq!(bytes, b"console.log('console bundle');\n");
    assert_eq!(mime_type.as_deref(), Some("application/javascript"));
}

#[test]
fn console_asset_resource_serves_css_bundle() {
    let (_temp_dir, console_root) = write_console_fixture();

    let result = super::build_console_response(
        &console_root,
        &format!("{CONSOLE_ASSET_RESOURCE_PREFIX}index-abc.css"),
    )
    .expect("console css asset should read");
    let Some(ResourceContents::BlobResourceContents {
        blob, mime_type, ..
    }) = result.contents.first()
    else {
        panic!("console css asset should be blob");
    };

    let bytes = BASE64_STANDARD.decode(blob).expect("decode css bundle");
    assert_eq!(bytes, b"body { color: rgb(1, 2, 3); }\n");
    assert_eq!(mime_type.as_deref(), Some("text/css"));
}

#[test]
fn console_asset_resource_rejects_path_traversal() {
    let (_temp_dir, console_root) = write_console_fixture();

    let error = super::build_console_response(
        &console_root,
        "ui://maenifold/console/assets/../../etc/passwd",
    )
    .expect_err("path traversal should fail");

    assert!(matches!(error, MaenifoldError::PathTraversal(_)));
}

#[test]
#[serial]
fn console_resource_appears_in_resources_list() {
    let (_temp_dir, _guard) = crate::mcp::resources::resources_tests_common::setup_test_env();

    let result = handle_list_resources().expect("resources list should load");
    let resource = result
        .resources
        .iter()
        .find(|resource| resource.uri == CONSOLE_UI_RESOURCE_URI)
        .expect("console resource should be listed");

    assert_eq!(
        resource.mime_type.as_deref(),
        Some(crate::mcp::tool_utils::MCP_APP_HTML_MIME_TYPE)
    );
}
