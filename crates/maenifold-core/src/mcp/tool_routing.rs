use crate::error::{MaenifoldError, Result};
use crate::mcp::stdio_pool::StdioPool;
use rmcp::model::Tool;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct ToolRoutingRegistry {
    routes: HashMap<String, String>,
    gateway_tools: Vec<Tool>,
}

impl ToolRoutingRegistry {
    /// Build from native tools + gateway pool tools.
    /// Returns Err on tool name collision.
    pub fn build(native_tools: &[Tool], pool: &StdioPool) -> Result<Self> {
        Self::build_from_sources(native_tools, pool.all_gateway_tools())
    }

    /// Look up which server owns a tool. None = native tool.
    pub fn lookup(&self, tool_name: &str) -> Option<&str> {
        self.routes.get(tool_name).map(String::as_str)
    }

    /// All gateway tools (for inclusion in tools/list response).
    pub fn gateway_tools(&self) -> &[Tool] {
        &self.gateway_tools
    }

    fn build_from_sources<I>(native_tools: &[Tool], gateway_sources: I) -> Result<Self>
    where
        I: IntoIterator<Item = (String, Vec<Tool>)>,
    {
        let native_names: HashSet<String> = native_tools.iter().map(tool_name).collect();
        let mut routes = HashMap::<String, String>::new();
        let mut gateway_tools = Vec::new();

        for (server_name, tools) in gateway_sources {
            for tool in tools {
                let name = tool_name(&tool);
                if native_names.contains(&name) {
                    return Err(collision(&name, "native", &server_name));
                }
                if let Some(other_server) = routes.get(&name) {
                    return Err(collision(&name, other_server, &server_name));
                }
                routes.insert(name, server_name.clone());
                gateway_tools.push(tool);
            }
        }

        Ok(Self {
            routes,
            gateway_tools,
        })
    }
}

fn tool_name(tool: &Tool) -> String {
    tool.name.to_string()
}

fn collision(tool: &str, server_a: &str, server_b: &str) -> MaenifoldError {
    MaenifoldError::Message(format!(
        "Tool name collision: '{tool}' is provided by both '{server_a}' and '{server_b}'"
    ))
}

#[cfg(test)]
mod tests {
    use super::ToolRoutingRegistry;
    use crate::mcp::stdio_pool::{StdioPool, StdioServerConfig};
    use rmcp::model::Tool;
    use serde_json::{Map, Value};
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;

    const PYTHON: &str = "/usr/bin/python3";
    const GOOD_SERVER: &str = r#"
import json, sys
tool_name = sys.argv[1]
def read_message():
    while True:
        line = sys.stdin.readline()
        if line == "": return None
        line = line.strip()
        if line: return json.loads(line)
def write_message(message):
    sys.stdout.write(json.dumps(message, separators=(",", ":")) + "\n")
    sys.stdout.flush()
while True:
    message = read_message()
    if message is None: break
    method = message.get("method")
    msg_id = message.get("id")
    if method == "initialize":
        write_message({"jsonrpc":"2.0","id":msg_id,"result":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"test","version":"0.0.1"}}})
    elif method == "notifications/initialized":
        continue
    elif method == "tools/list":
        write_message({"jsonrpc":"2.0","id":msg_id,"result":{"tools":[{"name":tool_name,"description":"desc","inputSchema":{"type":"object"}}]}})
    else:
        write_message({"jsonrpc":"2.0","id":msg_id,"error":{"code":-32601,"message":"unknown method"}})
"#;

    fn tool(name: &str) -> Tool {
        Tool::new(
            name.to_string(),
            format!("{name} description"),
            Arc::new(Map::<String, Value>::new()),
        )
    }

    fn server_config(name: &str, tool_name: &str) -> StdioServerConfig {
        StdioServerConfig {
            name: name.to_string(),
            command: PYTHON.to_string(),
            args: vec!["-u".to_string(), "-c".to_string(), GOOD_SERVER.to_string(), tool_name.to_string()],
            env: HashMap::new(),
            timeout: Some(Duration::from_secs(2)),
        }
    }

    #[test]
    fn empty_sources_return_empty_registry_and_missing_lookup() {
        let registry = ToolRoutingRegistry::build_from_sources(&[], Vec::<(String, Vec<Tool>)>::new())
            .expect("empty sources should build");

        assert_eq!(registry.lookup("missing-tool"), None);
        assert!(registry.gateway_tools().is_empty());
    }

    #[test]
    fn builds_gateway_routes() {
        let native_tools = vec![tool("native-tool")];
        let registry = ToolRoutingRegistry::build_from_sources(
            &native_tools,
            vec![
                ("server-a".to_string(), vec![tool("gateway-a")]),
                ("server-b".to_string(), vec![tool("gateway-b")]),
            ],
        )
        .expect("registry should build");

        assert_eq!(registry.lookup("native-tool"), None);
        assert_eq!(registry.lookup("gateway-a"), Some("server-a"));
        assert_eq!(registry.lookup("gateway-b"), Some("server-b"));
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
    fn rejects_native_gateway_collision() {
        let native_tools = vec![tool("shared-tool")];
        let error = ToolRoutingRegistry::build_from_sources(
            &native_tools,
            vec![("server-a".to_string(), vec![tool("shared-tool")])],
        )
        .expect_err("native/gateway collision should fail");

        assert_eq!(
            error.to_string(),
            "Tool name collision: 'shared-tool' is provided by both 'native' and 'server-a'"
        );
    }

    #[test]
    fn rejects_gateway_gateway_collision() {
        let error = ToolRoutingRegistry::build_from_sources(
            &[],
            vec![
                ("server-a".to_string(), vec![tool("shared-tool")]),
                ("server-b".to_string(), vec![tool("shared-tool")]),
            ],
        )
        .expect_err("gateway collision should fail");

        assert_eq!(
            error.to_string(),
            "Tool name collision: 'shared-tool' is provided by both 'server-a' and 'server-b'"
        );
    }

    #[test]
    fn gateway_tools_returns_only_gateway_tools() {
        let registry = ToolRoutingRegistry::build_from_sources(
            &[tool("native-a"), tool("native-b")],
            vec![(
                "server-a".to_string(),
                vec![tool("gateway-a"), tool("gateway-b")],
            )],
        )
        .expect("registry should build");

        assert_eq!(
            registry
                .gateway_tools()
                .iter()
                .map(|tool| tool.name.to_string())
                .collect::<Vec<_>>(),
            vec!["gateway-a".to_string(), "gateway-b".to_string()]
        );
    }

    #[tokio::test]
    async fn build_delegates_to_build_from_sources() {
        let native_tools = vec![tool("native-tool")];
        let pool = StdioPool::start(vec![server_config("server-a", "gateway-a")])
            .await
            .expect("pool should start");

        let from_build = ToolRoutingRegistry::build(&native_tools, &pool).expect("build should succeed");
        let from_sources = ToolRoutingRegistry::build_from_sources(&native_tools, pool.all_gateway_tools())
            .expect("build_from_sources should succeed");

        assert_eq!(from_build.lookup("native-tool"), from_sources.lookup("native-tool"));
        assert_eq!(from_build.lookup("gateway-a"), from_sources.lookup("gateway-a"));
        assert_eq!(
            from_build
                .gateway_tools()
                .iter()
                .map(|tool| tool.name.to_string())
                .collect::<Vec<_>>(),
            from_sources
                .gateway_tools()
                .iter()
                .map(|tool| tool.name.to_string())
                .collect::<Vec<_>>()
        );

        pool.shutdown().await;
    }
}
