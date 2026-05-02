use rmcp::model::ServerInfo;
use std::collections::BTreeMap;

pub(crate) fn native_domain_description(domain: &str) -> &'static str {
    match domain {
        "maenifold.memory" => "Maenifold memory file read/write commands.",
        "maenifold.graph" => "Maenifold graph traversal and repair commands.",
        "maenifold.workflow" => "Maenifold workflow and pipeline orchestration commands.",
        "maenifold.thinking" => "Maenifold thinking and assumption tracking commands.",
        "maenifold.system" => "Maenifold system, asset, ACP, and diagnostics commands.",
        _ => "Native Maenifold commands backed by typed handlers.",
    }
}

pub(crate) fn upstream_domain_description(
    server_name: &str,
    upstream_descriptions: &BTreeMap<String, String>,
) -> String {
    upstream_descriptions
        .get(server_name)
        .filter(|description| !description.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| {
            format!(
                "Gateway-backed MCP domain '{server_name}'. The upstream server did not provide initialize description or instructions; call GatewayIndex with this domain to inspect live commands and schemas before GatewayExecute."
            )
        })
}

pub(crate) fn upstream_initialize_description(info: &ServerInfo) -> Option<String> {
    let description = trimmed(info.server_info.description.as_deref());
    let instructions = trimmed(info.instructions.as_deref());
    match (description, instructions) {
        (Some(description), Some(instructions)) if description != instructions => {
            Some(format!("{description}\n\nInstructions: {instructions}"))
        }
        (Some(description), _) => Some(description.to_string()),
        (None, Some(instructions)) => Some(instructions.to_string()),
        (None, None) => None,
    }
}

pub(crate) fn server_description(domain_names: &[String]) -> String {
    format!(
        "Maenifold MCP gateway exposing live domains: {}. Start with GatewayIndex for domain and command discovery before GatewayExecute.",
        rendered_domains(domain_names)
    )
}

pub(crate) fn server_instructions(domain_names: &[String]) -> String {
    format!(
        "Cold-start flow: call GatewayIndex first to list live gateway domains and commands. Live domains currently visible: {}. Use GatewayIndex with includeSchemas=true for exact parameters, then call GatewayExecute with the selected domain, command, and parameters. Use DiscoverMcpTools only for per-server downstream MCP tool inventory; use ReadMcpResource only for concrete resource URIs; use GetHelp for detailed help on a named public tool or gateway command.",
        rendered_domains(domain_names)
    )
}

fn rendered_domains(domain_names: &[String]) -> String {
    if domain_names.is_empty() {
        "none advertised yet".to_string()
    } else {
        domain_names.join(", ")
    }
}

fn trimmed(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}
