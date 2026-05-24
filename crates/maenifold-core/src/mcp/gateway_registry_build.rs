use super::{
    CommandEntry, CommandTarget, GatewayRegistry, RegisteredCommand, RegisteredDomain,
    UpstreamServer,
};
use crate::error::{MaenifoldError, Result};
use crate::mcp::domain_mapping;
use crate::mcp::gateway_gate;
use crate::mcp::gateway_schema_validation::compile_validator;
use crate::mcp::gateway_tasks;
use crate::mcp::list_mcp_tools;
use crate::registry::ToolRegistry;
use rmcp::model::Tool;
use serde_json::Value;
use std::collections::BTreeMap;

impl GatewayRegistry {
    pub(crate) fn build(
        gateway_sources: Vec<(String, Vec<Tool>)>,
        upstream_descriptions: &BTreeMap<String, String>,
    ) -> Result<Self> {
        let mut registry = Self::default();
        registry.add_native_tools()?;
        registry.add_static_domain(
            gateway_gate::DOMAIN_NAME,
            gateway_gate::DOMAIN_DESCRIPTION,
            gateway_gate::commands(),
            |name| CommandTarget::Gate { command: name },
        )?;
        registry.add_static_domain(
            gateway_tasks::DOMAIN_NAME,
            gateway_tasks::DOMAIN_DESCRIPTION,
            gateway_tasks::commands(),
            |name| CommandTarget::Tasks { command: name },
        )?;
        for (server_name, mut tools) in gateway_sources {
            if server_name.trim().is_empty() {
                return Err(MaenifoldError::Message(
                    "Gateway server name cannot be empty".to_string(),
                ));
            }
            tools.sort_by(|left, right| left.name.to_string().cmp(&right.name.to_string()));
            let description = upstream_descriptions
                .get(&server_name)
                .filter(|value| !value.trim().is_empty())
                .cloned()
                .unwrap_or_else(|| {
                    format!(
                        "Gateway-backed MCP domain '{server_name}'. Call GatewayIndex with this domain to inspect live commands and schemas before GatewayExecute."
                    )
                });
            registry.hidden_tools.insert(
                super::describe::legacy_facade_name(&server_name),
                server_name.clone(),
            );
            let mut upstream = UpstreamServer::default();
            let mut commands = Vec::new();
            for tool in tools {
                let name = tool.name.to_string();
                let schema = Value::Object((*tool.input_schema).clone());
                upstream
                    .validators
                    .insert(name.clone(), compile_validator(&schema));
                registry
                    .hidden_tools
                    .entry(name.clone())
                    .or_insert_with(|| server_name.clone());
                commands.push((
                    name.clone(),
                    RegisteredCommand {
                        entry: CommandEntry {
                            name: name.clone(),
                            description: tool
                                .description
                                .as_deref()
                                .unwrap_or("No description available.")
                                .to_string(),
                            schema,
                            safety_class: domain_mapping::tool_safety(&name).to_string(),
                        },
                        target: CommandTarget::Upstream {
                            server_name: server_name.clone(),
                            tool_name: name,
                        },
                    },
                ));
            }
            let domain = registry.domain_mut(&server_name, &description);
            for (name, command) in commands {
                if domain.commands.contains_key(&name) {
                    return Err(MaenifoldError::Message(format!(
                        "Duplicate gateway command '{name}' in domain '{server_name}'"
                    )));
                }
                domain.commands.insert(name, command);
            }
            registry.upstream.insert(server_name, upstream);
        }
        Ok(registry)
    }

    fn add_native_tools(&mut self) -> Result<()> {
        for tool in list_mcp_tools(ToolRegistry::global())? {
            let Some(domain) = domain_mapping::tool_domain(&tool.name) else {
                continue;
            };
            self.domain_mut(domain, native_domain_description(domain))
                .commands
                .insert(
                    tool.name.clone(),
                    RegisteredCommand {
                        entry: CommandEntry {
                            name: tool.name.clone(),
                            description: tool.description,
                            schema: tool.input_schema,
                            safety_class: domain_mapping::tool_safety(&tool.name).to_string(),
                        },
                        target: CommandTarget::Native {
                            tool_name: tool.name,
                        },
                    },
                );
        }
        Ok(())
    }

    fn add_static_domain<F>(
        &mut self,
        name: &str,
        description: &str,
        commands: Vec<CommandEntry>,
        target: F,
    ) -> Result<()>
    where
        F: Fn(String) -> CommandTarget,
    {
        let domain = self.domain_mut(name, description);
        for entry in commands {
            let command_name = entry.name.clone();
            if domain.commands.contains_key(&command_name) {
                return Err(MaenifoldError::Message(format!(
                    "Duplicate gateway command '{command_name}' in domain '{name}'"
                )));
            }
            domain.commands.insert(
                command_name.clone(),
                RegisteredCommand {
                    entry,
                    target: target(command_name),
                },
            );
        }
        Ok(())
    }

    fn domain_mut(&mut self, name: &str, description: &str) -> &mut RegisteredDomain {
        self.domains
            .entry(name.to_string())
            .or_insert_with(|| RegisteredDomain {
                description: description.to_string(),
                commands: BTreeMap::new(),
            })
    }
}

fn native_domain_description(domain: &str) -> &'static str {
    match domain {
        "maenifold.memory" => "Maenifold memory file read/write commands.",
        "maenifold.graph" => "Maenifold graph traversal and repair commands.",
        "maenifold.workflow" => "Maenifold workflow and pipeline orchestration commands.",
        "maenifold.thinking" => "Maenifold thinking and assumption tracking commands.",
        "maenifold.system" => "Maenifold system, asset, ACP, and diagnostics commands.",
        _ => "Native Maenifold commands backed by typed handlers.",
    }
}
