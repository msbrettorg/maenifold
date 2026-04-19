use crate::error::{MaenifoldError, Result};
use rmcp::model::{ListResourcesResult, ReadResourceResult};

mod asset;
pub mod agent;
pub mod gate;
pub mod graph;
pub mod pipeline;
pub mod subscription;
mod subscription_worker;
pub mod workflow;

pub use subscription::{AssetWatcherState, start_asset_watcher};

// T-12.8: RTM FR-11 — resources/list assembly now lives in mcp/resources.
pub fn handle_list_resources() -> Result<ListResourcesResult> {
    asset::list_resources().map(|resources| ListResourcesResult {
        resources,
        next_cursor: None,
        meta: None,
    })
}

// T-12.0: RTM FR-11 / RTM-070 — preserve existing asset:// read behavior.
pub fn handle_read_resource(uri: &str) -> Result<ReadResourceResult> {
    asset::handle_read_resource(uri)
}

pub fn map_resource_error(err: MaenifoldError) -> rmcp::ErrorData {
    match err {
        // T-10.2: RTM FR-11 / RTM-070 — missing assets map to MCP resource-not-found.
        MaenifoldError::AssetNotFound(kind, id) => {
            rmcp::ErrorData::resource_not_found(format!("Asset not found: {kind}/{id}"), None)
        }
        // T-10.2: RTM FR-11 / RTM-069 / RTM-070 — malformed URIs stay invalid-params errors.
        MaenifoldError::UnknownTool(_)
        | MaenifoldError::InvalidPayload(_)
        | MaenifoldError::InvalidArgument(_)
        | MaenifoldError::InvalidResourceUri(_)
        | MaenifoldError::PathTraversal(_)
        | MaenifoldError::MissingProperty(_) => {
            rmcp::ErrorData::invalid_params(err.to_string(), None)
        }
        _ => rmcp::ErrorData::internal_error(err.to_string(), None),
    }
}
