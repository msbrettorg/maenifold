use crate::assets;
use crate::benchmark;
use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use crate::graph;
use crate::graph::analyzer;
use crate::mcp::config_tools;
use crate::memory;
use crate::repair;
use crate::search;
use crate::sessions;
use crate::similar;
use crate::sync;
use crate::thinking;
use crate::tools::{self, ToolContext};
use crate::util::json::OutputFormat;
use crate::workflow;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::LazyLock;

pub type ToolFn = fn(&ToolContext, &Value) -> Result<String>;

#[derive(Clone, Copy)]
pub struct ToolDescriptor {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub invoke: ToolFn,
}

pub struct ToolRegistry {
    descriptors: BTreeMap<String, ToolDescriptor>,
}

// T-0.3d: RTM-047 (NFR-4)
impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    pub fn global() -> &'static Self {
        static REGISTRY: LazyLock<ToolRegistry> = LazyLock::new(ToolRegistry::new);
        &REGISTRY
    }

    pub fn new() -> Self {
        let mut descriptors = BTreeMap::new();
        for descriptor in DESCRIPTORS {
            descriptors.insert(descriptor.name.to_ascii_lowercase(), *descriptor);
            for alias in descriptor.aliases {
                descriptors.insert(alias.to_ascii_lowercase(), *descriptor);
            }
        }
        Self { descriptors }
    }

    pub fn try_invoke(
        &self,
        name: &str,
        payload: &Value,
        output_format: OutputFormat,
    ) -> Result<String> {
        let descriptor = self
            .descriptors
            .get(&name.to_ascii_lowercase())
            .ok_or_else(|| MaenifoldError::UnknownTool(name.to_string()))?;
        let config = Config::current();
        config.ensure_directories()?;
        let context = ToolContext::new(config, output_format);
        (descriptor.invoke)(&context, payload)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.descriptors.contains_key(&name.to_ascii_lowercase())
    }

    pub fn canonical_names(&self) -> Vec<&'static str> {
        DESCRIPTORS.iter().map(|d| d.name).collect()
    }
}

const DESCRIPTORS: &[ToolDescriptor] = &[
    ToolDescriptor {
        name: "WriteMemory",
        aliases: &["writememory"],
        invoke: memory::write_memory,
    },
    ToolDescriptor {
        name: "ReadMemory",
        aliases: &["readmemory"],
        invoke: memory::read_memory,
    },
    ToolDescriptor {
        name: "EditMemory",
        aliases: &["editmemory"],
        invoke: memory::edit_memory,
    },
    ToolDescriptor {
        name: "DeleteMemory",
        aliases: &["deletememory"],
        invoke: memory::delete_memory,
    },
    ToolDescriptor {
        name: "MoveMemory",
        aliases: &["movememory"],
        invoke: memory::move_memory,
    },
    ToolDescriptor {
        name: "SearchMemories",
        aliases: &["searchmemories"],
        invoke: search::search_memories,
    },
    ToolDescriptor {
        name: "ExtractConceptsFromFile",
        aliases: &["extractconceptsfromfile"],
        invoke: search::extract_concepts_from_file,
    },
    ToolDescriptor {
        name: "Sync",
        aliases: &["sync"],
        invoke: sync::sync,
    },
    ToolDescriptor {
        name: "BuildContext",
        aliases: &["buildcontext"],
        // T-12.7 structural refactor: keep BuildContext routed through graph.rs.
        invoke: graph::build_context,
    },
    ToolDescriptor {
        name: "Visualize",
        aliases: &["visualize"],
        // T-12.0: RTM NFR-1 keeps Visualize routed through graph/analyzer.rs after the module reorg.
        invoke: analyzer::visualize,
    },
    ToolDescriptor {
        name: "SequentialThinking",
        aliases: &["sequentialthinking"],
        // T-11.8: RTM FR-11 - route SequentialThinking through the extracted thinking module.
        invoke: thinking::sequential_thinking,
    },
    ToolDescriptor {
        name: "Workflow",
        aliases: &["workflow"],
        invoke: workflow::workflow,
    },
    ToolDescriptor {
        name: "Pipeline",
        aliases: &["pipeline"],
        invoke: workflow::pipeline::pipeline,
    },
    ToolDescriptor {
        name: "RecentActivity",
        aliases: &["recentactivity"],
        // T-2.1: RTM FR-4
        invoke: tools::recent_activity,
    },
    ToolDescriptor {
        name: "MemoryStatus",
        aliases: &["memorystatus"],
        invoke: tools::memory_status,
    },
    ToolDescriptor {
        name: "GetConfig",
        aliases: &["getconfig"],
        invoke: tools::get_config,
    },
    ToolDescriptor {
        name: "GetHelp",
        aliases: &["gethelp"],
        invoke: memory::get_help,
    },
    ToolDescriptor {
        name: "ListMemories",
        aliases: &["listmemories"],
        invoke: memory::list_memories,
    },
    ToolDescriptor {
        name: "UpdateAssets",
        aliases: &["updateassets"],
        invoke: assets::update_assets,
    },
    ToolDescriptor {
        name: "RepairConcepts",
        aliases: &["repairconcepts"],
        // T-3.1a: RTM FR-6
        // T-12.2: RTM FR-6 - route RepairConcepts through [[repair.rs]].
        invoke: repair::repair_concepts,
    },
    ToolDescriptor {
        name: "AnalyzeConceptCorruption",
        aliases: &["analyzeconceptcorruption"],
        // T-1.2: RTM FR-2
        // T-12.2: RTM FR-2 - route AnalyzeConceptCorruption through [[repair.rs]].
        invoke: repair::analyze_concept_corruption,
    },
    ToolDescriptor {
        name: "FindSimilarConcepts",
        aliases: &["findsimilarconcepts"],
        invoke: similar::find_similar_concepts,
    },
    // StartWatcher/StopWatcher removed — vestigial tools with no purpose
    ToolDescriptor {
        name: "RunFullBenchmark",
        aliases: &["runfullbenchmark"],
        // T-2.2: RTM FR-5
        invoke: benchmark::run_full_benchmark,
    },
    ToolDescriptor {
        name: "Adopt",
        aliases: &["adopt"],
        invoke: assets::adopt,
    },
    ToolDescriptor {
        name: "AssumptionLedger",
        aliases: &["assumptionledger"],
        invoke: sessions::assumption_ledger,
    },
    ToolDescriptor {
        name: "ListAssets",
        aliases: &["listassets"],
        invoke: tools::list_assets,
    },
    ToolDescriptor {
        name: "ReadMcpResource",
        aliases: &["readmcpresource"],
        invoke: tools::read_mcp_resource,
    },
    ToolDescriptor {
        name: "ReadTextFile",
        aliases: &["readtextfile", "read_text_file", "read-text-file"],
        invoke: tools::read_text_file,
    },
    ToolDescriptor {
        name: "ListSessions",
        aliases: &["listsessions", "list_sessions"],
        invoke: tools::list_sessions,
    },
    ToolDescriptor {
        name: "AcpExec",
        aliases: &["acpexec", "acp_exec"],
        invoke: tools::acp_exec,
    },
    ToolDescriptor {
        name: "EmbeddingModels",
        aliases: &["embeddingmodels", "embedding_models"],
        invoke: tools::embedding_models,
    },
    ToolDescriptor {
        name: "TeamsMonitor",
        aliases: &["teamsmonitor", "teams_monitor"],
        invoke: tools::teams_monitor,
    },
    ToolDescriptor {
        name: "Schedule",
        aliases: &["schedule", "scheduled_tasks"],
        invoke: tools::schedule,
    },
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
