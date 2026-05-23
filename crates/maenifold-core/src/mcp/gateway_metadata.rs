use crate::mcp::gateway_catalog::DomainEntry;
use rmcp::model::ServerInfo;
use std::collections::BTreeMap;

/// Maximum length of any single domain's description in
/// `serverInfo.description`. We truncate before joining so a single chatty
/// upstream (e.g. azure-mcp-server emits ~1.8KB of usage rules at initialize)
/// cannot dominate the rendered bullet list. The cap is generous enough to
/// keep a useful sentence or two — the full text is always available via
/// `GatewayIndex` / `gateway://domains/{name}` resources.
const DOMAIN_DESCRIPTION_CAP_DESCRIPTION: usize = 280;

/// Same as above, but for the `instructions` field where a longer paragraph
/// per domain is acceptable for clients that surface this string in agent
/// prompts.
const DOMAIN_DESCRIPTION_CAP_INSTRUCTIONS: usize = 600;

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

pub(crate) fn server_description(domains: &[DomainEntry]) -> String {
    if domains.is_empty() {
        return "Maenifold MCP gateway (no live domains advertised yet). \
             Call GatewayIndex once the gateway pool finishes initializing."
            .to_string();
    }
    format!(
        "Maenifold MCP gateway. Live gateway domains and the description each upstream advertised at initialize:\n\n{}\n\n\
         Start with GatewayIndex for command lists and schemas (the description above is each domain's own self-description, not its command set), then call GatewayExecute with the selected domain, command, and parameters.",
        render_domain_bullets(domains, DOMAIN_DESCRIPTION_CAP_DESCRIPTION)
    )
}

pub(crate) fn server_instructions(domains: &[DomainEntry]) -> String {
    if domains.is_empty() {
        return "Cold-start flow: the gateway pool has not finished initializing yet. \
             Call GatewayIndex once initialization completes to list live gateway domains and commands."
            .to_string();
    }
    format!(
        "Cold-start flow: the gateway exposes the following live domains, each shown with its name, command count, and the description that the upstream MCP server itself advertised at initialize:\n\n{}\n\n\
         Use GatewayIndex with includeSchemas=true on the selected domain for exact parameters, then call GatewayExecute with the domain, command, and parameters. \
         Use ReadMcpResource only for concrete resource URIs; \
         use GetHelp for detailed help on a named public tool or gateway command.",
        render_domain_bullets(domains, DOMAIN_DESCRIPTION_CAP_INSTRUCTIONS)
    )
}

fn render_domain_bullets(domains: &[DomainEntry], per_description_cap: usize) -> String {
    domains
        .iter()
        .map(|domain| {
            let header = format!("- {} ({} commands)", domain.name, domain.command_count);
            let description = collapse_description(&domain.description);
            if description.is_empty() {
                format!("{header}: <no self-description provided>")
            } else {
                format!(
                    "{header}: {}",
                    truncate_chars(&description, per_description_cap)
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Collapse a multi-line description into a single line for the bullet list.
/// Newlines inside an upstream's initialize description (common with servers
/// that bundle instructions) would otherwise break the markdown rendering.
fn collapse_description(value: &str) -> String {
    value
        .split('\n')
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut out: String = value.chars().take(max_chars.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn trimmed(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}
