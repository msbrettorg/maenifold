use crate::error::{MaenifoldError, Result};
use crate::mcp::stdio_pool::StdioPool;
use rmcp::model::Tool;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct ToolRoutingRegistry {
    /// tool_name -> server_name. None means native tool.
    routes: HashMap<String, String>,
    /// All gateway tools (cloned from pool discovery)
    gateway_tools: Vec<Tool>,
}

impl ToolRoutingRegistry {
    /// Build from native tools + gateway pool tools.
    /// Err on collision (same tool name from two sources).
    pub fn build(native_tool_names: &[String], pool: &StdioPool) -> Result<Self> {
        Self::build_from_sources(native_tool_names, pool.all_gateway_tools())
    }

    /// Look up which server owns a tool. None = native.
    pub fn lookup(&self, tool_name: &str) -> Option<&str> {
        self.routes.get(tool_name).map(String::as_str)
    }

    /// All gateway tools for tools/list response.
    pub fn gateway_tools(&self) -> &[Tool] {
        &self.gateway_tools
    }

    fn build_from_sources<I>(native_tool_names: &[String], gateway_sources: I) -> Result<Self>
    where
        I: IntoIterator<Item = (String, Vec<Tool>)>,
    {
        let native_set: HashSet<&str> = native_tool_names.iter().map(String::as_str).collect();
        let mut routes = HashMap::new();
        let mut gateway_tools = Vec::new();

        for (server_name, tools) in gateway_sources {
            for tool in tools {
                let name = tool.name.to_string();

                if native_set.contains(name.as_str()) {
                    return Err(MaenifoldError::Message(format!(
                        "Tool name collision: '{}' is provided by both native and gateway server '{}'",
                        name, server_name
                    )));
                }

                if let Some(existing_server) = routes.get(name.as_str()) {
                    return Err(MaenifoldError::Message(format!(
                        "Tool name collision: '{}' is provided by both '{}' and '{}'",
                        name, existing_server, server_name
                    )));
                }

                routes.insert(name, server_name.clone());
                gateway_tools.push(tool);
            }
        }

        gateway_tools.sort_by(|left, right| left.name.to_string().cmp(&right.name.to_string()));

        Ok(Self {
            routes,
            gateway_tools,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::ToolRoutingRegistry;
    use rmcp::model::Tool;
    use serde_json::{Map, Value};
    use std::sync::Arc;

    fn tool(name: &str) -> Tool {
        Tool::new(
            name.to_string(),
            Some(format!("{name} description")),
            Arc::new(Map::<String, Value>::new()),
        )
    }

    #[test]
    fn builds_routes_for_gateway_tools() {
        let registry = ToolRoutingRegistry::build_from_sources(
            &["native-tool".to_string()],
            vec![
                ("server-a".to_string(), vec![tool("gateway-a")]),
                ("server-b".to_string(), vec![tool("gateway-b")]),
            ],
        )
        .expect("registry should build");

        assert_eq!(registry.lookup("gateway-a"), Some("server-a"));
        assert_eq!(registry.lookup("gateway-b"), Some("server-b"));
        assert_eq!(registry.lookup("native-tool"), None);
        assert_eq!(
            registry
                .gateway_tools()
                .iter()
                .map(|tool| tool.name.to_string())
                .collect::<Vec<_>>(),
            vec!["gateway-a".to_string(), "gateway-b".to_string()]
        );
    }

    #[test]
    fn rejects_native_gateway_collisions() {
        let error = ToolRoutingRegistry::build_from_sources(
            &["shared-tool".to_string()],
            vec![("server-a".to_string(), vec![tool("shared-tool")])],
        )
        .expect_err("native and gateway collisions should fail");

        assert_eq!(
            error.to_string(),
            "Tool name collision: 'shared-tool' is provided by both native and gateway server 'server-a'"
        );
    }

    #[test]
    fn rejects_gateway_gateway_collisions() {
        let error = ToolRoutingRegistry::build_from_sources(
            &[],
            vec![
                ("server-a".to_string(), vec![tool("shared-tool")]),
                ("server-b".to_string(), vec![tool("shared-tool")]),
            ],
        )
        .expect_err("gateway collisions should fail");

        assert_eq!(
            error.to_string(),
            "Tool name collision: 'shared-tool' is provided by both 'server-a' and 'server-b'"
        );
    }
}
