use super::context::{ToolContext, render};
use crate::acp_config::AgentsConfig;
use crate::error::Result;
use serde_json::{Map, Value, json};

pub fn list_agents(context: &ToolContext, _payload: &Value) -> Result<String> {
    let agents = list_agents_value(&context.config.maenifold_root)?;
    Ok(render(context.output_format, agents))
}

fn list_agents_value(root: &std::path::Path) -> Result<Value> {
    let config = AgentsConfig::load(root);
    let mut names = config.agents.keys().cloned().collect::<Vec<_>>();
    names.sort();

    let mut agents = Vec::with_capacity(names.len());
    for name in names {
        let entry = config
            .agents
            .get(&name)
            .expect("sorted key should resolve to an agent entry");
        agents.push(list_agent_entry(&name, entry)?);
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
        assert_eq!(agent.get("command"), Some(&json!(command.display().to_string())));
        assert_eq!(agent.get("args"), Some(&json!(["serve", "--acp"])));
        assert_eq!(
            agent.get("mcp_servers").and_then(Value::as_array).map(Vec::len),
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
