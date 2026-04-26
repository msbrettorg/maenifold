use crate::error::{MaenifoldError, Result};
use crate::mcp::gateway_consolidation::ConsolidatedServerTool;
use crate::mcp::list_mcp_tools;
use crate::mcp::tool_routing::ToolRoutingRegistry;
use crate::registry::ToolRegistry;
use rmcp::model::Tool;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;

const NATIVE_DOMAIN: &str = "maenifold";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DomainEntry {
    pub name: String,
    pub description: String,
    pub command_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct CommandEntry {
    pub name: String,
    pub description: String,
    pub schema: Value,
    pub safety_class: String,
}

#[derive(Debug, Clone)]
pub(crate) struct GatewayCatalog {
    domains: BTreeMap<String, CatalogDomain>,
}

#[derive(Debug, Clone)]
struct CatalogDomain {
    description: String,
    commands: BTreeMap<String, CatalogCommand>,
}

#[derive(Debug, Clone)]
struct CatalogCommand {
    entry: CommandEntry,
    target: CommandTarget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommandTarget {
    Native {
        tool_name: String,
    },
    Upstream {
        server_name: String,
        tool_name: String,
    },
}

impl GatewayCatalog {
    pub(crate) fn build(
        consolidated: &[ConsolidatedServerTool],
        passthrough: &[Tool],
        routing: &ToolRoutingRegistry,
    ) -> Result<Self> {
        let mut catalog = Self {
            domains: BTreeMap::new(),
        };
        catalog.add_native_tools()?;
        for server in consolidated {
            for tool in &server.tool_catalog {
                catalog.add_upstream_tool(&server.server_name, tool)?;
            }
        }
        for tool in passthrough {
            let name = tool.name.to_string();
            if let Some(server_name) = routing.lookup(&name) {
                catalog.add_upstream_tool(server_name, tool)?;
            }
        }
        Ok(catalog)
    }

    pub(crate) fn domains(&self) -> Vec<DomainEntry> {
        self.domains
            .iter()
            .map(|(name, domain)| DomainEntry {
                name: name.clone(),
                description: domain.description.clone(),
                command_count: domain.commands.len(),
            })
            .collect()
    }

    pub(crate) fn commands(&self, domain: &str) -> Result<Vec<CommandEntry>> {
        let domain = self.domain(domain)?;
        Ok(domain.commands.values().map(|cmd| cmd.entry.clone()).collect())
    }

    pub(crate) fn resolve(&self, domain: &str, command: &str) -> Result<CommandTarget> {
        Ok(self.command_record(domain, command)?.target.clone())
    }

    pub(crate) fn render_index(
        &self,
        domain: Option<&str>,
        command: Option<&str>,
        include_schemas: bool,
        include_examples: bool,
    ) -> Result<Value> {
        let mut value = match (domain, command) {
            (Some(domain), Some(command)) => {
                json!({ "domain": domain, "command": self.command_record(domain, command)?.entry.clone() })
            }
            (Some(domain), None) => json!({
                "domain": domain,
                "commands": self.commands(domain)?,
            }),
            (None, None) => json!({ "domains": self.domains() }),
            (None, Some(_)) => {
                return Err(MaenifoldError::GatewayInvalidParams(
                    "GatewayIndex command filter requires a domain.".to_string(),
                ));
            }
        };
        if !include_schemas {
            strip_schemas(&mut value);
        }
        if include_examples {
            value["examples"] = json!([]);
        }
        Ok(value)
    }

    fn add_native_tools(&mut self) -> Result<()> {
        let tools = list_mcp_tools(ToolRegistry::global())?;
        let domain = self.insert_domain(
            NATIVE_DOMAIN,
            "Native Maenifold commands backed by typed handlers.",
        )?;
        for tool in tools {
            let entry = CommandEntry {
                name: tool.name.clone(),
                description: tool.description,
                schema: tool.input_schema,
                safety_class: safety_class(&tool.name).to_string(),
            };
            domain.commands.insert(
                tool.name.clone(),
                CatalogCommand {
                    entry,
                    target: CommandTarget::Native {
                        tool_name: tool.name,
                    },
                },
            );
        }
        Ok(())
    }

    fn add_upstream_tool(&mut self, server_name: &str, tool: &Tool) -> Result<()> {
        let domain = self.insert_domain(server_name, "Gateway MCP server commands.")?;
        let name = tool.name.to_string();
        domain.commands.insert(
            name.clone(),
            CatalogCommand {
                entry: command_entry(tool),
                target: CommandTarget::Upstream {
                    server_name: server_name.to_string(),
                    tool_name: name,
                },
            },
        );
        Ok(())
    }

    fn insert_domain(&mut self, name: &str, description: &str) -> Result<&mut CatalogDomain> {
        Ok(self
            .domains
            .entry(name.to_string())
            .or_insert_with(|| CatalogDomain {
                description: description.to_string(),
                commands: BTreeMap::new(),
            }))
    }

    fn domain(&self, domain: &str) -> Result<&CatalogDomain> {
        self.domains.get(domain).ok_or_else(|| {
            MaenifoldError::GatewayInvalidParams(format!("Unknown gateway domain '{domain}'."))
        })
    }

    fn command_record(&self, domain: &str, command: &str) -> Result<&CatalogCommand> {
        self.domain(domain)?.commands.get(command).ok_or_else(|| {
            MaenifoldError::GatewayInvalidParams(format!(
                "Unknown gateway command '{command}' for domain '{domain}'."
            ))
        })
    }
}

fn command_entry(tool: &Tool) -> CommandEntry {
    let name = tool.name.to_string();
    CommandEntry {
        name: name.clone(),
        description: tool.description.as_deref().unwrap_or("").to_string(),
        schema: Value::Object((*tool.input_schema).clone()),
        safety_class: safety_class(&name).to_string(),
    }
}

fn safety_class(name: &str) -> &str {
    match name {
        name if name.starts_with("Read")
            || name.starts_with("List")
            || name.starts_with("Get")
            || name.starts_with("Search")
            || name.starts_with("Find")
            || name.starts_with("Build")
            || name.starts_with("Visualize")
            || name.starts_with("Extract") =>
        {
            "read"
        }
        name if name.starts_with("Delete") || name.starts_with("Move") => "destructive",
        _ => "mutating",
    }
}

fn strip_schemas(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.remove("schema");
            for child in map.values_mut() {
                strip_schemas(child);
            }
        }
        Value::Array(items) => {
            for item in items {
                strip_schemas(item);
            }
        }
        _ => {}
    }
}
