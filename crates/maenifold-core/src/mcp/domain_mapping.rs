use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SafetyClass {
    Read,
    Write,
    Mutate,
    Destructive,
    Orchestration,
}

impl fmt::Display for SafetyClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Mutate => "mutate",
            Self::Destructive => "destructive",
            Self::Orchestration => "orchestration",
        };
        f.write_str(value)
    }
}

pub(crate) const TOOL_DOMAINS: &[(&str, &str)] = &[
    ("WriteMemory", "maenifold.memory"),
    ("ReadMemory", "maenifold.memory"),
    ("EditMemory", "maenifold.memory"),
    ("DeleteMemory", "maenifold.memory"),
    ("MoveMemory", "maenifold.memory"),
    ("SearchMemories", "maenifold.memory"),
    ("ListMemories", "maenifold.memory"),
    ("ExtractConceptsFromFile", "maenifold.memory"),
    ("BuildContext", "maenifold.graph"),
    ("Visualize", "maenifold.graph"),
    ("FindSimilarConcepts", "maenifold.graph"),
    ("Sync", "maenifold.graph"),
    ("RepairConcepts", "maenifold.graph"),
    ("AnalyzeConceptCorruption", "maenifold.graph"),
    ("Workflow", "maenifold.workflow"),
    ("Pipeline", "maenifold.workflow"),
    ("SequentialThinking", "maenifold.thinking"),
    ("AssumptionLedger", "maenifold.thinking"),
    ("MemoryStatus", "maenifold.system"),
    ("GetConfig", "maenifold.system"),
    ("UpdateAssets", "maenifold.system"),
    ("RunFullBenchmark", "maenifold.system"),
    ("Adopt", "maenifold.system"),
    ("ListAssets", "maenifold.system"),
    ("ReadMcpResource", "maenifold.system"),
    ("ListSessions", "maenifold.system"),
    ("TeamsMonitor", "maenifold.system"),
    ("AcpExec", "maenifold.system"),
    ("EmbeddingModels", "maenifold.system"),
    ("ReadTextFile", "maenifold.system"),
    ("Schedule", "maenifold.system"),
    ("RecentActivity", "maenifold.system"),
    ("ReloadConfig", "maenifold.config"),
    ("UpdateSettings", "maenifold.config"),
    ("AddAgent", "maenifold.config"),
    ("UpdateAgent", "maenifold.config"),
    ("DeleteAgent", "maenifold.config"),
    ("AddMcpServer", "maenifold.config"),
    ("UpdateMcpServer", "maenifold.config"),
    ("DeleteMcpServer", "maenifold.config"),
];

pub(crate) fn tool_domain(tool_name: &str) -> Option<&'static str> {
    TOOL_DOMAINS
        .iter()
        .find_map(|(name, domain)| (*name == tool_name).then_some(*domain))
}

pub(crate) fn tool_safety(tool_name: &str) -> SafetyClass {
    match tool_name {
        "ReadMemory"
        | "SearchMemories"
        | "ListMemories"
        | "ExtractConceptsFromFile"
        | "BuildContext"
        | "Visualize"
        | "FindSimilarConcepts"
        | "AnalyzeConceptCorruption"
        | "MemoryStatus"
        | "GetConfig"
        | "RunFullBenchmark"
        | "Adopt"
        | "ListAssets"
        | "ReadMcpResource"
        | "ListSessions"
        | "ReadTextFile"
        | "RecentActivity" => SafetyClass::Read,
        "WriteMemory" => SafetyClass::Write,
        "DeleteMemory" => SafetyClass::Destructive,
        "Workflow" | "Pipeline" | "SequentialThinking" | "AssumptionLedger" | "AcpExec"
        | "Schedule" => SafetyClass::Orchestration,
        "EditMemory" | "MoveMemory" | "Sync" | "RepairConcepts" | "UpdateAssets"
        | "TeamsMonitor" | "EmbeddingModels" | "ReloadConfig" | "UpdateSettings" | "AddAgent"
        | "UpdateAgent" | "AddMcpServer" | "UpdateMcpServer" => SafetyClass::Mutate,
        "DeleteAgent" | "DeleteMcpServer" => SafetyClass::Destructive,
        _ => SafetyClass::Mutate,
    }
}

pub(crate) fn gate_tool_safety(tool_name: &str) -> SafetyClass {
    match tool_name {
        "status" | "history" | "task_body" => SafetyClass::Read,
        "cancel" => SafetyClass::Destructive,
        "sync" | "append" | "pass" | "fail" => SafetyClass::Mutate,
        _ => SafetyClass::Mutate,
    }
}
