use super::DomainEntry;
use rmcp::model::ServerInfo;

const DOMAIN_DESCRIPTION_CAP_DESCRIPTION: usize = 280;
const DOMAIN_DESCRIPTION_CAP_INSTRUCTIONS: usize = 600;

pub(crate) fn server_description(domains: &[DomainEntry]) -> String {
    if domains.is_empty() {
        return "Maenifold MCP gateway (no live domains advertised yet). Call GatewayIndex once the gateway pool finishes initializing.".to_string();
    }
    format!(
        "Maenifold MCP gateway. Live gateway domains and the description each upstream advertised at initialize:\n\n{}\n\nStart with GatewayIndex for command lists and schemas, then call GatewayExecute with the selected domain, command, and parameters.",
        render_domain_bullets(domains, DOMAIN_DESCRIPTION_CAP_DESCRIPTION)
    )
}

pub(crate) fn server_instructions(domains: &[DomainEntry]) -> String {
    if domains.is_empty() {
        return "Cold-start flow: the gateway pool has not finished initializing yet. Call GatewayIndex once initialization completes to list live gateway domains and commands.".to_string();
    }
    format!(
        "Cold-start flow: the gateway exposes the following live domains, each shown with its name, command count, and initialize description:\n\n{}\n\nUse GatewayIndex with includeSchemas=true on the selected domain for exact parameters, then call GatewayExecute with the domain, command, and parameters. Use ReadMcpResource only for concrete resource URIs; use GetHelp for detailed help on a named public tool or gateway command.",
        render_domain_bullets(domains, DOMAIN_DESCRIPTION_CAP_INSTRUCTIONS)
    )
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

pub(super) fn legacy_facade_name(server_name: &str) -> String {
    let sanitized: String = server_name
        .chars()
        .flat_map(|ch| {
            if ch.is_alphanumeric() {
                ch.to_lowercase().collect::<Vec<_>>()
            } else {
                vec!['_']
            }
        })
        .take(60)
        .collect();
    format!("gateway_{sanitized}")
}

fn render_domain_bullets(domains: &[DomainEntry], max_chars: usize) -> String {
    domains
        .iter()
        .map(|domain| {
            let description = collapse_description(&domain.description);
            format!(
                "- {} ({} commands): {}",
                domain.name,
                domain.command_count,
                truncate_chars(&description, max_chars)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

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
