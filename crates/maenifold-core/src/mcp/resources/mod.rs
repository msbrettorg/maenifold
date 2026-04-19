use crate::error::{MaenifoldError, Result};
use rmcp::model::{ListResourceTemplatesResult, ListResourcesResult, ReadResourceResult};
use rmcp::service::RequestContext;
use rmcp::RoleServer;

pub mod agent;
mod asset;
pub mod gate;
pub mod gate_history;
pub mod gate_status;
pub mod gate_task;
pub mod graph;
pub mod pipeline;
pub mod subscription;
mod subscription_worker;
pub mod throttle;
pub mod workflow;

pub use subscription::{notify_resource_updated, start_asset_watcher, AssetWatcherState};

// T-12.8: RTM FR-11 — resources/list assembly now lives in mcp/resources.
pub fn handle_list_resources() -> Result<ListResourcesResult> {
    let mut resources = asset::list_resources()?;
    resources.extend(agent::list_resources());
    resources.extend(gate::list_resources());
    resources.extend(graph::list_resources());
    resources.extend(pipeline::list_resources()?);
    resources.extend(workflow::list_resources());
    Ok(ListResourcesResult {
        resources,
        next_cursor: None,
        meta: None,
    })
}

pub fn handle_list_resource_templates() -> Result<ListResourceTemplatesResult> {
    let mut resource_templates = graph::list_resource_templates();
    resource_templates.extend(agent::list_resource_templates());
    resource_templates.extend(gate::list_resource_templates());
    resource_templates.extend(pipeline::list_resource_templates());
    resource_templates.extend(workflow::list_resource_templates());
    Ok(ListResourceTemplatesResult {
        resource_templates,
        next_cursor: None,
        meta: None,
    })
}

// T-12.0: RTM FR-11 / RTM-070 — preserve existing asset:// read behavior.
pub fn handle_read_resource(uri: &str) -> Result<ReadResourceResult> {
    if agent::is_agent_uri(uri) {
        agent::handle_read_resource(uri)
    } else if uri.starts_with("gate://") {
        gate::handle_read_resource(uri)
    } else if workflow::is_workflow_uri(uri) {
        workflow::handle_read_resource(uri)
    } else if graph::is_graph_uri(uri) {
        graph::handle_read_resource(uri)
    } else if pipeline::is_pipeline_uri(uri) {
        pipeline::handle_read_resource(uri)
    } else {
        asset::handle_read_resource(uri)
    }
}

pub fn handle_subscribe(
    uri: &str,
    context: &RequestContext<RoleServer>,
) -> std::result::Result<(), rmcp::ErrorData> {
    if !resource_exists(uri) {
        return Err(rmcp::ErrorData::resource_not_found(
            format!("Resource not found: {uri}"),
            None,
        ));
    }
    subscription::subscribe(uri, context)
}

pub fn handle_unsubscribe(
    uri: &str,
    context: &RequestContext<RoleServer>,
) -> std::result::Result<(), rmcp::ErrorData> {
    subscription::unsubscribe(uri, context);
    Ok(())
}

fn resource_exists(uri: &str) -> bool {
    if uri.starts_with("agent://session/") {
        return true;
    }

    if handle_list_resources()
        .map(|result| result.resources.iter().any(|resource| resource.uri == uri))
        .unwrap_or(false)
    {
        return true;
    }

    handle_list_resource_templates()
        .map(|result| {
            result
                .resource_templates
                .iter()
                .any(|template| uri_matches_template(uri, &template.uri_template))
        })
        .unwrap_or(false)
}

fn uri_matches_template(uri: &str, uri_template: &str) -> bool {
    if let Some((base, _)) = uri_template.split_once('{') {
        return !base.is_empty()
            && uri
                .strip_prefix(base)
                .map(|remainder| !remainder.is_empty())
                .unwrap_or(false);
    }

    uri == uri_template
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
