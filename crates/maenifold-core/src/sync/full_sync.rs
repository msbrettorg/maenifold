use super::sync_state::{
    COMMUNITY_LAST_CHANGE_KEY, COMMUNITY_PENDING_COUNT_KEY, SyncSummary, community_gamma,
    community_recompute_file_threshold, community_recompute_time_threshold, current_iso_timestamp,
    ensure_incremental_sync_state_table, generate_sync_holder_id, load_help_doc,
    load_incremental_state_i64, load_incremental_state_timestamp, render_sync_markdown,
    reset_incremental_community_state, sync_role,
};
use crate::community;
use crate::db::{self, GraphDatabase};
use crate::error::Result;
use crate::graph;
use crate::sync::leader::LeaderElection;
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use serde_json::Value;

#[allow(clippy::too_many_lines)]
pub fn sync(context: &ToolContext, payload: &Value) -> Result<String> {
    let learn = payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if learn {
        let help = load_help_doc(&context.config.assets_path, "Sync")
            .unwrap_or_else(|| "ERROR: Help file not found for Sync".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }

    if let Err(error) = GraphDatabase::initialize_database(&context.config) {
        let message = error.to_string();
        // T-0.1: RTM FR-25.2 preserves BuildContext availability when community enrichment schema is degraded.
        if message.contains("concept_communities") || message.contains("community_id") {
            graph::emit_graph_tools_diagnostic(
                "EnrichWithCommunityData",
                "Community enrichment schema is degraded; continuing without community data",
                &message,
            );
        } else {
            return Err(error);
        }
    }

    let configured_role = sync_role();
    if configured_role == "follower" {
        return Ok(render_sync_markdown(
            SyncSummary {
                status: "skipped",
                files_processed: 0,
                concept_mentions: 0,
                concepts_filtered: 0,
                concept_relations: 0,
                abandoned_sessions: 0,
                communities_detected: 0,
                file_errors: Vec::new(),
                vector_warning_code: None,
                // T-21.7: RTM FR-21.7 follower path does not execute sync phases.
                files_changed: 0,
                file_processing_secs: 0.0,
                graph_rebuild_secs: 0.0,
                embeddings_secs: 0.0,
                community_secs: 0.0,
                total_secs: 0.0,
            },
            None,
        ));
    }

    // T-4.6: RTM FR-30.1
    let holder_id = generate_sync_holder_id();
    let leader_election = LeaderElection::new(&context.config.database_path);
    let (lease_owned, _reason_code, epoch) = if configured_role == "leader" {
        leader_election.force_acquire(&holder_id, context.config.sync_lease_ms())?
    } else {
        leader_election.try_acquire(&holder_id, context.config.sync_lease_ms())?
    };
    if !lease_owned && configured_role == "auto" {
        return Ok("Skipped: another process is leader".to_string());
    }

    // T-64.4: RTM NFR-1 delegates full-sync execution to the orchestrator submodule.
    let sync_result = super::orchestrator::run_full_sync(context);
    if lease_owned {
        let _ = leader_election.release(&holder_id, epoch);
    }
    sync_result
}

pub fn recompute_communities_if_needed(context: &ToolContext) -> Result<Option<usize>> {
    GraphDatabase::initialize_database(&context.config)?;
    let connection = db::open_connection(&context.config.database_path)?;
    ensure_incremental_sync_state_table(&connection)?;
    let pending_count = load_incremental_state_i64(&connection, COMMUNITY_PENDING_COUNT_KEY)?;
    if pending_count <= 0 {
        return Ok(None);
    }

    let now = chrono::Utc::now();
    let last_change_at = load_incremental_state_timestamp(&connection, COMMUNITY_LAST_CHANGE_KEY);
    let should_recompute = pending_count >= community_recompute_file_threshold() as i64
        || last_change_at
            .map(|timestamp| {
                now.signed_duration_since(timestamp) >= community_recompute_time_threshold()
            })
            .unwrap_or(true);
    if !should_recompute {
        return Ok(None);
    }

    let communities =
        community::detect_and_persist(&context.config.database_path, community_gamma(), None)?;
    let recomputed_at = current_iso_timestamp();
    reset_incremental_community_state(&connection, &recomputed_at)?;
    Ok(Some(communities))
}
