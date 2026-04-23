mod file_index;
mod file_sync;
mod full_sync;
mod graph_relations;
mod graph_state;
mod orchestrator;
// T-12.1: RTM NFR-1 keeps sync internals grouped under the existing [[sync]] facade.
pub mod leader;
mod sync_state;
mod vector_diagnostics;
mod vector_embeddings;
mod vector_embeddings_helpers;
mod file_index_lineage;
mod watcher_batch;
// T-64.4: RTM NFR-1 extracted watcher health tracking into a dedicated submodule.
mod watcher_health;
pub mod watchers;

// T-12.0: RTM NFR-1 — shared guard marker constants prevent silent test rot on message refactoring.
pub const SKIP_MTIME_GUARD_MARKER: &str = "mtime guard";
pub const SKIP_HASH_GUARD_MARKER: &str = "hash guard";
pub const SYNCED_MENTIONS_MARKER: &str = "concept mentions updated";

// T-12.4: RTM NFR-1 preserves the caller-facing [[sync]] facade while splitting the runtime
// into focused submodules that still export `crate::sync::*` for existing call sites.
pub use self::{
    file_sync::{sync_files, sync_single_file},
    full_sync::{recompute_communities_if_needed, sync},
    vector_diagnostics::{
        VectorFailureReasonCodes, VectorSyncDiagnostics, VectorUnavailableReasonCodes,
        build_vector_failure_report, build_vector_unavailable_report,
    },
    vector_embeddings::{generate_concept_embeddings, generate_file_embedding},
};
