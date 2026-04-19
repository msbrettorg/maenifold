use crate::config::Config;
use crate::db;
use crate::error::{MaenifoldError, Result};
use rmcp::model::{ReadResourceResult, Resource};

#[path = "graph_concept.rs"]
mod graph_concept;
#[path = "graph_corpus.rs"]
mod graph_corpus;

pub(super) fn list_resources() -> Vec<Resource> {
    graph_corpus::list_resources()
}

pub(super) fn list_resource_templates() -> Vec<rmcp::model::ResourceTemplate> {
    graph_concept::list_resource_templates()
}

pub(super) fn is_graph_uri(uri: &str) -> bool {
    uri.starts_with("graph://")
}

pub(super) fn handle_read_resource(uri: &str) -> Result<ReadResourceResult> {
    let config = Config::current();
    let connection = db::open_connection(&config.database_path)?;

    graph_corpus::read_or_route_to_concept(uri, &connection).map_err(|err| match err {
        graph_corpus::GraphReadRouteError::NotHandled => MaenifoldError::InvalidResourceUri(
            format!(
                "{uri}. Expected one of '{}', '{}', '{}', or '{}'",
                graph_corpus::CORPUS_SNAPSHOT_URI,
                graph_corpus::CORPUS_COMMUNITIES_URI,
                graph_corpus::GRAPH_STATS_URI,
                graph_concept::CONCEPT_TEMPLATE_URI
            ),
        ),
        graph_corpus::GraphReadRouteError::ResourceError(error) => error,
    })
}
