use super::ToolDescriptor;
use crate::mcp::config_tools;

pub(super) const DESCRIPTORS: &[ToolDescriptor] = &[
    ToolDescriptor {
        name: "ReloadConfig",
        aliases: &["reloadconfig", "reload_config"],
        invoke: config_tools::registry_invocation_unavailable,
    },
    ToolDescriptor {
        name: "UpdateSettings",
        aliases: &["updatesettings", "update_settings"],
        invoke: config_tools::registry_invocation_unavailable,
    },
    ToolDescriptor {
        name: "AddAgent",
        aliases: &["addagent", "add_agent"],
        invoke: config_tools::registry_invocation_unavailable,
    },
    ToolDescriptor {
        name: "UpdateAgent",
        aliases: &["updateagent", "update_agent"],
        invoke: config_tools::registry_invocation_unavailable,
    },
    ToolDescriptor {
        name: "DeleteAgent",
        aliases: &["deleteagent", "delete_agent"],
        invoke: config_tools::registry_invocation_unavailable,
    },
    ToolDescriptor {
        name: "AddMcpServer",
        aliases: &["addmcpserver", "add_mcp_server"],
        invoke: config_tools::registry_invocation_unavailable,
    },
    ToolDescriptor {
        name: "UpdateMcpServer",
        aliases: &["updatemcpserver", "update_mcp_server"],
        invoke: config_tools::registry_invocation_unavailable,
    },
    ToolDescriptor {
        name: "DeleteMcpServer",
        aliases: &["deletemcpserver", "delete_mcp_server"],
        invoke: config_tools::registry_invocation_unavailable,
    },
];
