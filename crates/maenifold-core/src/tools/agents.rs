use super::context::{ToolContext, render};
use agent_client_protocol as acp;
use crate::acp_config::AgentsConfig;
use crate::acp_native::{GLOBAL_AGENT_POOL, SessionOpts};
use crate::error::Result;
use crate::workflow::pipeline::SUPERVISOR_RUNTIME;
use serde_json::{Map, Value, json};

pub fn list_agents(context: &ToolContext, payload: &Value) -> Result<String> {
    let probe = payload
        .get("probe")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let agents = list_agents_value(&context.config.maenifold_root, probe)?;
    Ok(render(context.output_format, agents))
}

fn list_agents_value(root: &std::path::Path, probe: bool) -> Result<Value> {
    let config = AgentsConfig::load(root);
    let mut names = config.agents.keys().cloned().collect::<Vec<_>>();
    names.sort();

    let mut agents = Vec::with_capacity(names.len());
    for name in &names {
        let entry = config
            .agents
            .get(name)
            .expect("sorted key should resolve to an agent entry");
        let mut value = list_agent_entry(name, entry)?;
        if probe {
            let caps = probe_agent(root, name, entry);
            if let Value::Object(ref mut obj) = value {
                obj.insert("capabilities".to_string(), caps);
            }
        }
        agents.push(value);
    }

    Ok(Value::Array(agents))
}

fn list_agent_entry(name: &str, entry: &crate::acp_config::AgentEntry) -> Result<Value> {
    let mut object = match serde_json::to_value(entry)? {
        Value::Object(object) => object,
        _ => Map::new(),
    };
    let (command, args) = entry.command_parts();

    object.insert("name".to_string(), json!(name));
    object.insert("command".to_string(), json!(command));
    object.insert(
        "args".to_string(),
        Value::Array(args.into_iter().map(|arg| json!(arg)).collect()),
    );

    Ok(Value::Object(object))
}

fn serialize_category(category: &Option<acp::SessionConfigOptionCategory>) -> Value {
    match category {
        Some(acp::SessionConfigOptionCategory::Model) => json!("model"),
        Some(acp::SessionConfigOptionCategory::Mode) => json!("mode"),
        Some(acp::SessionConfigOptionCategory::ThoughtLevel) => json!("thought_level"),
        Some(acp::SessionConfigOptionCategory::Other(value)) => json!(value),
        Some(_) | None => Value::Null,
    }
}

fn serialize_agent_info(agent_info: &Option<acp::Implementation>) -> Value {
    match agent_info {
        Some(info) => json!({
            "name": info.name,
            "version": info.version,
            "title": info.title,
        }),
        None => Value::Null,
    }
}

fn serialize_agent_capabilities(capabilities: &Option<acp::AgentCapabilities>) -> Value {
    match capabilities {
        Some(capabilities) => json!({
            "promptCapabilities": {
                "image": capabilities.prompt_capabilities.image,
                "audio": capabilities.prompt_capabilities.audio,
                "embeddedContext": capabilities.prompt_capabilities.embedded_context,
            },
            "sessionCapabilities": {
                "list": capabilities.session_capabilities.list.is_some(),
                "loadSession": capabilities.load_session,
            },
        }),
        None => Value::Null,
    }
}

/// Spawn an agent, initialize, create a session, and capture advertised
/// capabilities (modes, models, configOptions).
fn probe_agent(
    root: &std::path::Path,
    name: &str,
    entry: &crate::acp_config::AgentEntry,
) -> Value {
    let command = entry.command.clone();
    let env = entry.env.clone();
    let cwd = root.to_path_buf();
    let agent_name = name.to_string();

    let result: std::result::Result<Value, String> = SUPERVISOR_RUNTIME.block_on(async move {
        let handle = GLOBAL_AGENT_POOL
            .get_or_create(&agent_name, &command, &cwd, env)
            .await;

        let info = handle
            .new_session(SessionOpts {
                cwd: cwd.clone(),
                model: None,
                mode: None,
                thought_level: None,
                config_options: None,
                allowed_tools: None,
                max_turns: None,
                timeout_seconds: Some(30),
                mcp_servers: vec![],
            })
            .await
            .map_err(|e| e.to_string())?;

        let session_id = info.session_id.clone();
        let has_config_options = !info.config_options.is_empty();
        let agent_info = handle.agent_info();
        let agent_capabilities = handle.capabilities();

        let capabilities = json!({
            "style": if has_config_options { "modern" } else { "legacy" },
            "agentInfo": serialize_agent_info(&agent_info),
            "agentCapabilities": serialize_agent_capabilities(&agent_capabilities),
            "modes": info.modes.iter().map(|mode| {
                json!({
                    "id": mode.id.to_string(),
                    "name": mode.name,
                    "description": mode.description,
                })
            }).collect::<Vec<_>>(),
            "models": info.models.iter().map(|model| {
                json!({
                    "modelId": model.model_id.to_string(),
                    "name": model.name,
                    "description": model.description,
                })
            }).collect::<Vec<_>>(),
            "configOptions": info.config_options.iter().map(|opt| {
                json!({
                    "id": opt.id.to_string(),
                    "name": opt.name,
                    "description": opt.description,
                    "category": serialize_category(&opt.category),
                })
            }).collect::<Vec<_>>(),
        });

        handle.close_session(&session_id).await;
        Ok(capabilities)
    });

    match result {
        Ok(value) => value,
        Err(error) => json!({ "error": error }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::mcp::schema::mcp_tool_from_registry_name;
    use crate::registry::ToolRegistry;
    use crate::util::json::OutputFormat;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    fn write_executable(root: &Path, file_name: &str) -> PathBuf {
        let path = root.join(file_name);
        fs::write(&path, "#!/bin/sh\n").expect("write dummy");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod");
        }
        path
    }

    fn test_context(root: &Path, output_format: OutputFormat) -> ToolContext {
        let config = Config {
            maenifold_root: root.to_path_buf(),
            memory_path: root.join("memory"),
            assets_path: root.join("assets"),
            thinking_path: root.join("memory").join("thinking"),
            sequential_path: root.join("memory").join("thinking").join("sequential"),
            database_path: root.join("memory.db"),
        };
        ToolContext::new(config, output_format)
    }

    #[test]
    fn list_agents_handler_returns_json_array() {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path();
        let command = write_executable(root, "test-agent");
        fs::write(
            root.join("agents.json"),
            serde_json::to_string_pretty(&json!({
                "agents": {
                    "TestAgent": {
                        "command": format!("{} serve --acp", command.display())
                    }
                }
            }))
            .expect("serialize agents config"),
        )
        .expect("write agents config");

        let output = list_agents(&test_context(root, OutputFormat::Json), &json!({}))
            .expect("list agents should succeed");
        let value: Value = serde_json::from_str(&output).expect("output should be JSON");

        assert!(value.is_array(), "expected JSON array, got {value}");
    }

    #[test]
    fn list_agents_handler_includes_expected_fields() {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path();
        let command = write_executable(root, "test-agent");
        fs::write(
            root.join("agents.json"),
            serde_json::to_string_pretty(&json!({
                "agents": {
                    "TestAgent": {
                        "command": format!("{} serve --acp", command.display()),
                        "mcp_servers": [
                            {
                                "type": "http",
                                "name": "memory",
                                "url": "http://localhost:8080/mcp"
                            }
                        ]
                    }
                }
            }))
            .expect("serialize agents config"),
        )
        .expect("write agents config");

        let output = list_agents(&test_context(root, OutputFormat::Json), &json!({}))
            .expect("list agents should succeed");
        let agents: Value = serde_json::from_str(&output).expect("output should be JSON");
        let agent = &agents.as_array().expect("array output")[0];

        assert_eq!(agent.get("name"), Some(&json!("TestAgent")));
        assert_eq!(
            agent.get("command"),
            Some(&json!(command.display().to_string()))
        );
        assert_eq!(agent.get("args"), Some(&json!(["serve", "--acp"])));
        assert_eq!(
            agent
                .get("mcp_servers")
                .and_then(Value::as_array)
                .map(Vec::len),
            Some(1)
        );
    }

    #[test]
    fn list_agents_schema_and_registry_are_registered() {
        let registry = ToolRegistry::new();
        assert!(registry.contains("ListAgents"));
        assert!(registry.contains("listagents"));
        assert!(registry.contains("list_agents"));

        let tool = mcp_tool_from_registry_name("ListAgents").expect("schema should exist");
        assert_eq!(tool.name, "ListAgents");
        assert_eq!(tool.input_schema.get("type"), Some(&json!("object")));
        assert!(
            tool.input_schema.get("required").is_none(),
            "ListAgents should not require parameters"
        );
    }
}
