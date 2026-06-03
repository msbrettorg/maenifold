use crate::error::{MaenifoldError, Result};
use serde::Serialize;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

#[path = "gateway_registry_build.rs"]
mod build_impl;
#[path = "gateway_registry_describe.rs"]
mod describe;

pub(crate) use describe::{
    server_description, server_instructions, upstream_initialize_description,
};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DomainEntry {
    pub name: String,
    pub description: String,
    pub command_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DomainCommandSummary {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DomainIndexEntry {
    pub name: String,
    pub description: String,
    pub command_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commands: Option<Vec<DomainCommandSummary>>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct CommandEntry {
    pub name: String,
    pub description: String,
    pub schema: Value,
    pub safety_class: String,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<Map<String, Value>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommandTarget {
    Native {
        tool_name: String,
    },
    ConfigCommand {
        command: String,
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

#[derive(Debug, Clone)]
pub(crate) struct RegisteredCommand {
    pub(crate) entry: CommandEntry,
    pub(crate) target: CommandTarget,
}

#[derive(Debug, Clone)]
pub(crate) struct RegisteredDomain {
    pub(crate) description: String,
    pub(crate) commands: BTreeMap<String, RegisteredCommand>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct UpstreamServer {
    pub(crate) validators: HashMap<String, Option<Arc<jsonschema::Validator>>>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct GatewayRegistry {
    pub(crate) domains: BTreeMap<String, RegisteredDomain>,
    pub(crate) upstream: HashMap<String, UpstreamServer>,
    pub(crate) hidden_tools: HashMap<String, String>,
}

impl GatewayRegistry {
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
        Ok(self
            .domain(domain)?
            .commands
            .values()
            .map(|command| command.entry.clone())
            .collect())
    }

    pub(crate) fn domain_index_entries(&self, include_commands: bool) -> Vec<DomainIndexEntry> {
        self.domains
            .iter()
            .map(|(name, domain)| DomainIndexEntry {
                name: name.clone(),
                description: domain.description.clone(),
                command_count: domain.commands.len(),
                commands: include_commands.then(|| {
                    domain
                        .commands
                        .values()
                        .map(|command| DomainCommandSummary {
                            name: command.entry.name.clone(),
                            description: command.entry.description.clone(),
                        })
                        .collect()
                }),
            })
            .collect()
    }

    pub(crate) fn render_index(
        &self,
        domain: Option<&str>,
        command: Option<&str>,
        include_commands: bool,
        include_schemas: bool,
        include_examples: bool,
    ) -> Result<Value> {
        let mut value = match (domain, command) {
            (Some(domain), Some(command)) => {
                json!({ "domain": domain, "command": self.command_record(domain, command)?.entry.clone() })
            }
            (Some(domain), None) => json!({ "domain": domain, "commands": self.commands(domain)? }),
            (None, None) => json!({ "domains": self.domain_index_entries(include_commands) }),
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

    pub(crate) fn resolve(&self, domain: &str, command: &str) -> Result<CommandTarget> {
        Ok(self.command_record(domain, command)?.target.clone())
    }

    pub(crate) fn canonical_safety_class(&self, domain: &str, command: &str) -> Result<String> {
        Ok(self
            .command_record(domain, command)?
            .entry
            .safety_class
            .clone())
    }

    pub(crate) fn required_parameters(&self, domain: &str, command: &str) -> Result<Vec<String>> {
        Ok(required_keys(
            &self.command_record(domain, command)?.entry.schema,
        ))
    }

    pub(crate) fn validate_upstream(
        &self,
        server_name: &str,
        tool_name: &str,
        parameters: &Value,
    ) -> Result<()> {
        let Some(Some(validator)) = self
            .upstream
            .get(server_name)
            .and_then(|server| server.validators.get(tool_name))
        else {
            return Ok(());
        };
        crate::mcp::gateway_schema_validation::validate_with_validator(
            tool_name, parameters, validator,
        )
        .map_err(|err| MaenifoldError::GatewayInvalidParams(err.to_string()))
    }

    pub(crate) fn hidden_tool_domain(&self, tool_name: &str) -> Option<&str> {
        self.hidden_tools.get(tool_name).map(String::as_str)
    }

    pub(crate) fn domain(&self, domain: &str) -> Result<&RegisteredDomain> {
        self.domains.get(domain).ok_or_else(|| {
            MaenifoldError::GatewayInvalidParams(format!("Unknown gateway domain '{domain}'."))
        })
    }

    pub(crate) fn command_record(&self, domain: &str, command: &str) -> Result<&RegisteredCommand> {
        let domain_record = self.domain(domain)?;
        domain_record.commands.get(command).ok_or_else(|| {
            let suggestions = self
                .domains
                .iter()
                .filter(|(name, _)| name.as_str() != domain)
                .filter(|(_, candidate)| candidate.commands.contains_key(command))
                .map(|(name, _)| format!("{name}/{command}"))
                .collect::<Vec<_>>();
            let available = domain_record
                .commands
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>();
            let mut message = format!("Unknown gateway command '{command}' for domain '{domain}'.");
            if !suggestions.is_empty() {
                message.push_str(&format!(" Did you mean: {}?", suggestions.join(", ")));
            }
            if !available.is_empty() {
                message.push_str(&format!(
                    " Available commands in '{domain}': {}.",
                    available.join(", ")
                ));
            }
            MaenifoldError::GatewayInvalidParams(message)
        })
    }
}

fn required_keys(schema: &Value) -> Vec<String> {
    schema
        .get("required")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn strip_schemas(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.remove("schema");
            for nested in map.values_mut() {
                strip_schemas(nested);
            }
        }
        Value::Array(items) => items.iter_mut().for_each(strip_schemas),
        _ => {}
    }
}
