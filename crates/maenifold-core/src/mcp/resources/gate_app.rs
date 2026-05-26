use crate::error::{MaenifoldError, Result};
use crate::mcp::tool_utils::{GATE_UI_RESOURCE_URI, MCP_APP_HTML_MIME_TYPE};
use rmcp::model::{AnnotateAble, RawResource, ReadResourceResult, Resource, ResourceContents};
use std::sync::OnceLock;

const GATE_UI_MIME_TYPE: &str = MCP_APP_HTML_MIME_TYPE;
const INDEX_HTML: &str = include_str!("../../../../../assets/gate-app/index.html");
const APP_CSS: &str = include_str!("../../../../../assets/gate-app/app.css");
const APP_JS: &str = include_str!("../../../../../assets/gate-app/app.js");
const STYLE_PLACEHOLDER: &str = "<!-- MAENIFOLD_STYLE -->";
const SCRIPT_PLACEHOLDER: &str = "<!-- MAENIFOLD_SCRIPT -->";

fn compose_html() -> &'static str {
    static HTML: OnceLock<String> = OnceLock::new();
    HTML.get_or_init(|| {
        INDEX_HTML
            .replace(STYLE_PLACEHOLDER, &format!("<style>\n{APP_CSS}\n  </style>"))
            .replace(SCRIPT_PLACEHOLDER, &format!("<script>\n{APP_JS}\n  </script>"))
    })
    .as_str()
}

pub(super) fn list_resources() -> Vec<Resource> {
    vec![
        RawResource::new(GATE_UI_RESOURCE_URI, "Gate MCP App")
            .with_title("Gate Board")
            .with_description("Interactive MCP App for inspecting Gate kanban status.")
            .with_mime_type(GATE_UI_MIME_TYPE)
            .optional_annotate(None),
    ]
}

pub(super) fn is_gate_app_uri(uri: &str) -> bool {
    uri == GATE_UI_RESOURCE_URI
}

pub(super) fn handle_read_resource(uri: &str) -> Result<ReadResourceResult> {
    if !is_gate_app_uri(uri) {
        return Err(MaenifoldError::InvalidResourceUri(format!(
            "{uri}. Expected '{GATE_UI_RESOURCE_URI}'"
        )));
    }
    Ok(ReadResourceResult::new(vec![
        ResourceContents::text(compose_html(), uri.to_string()).with_mime_type(GATE_UI_MIME_TYPE),
    ]))
}

#[cfg(test)]
mod tests {
    use super::compose_html;

    const EXPECTED_SNAPSHOT: &str = include_str!("../../../../../assets/gate-app/snapshot.html");

    #[test]
    fn html_composition_is_byte_identical_to_snapshot() {
        assert_eq!(compose_html(), EXPECTED_SNAPSHOT);
    }

    #[test]
    fn composed_gate_html_includes_kanban_grid() {
        assert!(compose_html().contains(r#"id="kanban-grid""#));
    }

    #[test]
    fn composed_gate_html_has_no_hardcoded_stage_array() {
        assert!(!compose_html().contains(
            r#"["pending", "swe", "red-team", "blue-team", "done"]"#,
        ));
    }

    #[test]
    fn composed_gate_html_no_drawflow() {
        assert!(!compose_html().contains("drawflow.min.js"));
    }
}
