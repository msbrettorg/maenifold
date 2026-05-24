use crate::error::{MaenifoldError, Result};
use crate::mcp::stdio_pool::{ManagedServer, StdioPool};
use futures_util::stream::{FuturesUnordered, StreamExt};
use rmcp::model::Tool;
use std::collections::{BTreeMap, HashMap, HashSet};

pub(super) fn collect_gateway_tools(
    servers: &HashMap<String, ManagedServer>,
) -> Result<(
    Vec<Tool>,
    Vec<(String, Vec<Tool>)>,
    BTreeMap<String, String>,
)> {
    let mut tools = Vec::new();
    let mut gateway_tools = Vec::new();
    let mut gateway_descriptions = BTreeMap::new();
    for (server_name, server) in servers {
        let Some(session) = &server.session else {
            continue;
        };
        if let Some(description) = session.initialize_description() {
            gateway_descriptions.insert(server_name.clone(), description);
        }
        let mut server_tools = session.tools().iter().cloned().collect::<Vec<_>>();
        server_tools.sort_by(|left, right| left.name.to_string().cmp(&right.name.to_string()));
        ensure_unique_server_tools(server_name, &server_tools)?;
        tools.extend(server_tools.iter().cloned());
        gateway_tools.push((server_name.clone(), server_tools));
    }
    gateway_tools.sort_by(|(left, _), (right, _)| left.cmp(right));
    tools.sort_by(|left, right| {
        left.name
            .to_string()
            .cmp(&right.name.to_string())
            .then_with(|| left.description.cmp(&right.description))
    });
    Ok((tools, gateway_tools, gateway_descriptions))
}

pub(super) fn ensure_replacement_tools(
    servers: &HashMap<String, ManagedServer>,
    replaced_server: &str,
    replacement_tools: &[Tool],
) -> Result<()> {
    let _ = servers;
    ensure_unique_server_tools(replaced_server, replacement_tools)
}

pub(super) fn all_gateway_tools(pool: &StdioPool) -> Vec<(String, Vec<Tool>)> {
    pool.gateway_tools
        .read()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}

pub(super) fn all_gateway_descriptions(pool: &StdioPool) -> BTreeMap<String, String> {
    pool.gateway_descriptions
        .read()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}

pub(super) async fn shutdown_servers(servers: HashMap<String, ManagedServer>) {
    let mut shutdowns = FuturesUnordered::new();
    for server in servers.into_values() {
        if let Some(session) = server.session {
            shutdowns.push(async move { session.shutdown().await });
        }
    }
    while shutdowns.next().await.is_some() {}
}

fn ensure_unique_server_tools(server_name: &str, tools: &[Tool]) -> Result<()> {
    let mut names = HashSet::<String>::new();
    for tool in tools {
        let tool_name = tool.name.to_string();
        if !names.insert(tool_name.clone()) {
            return Err(MaenifoldError::Message(format!(
                "MCP tool name collision within gateway server '{server_name}': '{tool_name}'"
            )));
        }
    }
    Ok(())
}
