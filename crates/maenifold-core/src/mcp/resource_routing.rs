use crate::error::{JSONRPC_INTERNAL_ERROR, MaenifoldError, Result};
use crate::mcp::stdio_pool::StdioPool;
use rmcp::model::{AnnotateAble, RawResource, Resource, Tool};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub(crate) struct ResourceRoutingRegistry {
    routes: HashMap<String, String>,
    gateway_resources: Vec<Resource>,
}

impl ResourceRoutingRegistry {
    pub(crate) fn build(pool: &StdioPool) -> Result<Self> {
        Self::build_from_tool_sources(pool.all_gateway_tools())
    }

    pub(crate) fn lookup(&self, uri: &str) -> Option<&str> {
        self.routes.get(uri).map(String::as_str)
    }

    pub(crate) fn gateway_resources(&self) -> &[Resource] {
        &self.gateway_resources
    }

    fn build_from_tool_sources<I>(gateway_sources: I) -> Result<Self>
    where
        I: IntoIterator<Item = (String, Vec<Tool>)>,
    {
        let mut routes = HashMap::<String, String>::new();
        let mut gateway_resources = Vec::new();

        for (server_name, tools) in gateway_sources {
            for tool in tools {
                let Some(uri) = ui_resource_uri(&tool) else {
                    continue;
                };
                if !uri.starts_with("ui://") {
                    continue;
                }
                if let Some(other_server) = routes.get(&uri) {
                    if other_server == &server_name {
                        continue;
                    }
                    return Err(collision(&uri, other_server, &server_name));
                }
                routes.insert(uri.clone(), server_name.clone());
                gateway_resources.push(resource_from_tool(&server_name, &tool, &uri));
            }
        }

        gateway_resources.sort_by(|left, right| left.uri.cmp(&right.uri));
        Ok(Self {
            routes,
            gateway_resources,
        })
    }
}

fn ui_resource_uri(tool: &Tool) -> Option<String> {
    let meta = tool.meta.as_ref()?;
    meta.0
        .get("ui")?
        .get("resourceUri")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn resource_from_tool(server_name: &str, tool: &Tool, uri: &str) -> Resource {
    let name = tool
        .title
        .clone()
        .unwrap_or_else(|| format!("{} UI", tool.name));
    let description = format!(
        "UI resource advertised by gateway server '{server_name}' for tool '{}'.",
        tool.name
    );
    RawResource::new(uri.to_string(), name)
        .with_description(description)
        .optional_annotate(None)
}

fn collision(uri: &str, server_a: &str, server_b: &str) -> MaenifoldError {
    MaenifoldError::Message(format!(
        "Gateway resource collision: '{uri}' is provided by both '{server_a}' and '{server_b}'"
    ))
}

pub(crate) fn map_gateway_resource_error(
    server: &str,
    uri: &str,
    err: MaenifoldError,
) -> rmcp::ErrorData {
    match err {
        MaenifoldError::JsonRpc {
            code: -32002,
            message,
            ..
        } => rmcp::ErrorData::resource_not_found(message, None),
        MaenifoldError::JsonRpc {
            code,
            message,
            data,
        } => rmcp::ErrorData::new(
            rmcp::model::ErrorCode(i32::try_from(code).unwrap_or(JSONRPC_INTERNAL_ERROR as i32)),
            format!("Gateway resource fetch failed for '{uri}' from '{server}': {message}"),
            Some(data),
        ),
        err => rmcp::ErrorData::internal_error(
            format!("Gateway resource fetch failed for '{uri}' from '{server}': {err}"),
            None,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::ResourceRoutingRegistry;
    use rmcp::model::{Meta, Tool};
    use serde_json::{Map, Value, json};
    use std::sync::Arc;

    fn tool(name: &str, uri: Option<&str>) -> Tool {
        let mut tool = Tool::new(
            name.to_string(),
            format!("{name} description"),
            Arc::new(Map::<String, Value>::new()),
        );
        if let Some(uri) = uri {
            tool.meta = Some(Meta(
                json!({"ui": {"resourceUri": uri}})
                    .as_object()
                    .cloned()
                    .expect("test metadata must be an object"),
            ));
        }
        tool
    }

    #[test]
    fn builds_routes_from_tool_meta_without_resources_list() {
        let registry = ResourceRoutingRegistry::build_from_tool_sources(vec![(
            "server-a".to_string(),
            vec![tool("dashboard", Some("ui://server-a/dashboard"))],
        )])
        .expect("registry should build from tool metadata only");

        assert_eq!(registry.lookup("ui://server-a/dashboard"), Some("server-a"));
        assert_eq!(
            registry
                .gateway_resources()
                .iter()
                .map(|resource| resource.uri.clone())
                .collect::<Vec<_>>(),
            vec!["ui://server-a/dashboard".to_string()]
        );
    }

    #[test]
    fn ignores_tools_without_ui_resource_meta() {
        let registry = ResourceRoutingRegistry::build_from_tool_sources(vec![(
            "server-a".to_string(),
            vec![tool("plain", None), tool("asset", Some("asset://not-ui"))],
        )])
        .expect("non-ui tools should not fail registry build");

        assert_eq!(registry.lookup("asset://not-ui"), None);
        assert!(registry.gateway_resources().is_empty());
    }

    #[test]
    fn rejects_duplicate_ui_resource_ownership() {
        let error = ResourceRoutingRegistry::build_from_tool_sources(vec![
            (
                "server-a".to_string(),
                vec![tool("dashboard-a", Some("ui://shared/dashboard"))],
            ),
            (
                "server-b".to_string(),
                vec![tool("dashboard-b", Some("ui://shared/dashboard"))],
            ),
        ])
        .expect_err("duplicate ui resource ownership should fail");

        assert_eq!(
            error.to_string(),
            "Gateway resource collision: 'ui://shared/dashboard' is provided by both 'server-a' and 'server-b'"
        );
    }
}
