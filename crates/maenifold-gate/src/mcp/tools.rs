use crate::registry;
use rmcp::model::Tool;
use serde_json::{Map, Value, json};
use std::{process::Command, sync::Arc};

#[derive(Debug)]
pub struct DispatchResult {
    pub output: String,
}

pub fn build_tool_list() -> Vec<Tool> {
    vec![
        tool(
            "gate_pass",
            "Advance a gate task through the pipeline pass transition.",
            json!({
                "type": "object",
                "properties": {
                    "id": string_prop("Task ID to advance via the pass transition (for example, T-68.1)."),
                    "db": string_prop("Path to the gate.db cache file."),
                    "root": string_prop("Project root directory containing the todo/ and done/ task folders."),
                    "notes": string_prop("Optional notes to attach to the transition.")
                },
                "required": ["id"]
            }),
        ),
        tool(
            "gate_reject",
            "Reject a gate task through the pipeline reject transition.",
            json!({
                "type": "object",
                "properties": {
                    "id": string_prop("Task ID to reject via the reject transition (for example, T-68.1)."),
                    "db": string_prop("Path to the gate.db cache file."),
                    "root": string_prop("Project root directory containing the todo/ and done/ task folders."),
                    "notes": string_prop("Optional notes to attach to the transition.")
                },
                "required": ["id"]
            }),
        ),
        tool(
            "gate_append",
            "Append agent notes to a gate task file.",
            json!({
                "type": "object",
                "properties": {
                    "id": string_prop("Task ID whose notes section should be updated (for example, T-68.1)."),
                    "db": string_prop("Path to the gate.db cache file."),
                    "root": string_prop("Project root directory containing the todo/ and done/ task folders."),
                    "notes": string_prop("Notes content to append to the task file."),
                    "agent": string_prop("Agent role name to label the note entry (for example, swe or red-team).")
                },
                "required": ["id", "notes", "agent"]
            }),
        ),
        tool(
            "gate_status",
            "Inspect gate task state and return JSON output.",
            json!({
                "type": "object",
                "properties": {
                    "db": string_prop("Path to the gate.db cache file."),
                    "root": string_prop("Project root directory containing the todo/ and done/ task folders."),
                    "id": string_prop("Optional task ID to inspect; omit to list all tracked tasks.")
                }
            }),
        ),
        tool(
            "gate_sync",
            "Rebuild or reconcile the gate cache from task files.",
            json!({
                "type": "object",
                "properties": {
                    "db": string_prop("Path to the gate.db cache file."),
                    "root": string_prop("Project root directory containing the todo/ and done/ task folders."),
                    "rebuild": bool_prop("If true, delete and rebuild the gate cache from scratch.")
                },
                "required": ["root"]
            }),
        ),
        tool(
            "gate_list_gates",
            "Return the system-wide gate registry as JSON.",
            json!({"type": "object", "properties": {}}),
        ),
    ]
}

pub fn dispatch(name: &str, args: &mut Map<String, Value>) -> Result<DispatchResult, String> {
    dispatch_impl(name, args).map_err(|err| err.to_string())
}

fn dispatch_impl(name: &str, args: &mut Map<String, Value>) -> anyhow::Result<DispatchResult> {
    prepare_args(name, args)?;

    if name == "gate_list_gates" {
        if !args.is_empty() {
            let extras = args.keys().cloned().collect::<Vec<_>>().join(", ");
            anyhow::bail!("unknown arguments: {extras}");
        }
        return Ok(DispatchResult {
            output: serde_json::to_string(&registry::load()?)?,
        });
    }

    let exe = std::env::current_exe()?;
    let mut command = Command::new(exe);

    match name {
        "gate_pass" => {
            command.arg("pass");
            push_opt(&mut command, "--db", take_string(args, "db")?);
            push_opt(&mut command, "--root", take_string(args, "root")?);
            push_opt(&mut command, "--notes", take_string(args, "notes")?);
            command.arg(required_string(args, "id")?);
        }
        "gate_reject" => {
            command.arg("reject");
            push_opt(&mut command, "--db", take_string(args, "db")?);
            push_opt(&mut command, "--root", take_string(args, "root")?);
            push_opt(&mut command, "--notes", take_string(args, "notes")?);
            command.arg(required_string(args, "id")?);
        }
        "gate_append" => {
            command.arg("append");
            push_opt(&mut command, "--db", take_string(args, "db")?);
            push_opt(&mut command, "--root", take_string(args, "root")?);
            command.arg("--notes").arg(required_string(args, "notes")?);
            command.arg("--agent").arg(required_string(args, "agent")?);
            command.arg(required_string(args, "id")?);
        }
        "gate_status" => {
            command.arg("status");
            push_opt(&mut command, "--db", take_string(args, "db")?);
            push_opt(&mut command, "--root", take_string(args, "root")?);
            command.arg("--json");
            if let Some(id) = take_string(args, "id")? {
                command.arg(id);
            }
        }
        "gate_sync" => {
            command.arg("sync");
            push_opt(&mut command, "--db", take_string(args, "db")?);
            if take_bool(args, "rebuild")?.unwrap_or(false) {
                command.arg("--rebuild");
            }
            command.arg(required_string(args, "root")?);
        }
        other => anyhow::bail!("unknown tool: {other}"),
    }

    if !args.is_empty() {
        let extras = args.keys().cloned().collect::<Vec<_>>().join(", ");
        anyhow::bail!("unknown arguments: {extras}");
    }

    let output = command.output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let text = if !stdout.is_empty() {
        if !stderr.is_empty() {
            eprintln!("[gate] subprocess stderr: {stderr}");
        }
        stdout
    } else {
        stderr
    };

    if output.status.success() {
        Ok(DispatchResult { output: text })
    } else if text.is_empty() {
        anyhow::bail!("process exited with status {}", output.status);
    } else {
        anyhow::bail!("{text}");
    }
}

fn prepare_args(name: &str, args: &mut Map<String, Value>) -> anyhow::Result<()> {
    match name {
        "gate_sync" => {
            let root = required_string(args, "root")?;
            args.insert("root".into(), Value::String(root.clone()));
            Ok(())
        }
        "gate_pass" | "gate_reject" | "gate_append" | "gate_status" | "gate_list_gates" => Ok(()),
        _ => Ok(()),
    }
}

fn tool(name: &str, description: &str, schema: Value) -> Tool {
    let map = match schema {
        Value::Object(map) => map,
        _ => Map::new(),
    };
    Tool::new(name.to_string(), description.to_string(), Arc::new(map))
}

fn string_prop(description: &str) -> Value {
    json!({
        "type": "string",
        "description": description
    })
}

fn bool_prop(description: &str) -> Value {
    json!({
        "type": "boolean",
        "description": description
    })
}

fn required_string(args: &mut Map<String, Value>, key: &str) -> anyhow::Result<String> {
    take_string(args, key)?.ok_or_else(|| anyhow::anyhow!("missing required argument '{key}'"))
}

fn take_string(args: &mut Map<String, Value>, key: &str) -> anyhow::Result<Option<String>> {
    match args.remove(key) {
        Some(Value::String(value)) => Ok(Some(value)),
        Some(_) => anyhow::bail!("argument '{key}' must be a string"),
        None => Ok(None),
    }
}

fn take_bool(args: &mut Map<String, Value>, key: &str) -> anyhow::Result<Option<bool>> {
    match args.remove(key) {
        Some(Value::Bool(value)) => Ok(Some(value)),
        Some(_) => anyhow::bail!("argument '{key}' must be a boolean"),
        None => Ok(None),
    }
}

fn push_opt(command: &mut Command, flag: &str, value: Option<String>) {
    if let Some(value) = value {
        command.arg(flag).arg(value);
    }
}
