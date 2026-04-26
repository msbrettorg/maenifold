use crate::api;
use crate::error::{MaenifoldError, Result};
use rmcp::model::{AnnotateAble, ReadResourceResult, ResourceContents, ResourceTemplate};
use rusqlite::Connection;

pub(super) const COMMUNITY_TEMPLATE_URI: &str = "graph://community/{id}";
const COMMUNITY_URI_PREFIX: &str = "graph://community/";

pub(super) fn list_resource_templates() -> Vec<ResourceTemplate> {
    vec![
        rmcp::model::RawResourceTemplate {
            uri_template: COMMUNITY_TEMPLATE_URI.to_string(),
            name: "Graph Community Concepts".to_string(),
            title: Some("Graph Community Concepts".to_string()),
            description: Some(
                "Per-community concept graph with concept nodes and intra-community edges."
                    .to_string(),
            ),
            icons: None,
            mime_type: Some("application/json".to_string()),
        }
        .optional_annotate(None),
    ]
}

pub(super) fn read_community_resource(
    connection: &Connection,
    uri: &str,
) -> Result<ReadResourceResult> {
    let community_id = parse_community_id(uri)?;
    let graph = api::graph::get_community_concepts(connection, community_id)?;
    let body = serde_json::to_string_pretty(&graph)?;

    Ok(ReadResourceResult::new(vec![
        ResourceContents::text(body, uri.to_string()).with_mime_type("application/json"),
    ]))
}

fn parse_community_id(uri: &str) -> Result<i64> {
    let id = uri.strip_prefix(COMMUNITY_URI_PREFIX).ok_or_else(|| {
        MaenifoldError::InvalidResourceUri(format!(
            "{uri}. Expected URI to begin with '{COMMUNITY_URI_PREFIX}'"
        ))
    })?;

    if id.trim().is_empty() || id.contains('/') {
        return Err(MaenifoldError::InvalidResourceUri(format!(
            "{uri}. community id is required"
        )));
    }

    id.parse::<i64>().map_err(|error| {
        MaenifoldError::InvalidResourceUri(format!(
            "{uri}. invalid community id '{id}': {error}"
        ))
    })
}
