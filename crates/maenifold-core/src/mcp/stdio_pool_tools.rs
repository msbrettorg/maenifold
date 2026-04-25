use crate::error::{MaenifoldError, Result};
use crate::mcp::stdio_pool::ManagedServer;
use futures_util::stream::{FuturesUnordered, StreamExt};
use rmcp::model::Tool;
use std::collections::HashMap;

pub(super) fn collect_gateway_tools(
    servers: &HashMap<String, ManagedServer>,
) -> Result<(Vec<Tool>, Vec<(String, Vec<Tool>)>)> {
    let mut owners = HashMap::<String, String>::new();
    let mut tools = Vec::new();
    let mut gateway_tools = Vec::new();
    for (server_name, server) in servers {
        let Some(session) = &server.session else {
            continue;
        };
        let mut server_tools = session.tools().iter().cloned().collect::<Vec<_>>();
        server_tools.sort_by(|left, right| left.name.to_string().cmp(&right.name.to_string()));
        for tool in &server_tools {
            let tool_name = tool.name.to_string();
            if let Some(other) = owners.insert(tool_name.clone(), server_name.clone()) {
                return Err(MaenifoldError::Message(format!(
                    "MCP tool name collision: '{tool_name}' provided by both '{other}' and '{server_name}'"
                )));
            }
            tools.push(tool.clone());
        }
        gateway_tools.push((server_name.clone(), server_tools));
    }
    gateway_tools.sort_by(|(left, _), (right, _)| left.cmp(right));
    tools.sort_by(|left, right| left.name.to_string().cmp(&right.name.to_string()));
    Ok((tools, gateway_tools))
}

pub(super) fn ensure_replacement_tools(
    servers: &HashMap<String, ManagedServer>,
    replaced_server: &str,
    replacement_tools: &[Tool],
) -> Result<()> {
    let mut owners = HashMap::<String, String>::new();
    for (server_name, server) in servers {
        if server_name == replaced_server {
            continue;
        }
        let Some(session) = &server.session else {
            continue;
        };
        for tool in session.tools().iter() {
            owners.insert(tool.name.to_string(), server_name.clone());
        }
    }
    for tool in replacement_tools {
        let tool_name = tool.name.to_string();
        if let Some(other) = owners.insert(tool_name.clone(), replaced_server.to_string()) {
            return Err(MaenifoldError::Message(format!(
                "MCP tool name collision: '{tool_name}' provided by both '{other}' and '{replaced_server}'"
            )));
        }
    }
    Ok(())
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
