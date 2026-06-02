//! Schedule MCP action allowlist — defense against P0 trigger #6 (hidden inference).
use std::collections::HashSet;

/// One allow-listed gateway action the scheduler may invoke, plus
/// human-facing metadata so UI clients can render a picker without
/// inventing their own catalog.
#[derive(Debug, Clone)]
pub struct McpActionAllowEntry {
    pub domain: &'static str,
    pub command: &'static str,
    pub label: &'static str,
    pub description: &'static str,
}

/// Canonical (domain, command, label, description) catalog. The (domain,
/// command) pair is the contract enforced by `is_allowed`; the label and
/// description are UI sugar — they MUST stay in sync with the underlying
/// gateway commands.
pub const DEFAULT_ALLOWLIST_ENTRIES: &[McpActionAllowEntry] = &[
    McpActionAllowEntry {
        domain: "maenifold.gate",
        command: "gate_Status",
        label: "Gate · Status",
        description: "Read the current Gate board state for a project root.",
    },
    McpActionAllowEntry {
        domain: "maenifold.gate",
        command: "gate_Sync",
        label: "Gate · Sync",
        description: "Rebuild the Gate read-model from the markdown task files.",
    },
    McpActionAllowEntry {
        domain: "maenifold.memory",
        command: "ReadMemory",
        label: "Memory · Read",
        description: "Read a memory note by URI.",
    },
    McpActionAllowEntry {
        domain: "maenifold.memory",
        command: "ListMemories",
        label: "Memory · List",
        description: "List memory notes under a prefix.",
    },
    McpActionAllowEntry {
        domain: "maenifold.system",
        command: "MemoryStatus",
        label: "Memory · Status",
        description: "Summarise the memory store (counts, paths).",
    },
    McpActionAllowEntry {
        domain: "maenifold.graph",
        command: "BuildContext",
        label: "Graph · BuildContext",
        description: "Materialise an LLM-shaped context bundle from the graph.",
    },
    McpActionAllowEntry {
        domain: "maenifold.system",
        command: "RecentActivity",
        label: "System · RecentActivity",
        description: "Recent agent activity rollup from the graph.",
    },
    McpActionAllowEntry {
        domain: "maenifold.system",
        command: "Schedule",
        label: "System · Schedule",
        description: "Self-call the Schedule tool (list/get/pause_status etc).",
    },
];

pub fn default_allowlist() -> HashSet<(String, String)> {
    DEFAULT_ALLOWLIST_ENTRIES
        .iter()
        .map(|entry| (entry.domain.to_string(), entry.command.to_string()))
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
            ("maenifold.system", "MemoryStatus"),
            ("maenifold.graph", "BuildContext"),
            ("maenifold.system", "RecentActivity"),
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
