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
    /// Capture any value the LLM emits that does not match the canonical
    /// vocabulary. The intent resolver snaps Unknown to the catalog's
    /// canonical safety_class before returning the candidate.
    #[serde(other)]
    Unknown,
}

impl fmt::Display for SafetyClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Mutate => "mutate",
            Self::Destructive => "destructive",
            Self::Orchestration => "orchestration",
            Self::Unknown => "unknown",
        };
        f.write_str(value)
    }
}

impl std::str::FromStr for SafetyClass {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "read" => Ok(Self::Read),
            "write" => Ok(Self::Write),
            "mutate" => Ok(Self::Mutate),
            "destructive" => Ok(Self::Destructive),
            "orchestration" => Ok(Self::Orchestration),
            _ => Err(()),
        }
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
    ("DeleteWorkflowAsset", "maenifold.workflow"),
    ("SaveWorkflowAsset", "maenifold.workflow"),
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
    ("AcpExec", "maenifold.system"),
    ("EmbeddingModels", "maenifold.system"),
    ("ReadTextFile", "maenifold.system"),
    ("Schedule", "maenifold.system"),
    ("RecentActivity", "maenifold.system"),
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
        "DeleteMemory" | "DeleteWorkflowAsset" => SafetyClass::Destructive,
        "Workflow" | "Pipeline" | "SequentialThinking" | "AssumptionLedger" | "AcpExec"
        | "Schedule" => SafetyClass::Orchestration,
        "EditMemory" | "MoveMemory" | "Sync" | "RepairConcepts" | "UpdateAssets"
        | "EmbeddingModels" | "SaveWorkflowAsset" => SafetyClass::Mutate,
        _ => SafetyClass::Mutate,
    }
}

pub(crate) fn gate_tool_safety(tool_name: &str) -> SafetyClass {
    match tool_name {
        "status" | "history" | "task_body" => SafetyClass::Read,
        "cancel" => SafetyClass::Destructive,
        "sync" | "append" | "create" | "pass" | "fail" => SafetyClass::Mutate,
        _ => SafetyClass::Mutate,
    }
}
