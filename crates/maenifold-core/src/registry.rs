use crate::assets;
use crate::benchmark;
use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use crate::graph;
use crate::json::OutputFormat;
use crate::memory;
use crate::search;
use crate::sessions;
use crate::tools::{self, ToolContext};
use crate::watchers;
use crate::workflow;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::BTreeMap;

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
        static REGISTRY: Lazy<ToolRegistry> = Lazy::new(ToolRegistry::new);
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
        invoke: search::sync,
    },
    ToolDescriptor {
        name: "BuildContext",
        aliases: &["buildcontext"],
        // T-11.7: RTM FR-13 preserves graph tool routing after extracting handlers from search.rs.
        invoke: graph::build_context,
    },
    ToolDescriptor {
        name: "Visualize",
        aliases: &["visualize"],
        // T-11.7: RTM FR-13 preserves graph tool routing after extracting handlers from search.rs.
        invoke: graph::visualize,
    },
    ToolDescriptor {
        name: "SequentialThinking",
        aliases: &["sequentialthinking"],
        invoke: sessions::sequential_thinking,
    },
    ToolDescriptor {
        name: "Workflow",
        aliases: &["workflow"],
        invoke: workflow::workflow,
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
        invoke: search::repair_concepts,
    },
    ToolDescriptor {
        name: "AnalyzeConceptCorruption",
        aliases: &["analyzeconceptcorruption"],
        // T-1.2: RTM FR-2
        invoke: search::analyze_concept_corruption,
    },
    ToolDescriptor {
        name: "FindSimilarConcepts",
        aliases: &["findsimilarconcepts"],
        // T-11.7: RTM FR-13 preserves graph tool routing after extracting handlers from search.rs.
        invoke: graph::find_similar_concepts,
    },
    ToolDescriptor {
        name: "StartWatcher",
        aliases: &["startwatcher"],
        invoke: watchers::start_watcher,
    },
    ToolDescriptor {
        name: "StopWatcher",
        aliases: &["stopwatcher"],
        // T-0.1: RTM NFR-3 — preserve legacy registry contract while direct helper callers keep detailed stop text.
        invoke: watchers::stop_watcher_tool,
    },
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
];
