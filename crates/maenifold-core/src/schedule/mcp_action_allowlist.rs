//! Schedule MCP action allowlist — defense against P0 trigger #6 (hidden inference).
use std::collections::HashSet;

pub fn default_allowlist() -> HashSet<(String, String)> {
    [
        ("maenifold.gate", "gate_Status"),
        ("maenifold.gate", "gate_Sync"),
        ("maenifold.memory", "ReadMemory"),
        ("maenifold.memory", "ListMemories"),
        ("maenifold.memory", "MemoryStatus"),
        ("maenifold.graph", "BuildContext"),
        ("maenifold.graph", "RecentActivity"),
        ("maenifold.system", "Schedule"),
    ]
    .iter()
    .map(|(d, c)| (d.to_string(), c.to_string()))
    .collect()
}

pub fn is_allowed(allowlist: &HashSet<(String, String)>, domain: &str, command: &str) -> bool {
    allowlist.contains(&(domain.to_string(), command.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{default_allowlist, is_allowed};

    #[test]
    fn mcp_action_allowlist_includes_safe_actions() {
        let allowlist = default_allowlist();
        for (domain, command) in [
            ("maenifold.gate", "gate_Status"),
            ("maenifold.gate", "gate_Sync"),
            ("maenifold.memory", "ReadMemory"),
            ("maenifold.memory", "ListMemories"),
            ("maenifold.memory", "MemoryStatus"),
            ("maenifold.graph", "BuildContext"),
            ("maenifold.graph", "RecentActivity"),
            ("maenifold.system", "Schedule"),
        ] {
            assert!(
                is_allowed(&allowlist, domain, command),
                "{domain}.{command}"
            );
        }
    }

    #[test]
    fn mcp_action_allowlist_excludes_inference() {
        let allowlist = default_allowlist();
        for (domain, command) in [
            ("maenifold.acp", "AcpExec"),
            ("maenifold.system", "GatewayIntent"),
            ("maenifold.workflow", "Workflow"),
            ("maenifold.pipeline", "Pipeline"),
            ("maenifold.thinking", "SequentialThinking"),
            ("maenifold.memory", "SearchMemories"),
        ] {
            assert!(
                !is_allowed(&allowlist, domain, command),
                "{domain}.{command}"
            );
        }
    }

    #[test]
    fn gate_status_is_allowed() {
        let allowlist = default_allowlist();
        assert!(is_allowed(&allowlist, "maenifold.gate", "gate_Status"));
    }

    #[test]
    fn unknown_command_is_not_allowed() {
        let allowlist = default_allowlist();
        assert!(!is_allowed(
            &allowlist,
            "maenifold.gate",
            "gate_DeleteEverything"
        ));
    }

    #[test]
    fn unknown_domain_is_not_allowed() {
        let allowlist = default_allowlist();
        assert!(!is_allowed(&allowlist, "unknown.domain", "gate_Status"));
    }
}
