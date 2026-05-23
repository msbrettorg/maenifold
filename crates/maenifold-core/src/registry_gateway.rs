use crate::error::{MaenifoldError, Result};
use crate::mcp;
use crate::registry::ToolDescriptor;
use crate::tools::ToolContext;
use serde_json::Value;

pub(crate) fn descriptors() -> &'static [ToolDescriptor] {
    DESCRIPTORS
}

const DESCRIPTORS: &[ToolDescriptor] = &[
    ToolDescriptor {
        name: "GatewayIndex",
        aliases: &["gatewayindex"],
        invoke: gateway_index,
    },
    ToolDescriptor {
        name: "GatewayExecute",
        aliases: &["gatewayexecute"],
        invoke: gateway_execute,
    },
    ToolDescriptor {
        name: "GatewayBatch",
        aliases: &["gatewaybatch"],
        invoke: gateway_batch,
    },
    ToolDescriptor {
        name: "GatewayIntent",
        aliases: &["gatewayintent"],
        invoke: gateway_intent,
    },
    ToolDescriptor {
        name: "gate_Create",
        aliases: &["gate_create"],
        invoke: gate_create,
    },
    ToolDescriptor {
        name: "gate_Status",
        aliases: &["gate_status"],
        invoke: gate_status,
    },
    ToolDescriptor {
        name: "gate_Sync",
        aliases: &["gate_sync"],
        invoke: gate_sync,
    },
    ToolDescriptor {
        name: "gate_Append",
        aliases: &["gate_append"],
        invoke: gate_append,
    },
    ToolDescriptor {
        name: "gate_Pass",
        aliases: &["gate_pass"],
        invoke: gate_pass,
    },
    ToolDescriptor {
        name: "gate_Fail",
        aliases: &["gate_fail"],
        invoke: gate_fail,
    },
    ToolDescriptor {
        name: "gate_Reopen",
        aliases: &["gate_reopen"],
        invoke: gate_reopen,
    },
    ToolDescriptor {
        name: "gate_History",
        aliases: &["gate_history"],
        invoke: gate_history,
    },
    ToolDescriptor {
        name: "gate_TaskBody",
        aliases: &["gate_taskbody", "gate_task_body"],
        invoke: gate_task_body,
    },
];

macro_rules! gateway_tool {
    ($fn_name:ident, $tool_name:literal) => {
        fn $fn_name(context: &ToolContext, payload: &Value) -> Result<String> {
            invoke(context, $tool_name, payload)
        }
    };
}

gateway_tool!(gateway_index, "GatewayIndex");
gateway_tool!(gateway_execute, "GatewayExecute");
gateway_tool!(gateway_batch, "GatewayBatch");
gateway_tool!(gateway_intent, "GatewayIntent");
gateway_tool!(gate_create, "gate_Create");
gateway_tool!(gate_status, "gate_Status");
gateway_tool!(gate_sync, "gate_Sync");
gateway_tool!(gate_append, "gate_Append");
gateway_tool!(gate_pass, "gate_Pass");
gateway_tool!(gate_fail, "gate_Fail");
gateway_tool!(gate_reopen, "gate_Reopen");
gateway_tool!(gate_history, "gate_History");
gateway_tool!(gate_task_body, "gate_TaskBody");

fn invoke(context: &ToolContext, tool_name: &str, payload: &Value) -> Result<String> {
    if payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Ok(mcp::gateway_cli_help(tool_name));
    }

    let future = mcp::invoke_gateway_cli_tool(
        &context.config,
        dispatch_tool_name(tool_name),
        dispatch_payload(tool_name, payload),
    );

    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            if matches!(
                handle.runtime_flavor(),
                tokio::runtime::RuntimeFlavor::MultiThread
            ) {
                tokio::task::block_in_place(|| handle.block_on(future))
            } else {
                std::thread::scope(|scope| {
                    scope
                        .spawn(|| {
                            tokio::runtime::Builder::new_current_thread()
                                .enable_all()
                                .build()
                                .map_err(|error| {
                                    MaenifoldError::Message(format!(
                                        "Failed to build runtime for gateway CLI tool '{tool_name}': {error}"
                                    ))
                                })?
                                .block_on(future)
                        })
                        .join()
                        .map_err(|_| {
                            MaenifoldError::Message(format!(
                                "Gateway tool '{tool_name}' thread panicked"
                            ))
                        })?
                })
            }
        }
        Err(_) => tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| {
                MaenifoldError::Message(format!(
                    "Failed to build runtime for gateway CLI tool '{tool_name}': {error}"
                ))
            })?
            .block_on(future),
    }
}

fn dispatch_tool_name(tool_name: &str) -> &str {
    if tool_name.starts_with("gate_") {
        "GatewayExecute"
    } else {
        tool_name
    }
}

fn dispatch_payload(tool_name: &str, payload: &Value) -> Value {
    if tool_name.starts_with("gate_") {
        serde_json::json!({
            "domain": "maenifold.gate",
            "command": tool_name,
            "parameters": payload,
        })
    } else {
        payload.clone()
    }
}
