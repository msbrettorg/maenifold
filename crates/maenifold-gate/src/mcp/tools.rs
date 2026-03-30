use crate::registry;
use rmcp::model::Tool;
use serde_json::{Map, Value, json};
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

pub const SESSION_INIT_ERROR: &str =
    "No gate bound to this session. Run gate_sync with a root path first to initialize.";

pub struct DispatchResult {
    pub output: String,
    pub synced_root: Option<PathBuf>,
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
        tool(
            "gate_list_gates",
            "Return the system-wide gate registry as JSON.",
            json!({"type": "object", "properties": {}}),
        ),
    ]
}

pub fn dispatch(
    name: &str,
    args: &mut Map<String, Value>,
    bound_root: Option<&Path>,
) -> Result<DispatchResult, String> {
    dispatch_impl(name, args, bound_root).map_err(|err| err.to_string())
}

fn dispatch_impl(
    name: &str,
    args: &mut Map<String, Value>,
    bound_root: Option<&Path>,
) -> anyhow::Result<DispatchResult> {
    let synced_root = prepare_args(name, args, bound_root)?;

    if name == "gate_list_gates" {
        if !args.is_empty() {
            let extras = args.keys().cloned().collect::<Vec<_>>().join(", ");
            anyhow::bail!("unknown arguments: {extras}");
        }
        return Ok(DispatchResult {
            output: serde_json::to_string(&registry::load()?)?,
            synced_root,
        });
    }

    let exe = std::env::current_exe()?;
    let mut command = Command::new(exe);

    match name {
        "gate_pass" => {
            command.arg("pass");
            push_opt(&mut command, "--db", take_string(args, "db")?);
            push_opt(&mut command, "--root", take_string(args, "root")?);
            push_opt(&mut command, "--pipeline", take_string(args, "pipeline")?);
            push_opt(&mut command, "--notes", take_string(args, "notes")?);
            command.arg(required_string(args, "id")?);
        }
        "gate_reject" => {
            command.arg("reject");
            push_opt(&mut command, "--db", take_string(args, "db")?);
            push_opt(&mut command, "--root", take_string(args, "root")?);
            push_opt(&mut command, "--pipeline", take_string(args, "pipeline")?);
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
            push_opt(&mut command, "--pipeline", take_string(args, "pipeline")?);
            command.arg("--json");
            if let Some(id) = take_string(args, "id")? {
                command.arg(id);
            }
        }
        "gate_sync" => {
            command.arg("sync");
            push_opt(&mut command, "--db", take_string(args, "db")?);
            push_opt(&mut command, "--pipeline", take_string(args, "pipeline")?);
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
    let text = match (stdout.is_empty(), stderr.is_empty()) {
        (false, true) => stdout,
        (true, false) => stderr,
        (false, false) => format!("{stdout}\n{stderr}"),
        (true, true) => String::new(),
    };

    if output.status.success() {
        Ok(DispatchResult {
            output: text,
            synced_root,
        })
    } else if text.is_empty() {
        anyhow::bail!("process exited with status {}", output.status);
    } else {
        anyhow::bail!("{text}");
    }
}

fn prepare_args(
    name: &str,
    args: &mut Map<String, Value>,
    bound_root: Option<&Path>,
) -> anyhow::Result<Option<PathBuf>> {
    match name {
        "gate_sync" => {
            let root = required_string(args, "root")?;
            args.insert("root".into(), Value::String(root.clone()));
            Ok(Some(
                std::fs::canonicalize(&root).unwrap_or_else(|_| PathBuf::from(&root)),
            ))
        }
        "gate_pass" | "gate_reject" | "gate_append" | "gate_status" => {
            inject_bound_root(args, bound_root)?;
            Ok(None)
        }
        "gate_list_gates" => Ok(None),
        _ => Ok(None),
    }
}

fn inject_bound_root(
    args: &mut Map<String, Value>,
    bound_root: Option<&Path>,
) -> anyhow::Result<()> {
    if matches!(args.get("root"), Some(Value::String(_))) {
        return Ok(());
    }
    if matches!(args.get("root"), Some(_)) {
        anyhow::bail!("argument 'root' must be a string");
    }
    let root = bound_root.ok_or_else(|| anyhow::anyhow!(SESSION_INIT_ERROR))?;
    args.insert(
        "root".into(),
        Value::String(root.to_string_lossy().into_owned()),
    );
    Ok(())
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
