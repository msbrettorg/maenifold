use rmcp::model::Tool;
use serde_json::{Map, Value, json};
use std::{process::Command, sync::Arc};

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
                    "pipeline": string_prop("Optional path to the pipeline.json definition file."),
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
                    "pipeline": string_prop("Optional path to the pipeline.json definition file."),
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
                    "pipeline": string_prop("Optional path to the pipeline.json definition file."),
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
                    "pipeline": string_prop("Optional path to the pipeline.json definition file."),
                    "root": string_prop("Project root directory containing the todo/ and done/ task folders."),
                    "rebuild": bool_prop("If true, delete and rebuild the gate cache from scratch.")
                },
                "required": ["root"]
            }),
        ),
    ]
}

pub fn dispatch(name: &str, mut args: Map<String, Value>) -> anyhow::Result<String> {
    let exe = std::env::current_exe()?;
    let mut command = Command::new(exe);

    match name {
        "gate_pass" => {
            command.arg("pass");
            push_opt(&mut command, "--db", take_string(&mut args, "db")?);
            push_opt(&mut command, "--root", take_string(&mut args, "root")?);
            push_opt(
                &mut command,
                "--pipeline",
                take_string(&mut args, "pipeline")?,
            );
            push_opt(&mut command, "--notes", take_string(&mut args, "notes")?);
            command.arg(required_string(&mut args, "id")?);
        }
        "gate_reject" => {
            command.arg("reject");
            push_opt(&mut command, "--db", take_string(&mut args, "db")?);
            push_opt(&mut command, "--root", take_string(&mut args, "root")?);
            push_opt(
                &mut command,
                "--pipeline",
                take_string(&mut args, "pipeline")?,
            );
            push_opt(&mut command, "--notes", take_string(&mut args, "notes")?);
            command.arg(required_string(&mut args, "id")?);
        }
        "gate_append" => {
            command.arg("append");
            push_opt(&mut command, "--db", take_string(&mut args, "db")?);
            push_opt(&mut command, "--root", take_string(&mut args, "root")?);
            command
                .arg("--notes")
                .arg(required_string(&mut args, "notes")?);
            command
                .arg("--agent")
                .arg(required_string(&mut args, "agent")?);
            command.arg(required_string(&mut args, "id")?);
        }
        "gate_status" => {
            command.arg("status");
            push_opt(&mut command, "--db", take_string(&mut args, "db")?);
            push_opt(&mut command, "--root", take_string(&mut args, "root")?);
            push_opt(
                &mut command,
                "--pipeline",
                take_string(&mut args, "pipeline")?,
            );
            command.arg("--json");
            if let Some(id) = take_string(&mut args, "id")? {
                command.arg(id);
            }
        }
        "gate_sync" => {
            command.arg("sync");
            push_opt(&mut command, "--db", take_string(&mut args, "db")?);
            push_opt(
                &mut command,
                "--pipeline",
                take_string(&mut args, "pipeline")?,
            );
            if take_bool(&mut args, "rebuild")?.unwrap_or(false) {
                command.arg("--rebuild");
            }
            command.arg(required_string(&mut args, "root")?);
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
    let text = match (stdout.is_empty(), stderr.is_empty()) {
        (false, true) => stdout,
        (true, false) => stderr,
        (false, false) => format!("{stdout}\n{stderr}"),
        (true, true) => String::new(),
    };

    if output.status.success() {
        Ok(text)
    } else if text.is_empty() {
        anyhow::bail!("process exited with status {}", output.status);
    } else {
        anyhow::bail!("{text}");
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
