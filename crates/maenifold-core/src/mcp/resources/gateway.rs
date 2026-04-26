use crate::error::{MaenifoldError, Result};
use crate::mcp::stdio_pool::{ServerStartupStatus, StdioPool};
use rmcp::model::{AnnotateAble, RawResource, RawResourceTemplate, ReadResourceResult, Resource, ResourceContents, ResourceTemplate};
use serde_json::json;
use std::sync::Arc;

const GATEWAY_DIAGNOSTICS_URI: &str = "gateway://diagnostics";
const GATEWAY_DOMAINS_URI: &str = "gateway://domains";
const GATEWAY_DOMAIN_PREFIX: &str = "gateway://domains/";
const GATEWAY_EXAMPLES_URI: &str = "gateway://examples";

pub(super) fn list_resources() -> Vec<Resource> {
    vec![
        RawResource::new(GATEWAY_DIAGNOSTICS_URI, "Gateway startup diagnostics")
            .with_title("Gateway startup diagnostics")
            .with_description("Per-server gateway startup health and sanitized stderr tails.")
            .with_mime_type("application/json")
            .optional_annotate(None),
        RawResource::new(GATEWAY_DOMAINS_URI, "Gateway domains")
            .with_title("Gateway domains")
            .with_description("Deterministic gateway domain catalog.")
            .with_mime_type("application/json")
            .optional_annotate(None),
        RawResource::new(GATEWAY_EXAMPLES_URI, "Gateway examples")
            .with_title("Gateway examples")
            .with_description("Curated gateway usage examples.")
            .with_mime_type("application/json")
            .optional_annotate(None),
    ]
}

pub(super) fn list_resource_templates() -> Vec<ResourceTemplate> {
    vec![
        template("gateway://domains/{domain}", "Gateway domain", "Gateway domain detail."),
        template("gateway://domains/{domain}/commands", "Gateway domain commands", "Commands available in a gateway domain."),
        template("gateway://domains/{domain}/commands/{command}", "Gateway command", "Gateway command detail and JSON schema."),
    ]
}

pub(super) fn is_gateway_uri(uri: &str) -> bool {
    uri.starts_with("gateway://")
}

pub(super) async fn handle_read_resource(
    uri: &str,
    diagnostics: Option<Arc<StdioPool>>,
) -> Result<ReadResourceResult> {
    if !is_gateway_uri(uri) {
        return Err(MaenifoldError::InvalidResourceUri(format!(
            "{uri}. Expected a gateway:// resource URI"
        )));
    }
    if uri != GATEWAY_DIAGNOSTICS_URI {
        return handle_catalog_read(uri);
    }

    let pool = diagnostics.ok_or_else(|| {
        MaenifoldError::InvalidArgument("gateway diagnostics are not available".to_string())
    })?;
    let health = pool.health().await;
    let servers = health
        .servers
        .into_iter()
        .map(|server| {
            json!({
                "name": server.name,
                "status": match server.status {
                    ServerStartupStatus::Started => "started",
                    ServerStartupStatus::Failed => "failed",
                },
                "toolCount": server.tool_count,
                "error": server.last_start_error,
                "stderrTail": server.stderr_tail,
            })
        })
        .collect::<Vec<_>>();
    let payload = json!({ "servers": servers });
    let text = serde_json::to_string_pretty(&payload)?;

    Ok(ReadResourceResult::new(vec![
        ResourceContents::text(text, uri.to_string()).with_mime_type("application/json"),
    ]))
}

fn handle_catalog_read(uri: &str) -> Result<ReadResourceResult> {
    let value = if let Some(catalog) = super::gateway_catalog() {
        if uri == GATEWAY_DOMAINS_URI {
            json!({ "domains": catalog.domains() })
        } else if uri == GATEWAY_EXAMPLES_URI {
            catalog.render_index(None, None, false, true)?
        } else if let Some((domain, command)) = parse_command_uri(uri)? {
            catalog
                .render_index(Some(domain), Some(command), true, false)
                .map_err(|_| not_found(uri))?
        } else if let Some(domain) = parse_domain_commands_uri(uri)? {
            json!({ "domain": domain, "commands": catalog.commands(domain).map_err(|_| not_found(uri))? })
        } else if let Some(domain) = parse_domain_uri(uri)? {
            catalog
                .render_index(Some(domain), None, false, false)
                .map_err(|_| not_found(uri))?
        } else {
            return Err(MaenifoldError::InvalidResourceUri(uri.to_string()));
        }
    } else {
        unavailable_payload(uri)?
    };
    read_json(uri, value)
}

fn parse_command_uri(uri: &str) -> Result<Option<(&str, &str)>> {
    let Some(rest) = uri.strip_prefix(GATEWAY_DOMAIN_PREFIX) else { return Ok(None); };
    let Some((domain, command)) = rest.split_once("/commands/") else { return Ok(None); };
    validate_component("domain", domain)?;
    validate_component("command", command)?;
    Ok(Some((domain, command)))
}

fn parse_domain_commands_uri(uri: &str) -> Result<Option<&str>> {
    let Some(rest) = uri.strip_prefix(GATEWAY_DOMAIN_PREFIX) else { return Ok(None); };
    let Some(domain) = rest.strip_suffix("/commands") else { return Ok(None); };
    validate_component("domain", domain)?;
    Ok(Some(domain))
}

fn parse_domain_uri(uri: &str) -> Result<Option<&str>> {
    let Some(domain) = uri.strip_prefix(GATEWAY_DOMAIN_PREFIX) else { return Ok(None); };
    validate_component("domain", domain)?;
    if domain.contains('/') {
        return Ok(None);
    }
    Ok(Some(domain))
}

fn unavailable_payload(uri: &str) -> Result<serde_json::Value> {
    if uri == GATEWAY_DOMAINS_URI {
        Ok(json!({ "available": false, "domains": [] }))
    } else if uri == GATEWAY_EXAMPLES_URI {
        Ok(json!({ "available": false, "examples": [] }))
    } else if uri.starts_with(GATEWAY_DOMAIN_PREFIX) {
        Ok(json!({ "available": false, "uri": uri }))
    } else {
        Err(MaenifoldError::InvalidResourceUri(uri.to_string()))
    }
}

fn validate_component(name: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() || value.contains('/') {
        return Err(MaenifoldError::InvalidResourceUri(format!(
            "Invalid gateway {name} URI component in resource path"
        )));
    }
    Ok(())
}

fn read_json(uri: &str, value: serde_json::Value) -> Result<ReadResourceResult> {
    Ok(ReadResourceResult::new(vec![
        ResourceContents::text(serde_json::to_string_pretty(&value)?, uri.to_string())
            .with_mime_type("application/json"),
    ]))
}

fn not_found(uri: &str) -> MaenifoldError {
    MaenifoldError::AssetNotFound("gateway".to_string(), uri.to_string())
}

fn template(uri_template: &str, name: &str, description: &str) -> ResourceTemplate {
    RawResourceTemplate::new(uri_template, name)
        .with_title(name.to_string())
        .with_description(description)
        .with_mime_type("application/json")
        .optional_annotate(None)
}
