use crate::error::{MaenifoldError, Result};
use crate::mcp::domain_mapping;
use crate::mcp::gateway_catalog_render::{command_entry, strip_schemas};
use crate::mcp::gateway_consolidation::ConsolidatedServerTool;
use crate::mcp::gateway_gate;
use crate::mcp::gateway_metadata;
use crate::mcp::gateway_tasks;
use crate::mcp::list_mcp_tools;
use crate::mcp::tool_routing::ToolRoutingRegistry;
use crate::registry::ToolRegistry;
use rmcp::model::Tool;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[path = "gateway_catalog_helpers.rs"]
mod helpers;

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
    Gate {
        command: String,
    },
    Tasks {
        command: String,
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
        Self::build_with_upstream_descriptions(consolidated, passthrough, routing, &BTreeMap::new())
    }

    pub(crate) fn build_with_upstream_descriptions(
        consolidated: &[ConsolidatedServerTool],
        passthrough: &[Tool],
        routing: &ToolRoutingRegistry,
        upstream_descriptions: &BTreeMap<String, String>,
    ) -> Result<Self> {
        let mut catalog = Self {
            domains: BTreeMap::new(),
        };
        catalog.add_native_tools()?;
        catalog.add_gate_tools()?;
        catalog.add_tasks_tools()?;
        for server in consolidated {
            let description = gateway_metadata::upstream_domain_description(
                &server.server_name,
                upstream_descriptions,
            );
            for tool in &server.tool_catalog {
                catalog.add_upstream_tool(&server.server_name, tool, &description)?;
            }
        }
        for tool in passthrough {
            let name = tool.name.to_string();
            if let Some(server_name) = routing.lookup(&name) {
                let description = gateway_metadata::upstream_domain_description(
                    server_name,
                    upstream_descriptions,
                );
                catalog.add_upstream_tool(server_name, tool, &description)?;
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
        Ok(domain
            .commands
            .values()
            .map(|cmd| cmd.entry.clone())
            .collect())
    }

    pub(crate) fn resolve(&self, domain: &str, command: &str) -> Result<CommandTarget> {
        Ok(self.command_record(domain, command)?.target.clone())
    }

    #[allow(dead_code)]
    pub(crate) fn validate_candidate(
        &self,
        domain: &str,
        command: &str,
        safety_class: &str,
    ) -> Result<bool> {
        let record = self.command_record(domain, command)?;
        Ok(record.entry.safety_class == safety_class)
    }

    /// Return the canonical safety_class for (domain, command), as registered
    /// in the catalog. Used by the intent resolver to snap any LLM-emitted
    /// safety_class to the registry value instead of failing the whole
    /// candidate when the model paraphrases.
    pub(crate) fn canonical_safety_class(
        &self,
        domain: &str,
        command: &str,
    ) -> Result<String> {
        let record = self.command_record(domain, command)?;
        Ok(record.entry.safety_class.clone())
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
        for tool in tools {
            let Some(domain_name) = domain_mapping::tool_domain(&tool.name) else {
                continue;
            };
            let domain = self.insert_domain(
                domain_name,
                gateway_metadata::native_domain_description(domain_name),
            )?;
            let entry = CommandEntry {
                name: tool.name.clone(),
                description: tool.description,
                schema: tool.input_schema,
                safety_class: domain_mapping::tool_safety(&tool.name).to_string(),
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

    fn add_gate_tools(&mut self) -> Result<()> {
        let domain =
            self.insert_domain(gateway_gate::DOMAIN_NAME, gateway_gate::DOMAIN_DESCRIPTION)?;
        for command in gateway_gate::commands() {
            let name = command.name.clone();
            domain.commands.insert(
                name.clone(),
                CatalogCommand {
                    target: CommandTarget::Gate { command: name },
                    entry: command,
                },
            );
        }
        Ok(())
    }

    fn add_tasks_tools(&mut self) -> Result<()> {
        let domain = self.insert_domain(
            gateway_tasks::DOMAIN_NAME,
            gateway_tasks::DOMAIN_DESCRIPTION,
        )?;
        for command in gateway_tasks::commands() {
            let name = command.name.clone();
            domain.commands.insert(
                name.clone(),
                CatalogCommand {
                    target: CommandTarget::Tasks { command: name },
                    entry: command,
                },
            );
        }
        Ok(())
    }

    fn add_upstream_tool(
        &mut self,
        server_name: &str,
        tool: &Tool,
        domain_description: &str,
    ) -> Result<()> {
        let domain = self.insert_domain(server_name, domain_description)?;
        let name = tool.name.to_string();
        let safety = if server_name == "gate" {
            domain_mapping::gate_tool_safety(&name)
        } else {
            domain_mapping::tool_safety(&name)
        };
        domain.commands.insert(
            name.clone(),
            CatalogCommand {
                entry: command_entry(tool, safety.to_string()),
                target: CommandTarget::Upstream {
                    server_name: server_name.to_string(),
                    tool_name: name,
                },
            },
        );
        Ok(())
    }
}
