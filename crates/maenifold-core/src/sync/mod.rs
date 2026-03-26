mod file_index;
mod graph_relations;
mod graph_state;
// T-12.1: RTM NFR-1 keeps sync internals grouped under the existing [[sync]] facade.
pub mod leader;
mod sync_state;
mod vector_diagnostics;
mod vector_embeddings;
pub mod watchers;

use crate::community;
use crate::db::{self, GraphDatabase};
use crate::error::Result;
use crate::graph;
use crate::markdown::{WikiLinkFilter, get_memory_files, path_to_uri, read_markdown, uri_to_path};
use crate::sessions;
use crate::sync::leader::LeaderElection;
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use serde_json::Value;
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

// T-12.0: RTM NFR-1 — shared guard marker constants prevent silent test rot on message refactoring.
pub const SKIP_MTIME_GUARD_MARKER: &str = "mtime guard";
pub const SKIP_HASH_GUARD_MARKER: &str = "hash guard";
pub const SYNCED_MENTIONS_MARKER: &str = "concept mentions updated";

use self::{
    file_index::{
        delete_missing_files, indexed_timestamp_for_metadata, load_file_concepts,
        load_indexed_file_state, load_total_mentions_for_file, should_bootstrap_graph_state,
        should_skip_by_mtime, tx_delete_file_index, upsert_processed_file,
    },
    graph_state::{
        prune_orphaned_concepts, purge_unreferenced_concepts, rebuild_affected_graph_from_mentions,
        rebuild_graph_from_mentions,
    },
    sync_state::{
        COMMUNITY_LAST_CHANGE_KEY, COMMUNITY_PENDING_COUNT_KEY, SyncSummary, community_gamma,
        community_recompute_file_threshold, community_recompute_time_threshold,
        current_iso_timestamp, ensure_incremental_sync_state_table, generate_sync_holder_id,
        load_help_doc, load_incremental_state_i64, load_incremental_state_timestamp,
        record_incremental_graph_change, render_sync_markdown, reset_incremental_community_state,
        sync_role,
    },
    vector_diagnostics::{
        build_vector_verification_report, collect_vector_skip_diagnostics_for_paths,
        vector_sync_disabled_by_config,
    },
    vector_embeddings::{load_all_concept_names, vec0_extension_available},
};

// T-12.4: RTM NFR-1 preserves the caller-facing [[sync]] facade while splitting the runtime
// into focused submodules that still export `crate::sync::*` for existing call sites.
pub use self::{
    vector_diagnostics::{
        VectorFailureReasonCodes, VectorSyncDiagnostics, VectorUnavailableReasonCodes,
        build_vector_failure_report, build_vector_unavailable_report,
    },
    vector_embeddings::{generate_concept_embeddings, generate_file_embedding},
};

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

    let sync_result = run_sync(context);
    if lease_owned {
        let _ = leader_election.release(&holder_id, epoch);
    }
    sync_result
}

pub fn sync_single_file(context: &ToolContext, file_path: &Path) -> Result<String> {
    GraphDatabase::initialize_database(&context.config)?;
    let file_uri = path_to_uri(file_path, &context.config.memory_path)?;
    let sync_started_at = current_iso_timestamp();
    let mut connection = db::open_connection(&context.config.database_path)?;
    let tx = connection.transaction()?;
    // T-22.3: RTM-154, RTM-155 — incremental [[sync]] must honor [[WikiLink Filter]] and degrade to no-op if absent.
    let wikilink_filter = WikiLinkFilter::new(&context.config.memory_path);
    ensure_incremental_sync_state_table(&tx)?;

    let indexed_state = load_indexed_file_state(&tx, &file_uri)?;
    let previous_concepts = load_file_concepts(&tx, &file_uri)?;
    if !file_path.exists() {
        if indexed_state.is_none() && previous_concepts.is_empty() {
            tx.commit()?;
            return Ok(format!("Skipped {file_uri}: file not indexed"));
        }
        tx_delete_file_index(&tx, &file_uri)?;
        let relation_count =
            rebuild_affected_graph_from_mentions(&tx, &previous_concepts, &sync_started_at)?;
        prune_orphaned_concepts(&tx, &previous_concepts)?;
        purge_unreferenced_concepts(&tx)?;
        record_incremental_graph_change(&tx, &sync_started_at)?;
        tx.commit()?;
        return Ok(format!(
            "Synced {file_uri}: removed stale index entry and rebuilt {relation_count} affected relations"
        ));
    }

    let metadata = fs::metadata(file_path)?;
    let indexed_at = indexed_timestamp_for_metadata(&metadata);
    let last_indexed = indexed_state
        .as_ref()
        .and_then(|state| state.last_indexed.as_deref());
    let stored_hash = indexed_state
        .as_ref()
        .and_then(|state| state.file_md5.as_deref());
    if should_skip_by_mtime(metadata.modified().ok(), last_indexed) {
        tx.commit()?;
        return Ok(format!("Skipped {file_uri}: {SKIP_MTIME_GUARD_MARKER}"));
    }

    let (frontmatter, content, checksum) = read_markdown(&file_uri, &context.config)?;
    if stored_hash == Some(checksum.as_str()) {
        tx.execute(
            "UPDATE file_content SET last_indexed = ?2 WHERE file_path = ?1",
            rusqlite::params![file_uri, indexed_at],
        )?;
        tx.commit()?;
        return Ok(format!("Skipped {file_uri}: {SKIP_HASH_GUARD_MARKER}"));
    }

    let file_filtered = upsert_processed_file(
        &tx,
        &file_uri,
        &frontmatter,
        &content,
        &checksum,
        metadata.len() as i64,
        &indexed_at,
        Some(&wikilink_filter),
    )?;
    let new_concepts = load_file_concepts(&tx, &file_uri)?;
    let affected_concepts = previous_concepts.union(&new_concepts).cloned().collect();
    let relation_count =
        rebuild_affected_graph_from_mentions(&tx, &affected_concepts, &sync_started_at)?;
    prune_orphaned_concepts(&tx, &affected_concepts)?;
    purge_unreferenced_concepts(&tx)?;
    record_incremental_graph_change(&tx, &sync_started_at)?;
    // T-15.20: RTM FR-8 — check vec0 extension availability before generating embeddings,
    // matching the pattern in run_sync and C# TryEnsureVectorSupport.
    let vector_ready = !vector_sync_disabled_by_config()
        && vec0_extension_available(&tx, &context.config.assets_path);
    let mut vector_diagnostics = VectorSyncDiagnostics::default();
    let concept_list = new_concepts.iter().cloned().collect::<Vec<_>>();
    let _generated_concepts =
        generate_concept_embeddings(&tx, &concept_list, vector_ready, &mut vector_diagnostics);
    let _generated_file_embedding = generate_file_embedding(
        &tx,
        &file_uri,
        &content,
        vector_ready,
        &mut vector_diagnostics,
    );
    let mention_count = load_total_mentions_for_file(&tx, &file_uri)?;
    tx.commit()?;
    Ok(format!(
        // T-22.5: RTM FR-19 — report incremental [[WikiLink Filter]] exclusions in sync output.
        "Synced {file_uri}: {mention_count} {SYNCED_MENTIONS_MARKER}, {file_filtered} filtered, {relation_count} affected relations rebuilt"
    ))
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

pub fn sync_files(context: &ToolContext, file_paths: &[PathBuf]) -> Result<String> {
    // T-22.3: delegates to sync_single_file which applies [[WikiLink Filter]] per RTM-154.
    let mut file_errors = Vec::new();
    for file_path in file_paths {
        if let Err(error) = sync_single_file(context, file_path) {
            let error_message = error.to_string();
            eprintln!(
                "SyncFiles failed for {}: {}",
                file_path.display(),
                error_message
            );
            file_errors.push((file_path.display().to_string(), error_message));
        }
    }

    let mut output = format!(
        "SyncFiles complete:\n- STATUS: {}\n- {} FILES processed",
        if file_errors.is_empty() {
            "success"
        } else {
            "partial"
        },
        file_paths.len()
    );
    if vector_sync_disabled_by_config() {
        let diagnostics = collect_vector_skip_diagnostics_for_paths(file_paths)?;
        output.push('\n');
        output.push_str(
            &build_vector_unavailable_report(
                &diagnostics,
                VectorUnavailableReasonCodes::DISABLED_BY_CONFIG,
            )
            .join("\n"),
        );
    }
    if !file_errors.is_empty() {
        output.push_str("\n\n## Errors");
        for (file_path, error_message) in &file_errors {
            output.push_str(&format!("\n- `{file_path}`: {error_message}"));
        }
    }
    Ok(output)
}

fn run_sync(context: &ToolContext) -> Result<String> {
    // T-21.7: RTM FR-21.7 capture total [[sync]] runtime from the first instruction in run_sync.
    let sync_start = Instant::now();
    let mut connection = db::open_connection(&context.config.database_path)?;
    let vector_disabled = vector_sync_disabled_by_config();
    let vector_ready =
        !vector_disabled && vec0_extension_available(&connection, &context.config.assets_path);
    let file_uris = get_memory_files(&context.config, None)?;
    let file_uri_set = file_uris.iter().cloned().collect::<HashSet<_>>();
    // T-21.7: RTM FR-21.7 measure file processing separately from graph/vector/community phases.
    let file_processing_start = Instant::now();
    let tx = connection.transaction()?;
    let sync_started_at = current_iso_timestamp();
    let mut processed_files = Vec::new();
    let mut file_errors = Vec::new();
    // T-22.2: RTM-151, RTM-152, RTM-153, RTM-155 — full [[sync]] must filter blocked concepts before mention insertion.
    let wikilink_filter = WikiLinkFilter::new(&context.config.memory_path);
    let mut total_filtered = 0i64;

    delete_missing_files(&tx, &file_uri_set)?;
    for uri in &file_uris {
        tx.execute_batch("SAVEPOINT sync_file")?;
        let file_result: Result<()> = (|| {
            let file_path = crate::markdown::uri_to_path(uri, &context.config.memory_path)?;
            let metadata = fs::metadata(&file_path)?;
            let indexed_at = indexed_timestamp_for_metadata(&metadata);
            let stored_file = load_indexed_file_state(&tx, uri)?;
            let synced_file = stored_file
                .as_ref()
                .filter(|indexed_file| !should_bootstrap_graph_state(indexed_file));
            let stored_hash = synced_file.and_then(|indexed_file| indexed_file.file_md5.as_deref());
            let last_indexed =
                synced_file.and_then(|indexed_file| indexed_file.last_indexed.as_deref());
            if should_skip_by_mtime(metadata.modified().ok(), last_indexed) {
                return Ok(());
            }

            let (frontmatter, content, checksum) = read_markdown(uri, &context.config)?;
            if stored_hash == Some(checksum.as_str()) {
                tx.execute(
                    "UPDATE file_content SET last_indexed = ?2 WHERE file_path = ?1",
                    rusqlite::params![uri, indexed_at],
                )?;
                return Ok(());
            }

            let file_filtered = upsert_processed_file(
                &tx,
                uri,
                &frontmatter,
                &content,
                &checksum,
                metadata.len() as i64,
                &indexed_at,
                Some(&wikilink_filter),
            )?;
            total_filtered += file_filtered;
            processed_files.push((uri.clone(), content));
            Ok(())
        })();

        match file_result {
            Ok(()) => tx.execute_batch("RELEASE SAVEPOINT sync_file")?,
            Err(error) => {
                let error_message = error.to_string();
                eprintln!("Sync failed for {uri}: {error_message}");
                tx.execute_batch("ROLLBACK TO SAVEPOINT sync_file; RELEASE SAVEPOINT sync_file;")?;
                file_errors.push((uri.clone(), error_message));
            }
        }
    }

    // T-21.7: RTM FR-21.7 include changed-file count in the sync timing output.
    let files_changed = processed_files.len() as i64;
    let file_processing_elapsed = file_processing_start.elapsed();
    // T-21.7: RTM FR-21.7 graph rebuild timing.
    let graph_start = Instant::now();
    let total_mentions = rebuild_graph_from_mentions(&tx, &sync_started_at)?;
    let graph_elapsed = graph_start.elapsed();
    // T-21.7: RTM FR-21.7 embedding generation timing for concepts and files.
    let embeddings_start = Instant::now();
    let concept_list = load_all_concept_names(&tx)?;
    let mut vector_diagnostics = VectorSyncDiagnostics::default();
    let _generated_concepts =
        generate_concept_embeddings(&tx, &concept_list, vector_ready, &mut vector_diagnostics);
    for (file_uri, content) in &processed_files {
        let _generated_file_embedding = generate_file_embedding(
            &tx,
            file_uri,
            content,
            vector_ready,
            &mut vector_diagnostics,
        );
    }
    let embeddings_elapsed = embeddings_start.elapsed();

    tx.commit()?;
    let vec_memory_files_exists = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'vec_memory_files'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count > 0)?;
    if vec_memory_files_exists {
        let orphan_paths = {
            let mut statement = connection.prepare("SELECT file_path FROM vec_memory_files")?;
            let paths = statement.query_map([], |row| row.get::<_, String>(0))?;
            let mut orphan_paths = BTreeSet::new();
            for path in paths {
                let path = path?;
                let exists_on_disk = uri_to_path(&path, &context.config.memory_path)
                    .map(|file_path| file_path.exists())
                    .unwrap_or(false);
                if !exists_on_disk {
                    orphan_paths.insert(path);
                }
            }
            orphan_paths
        };
        if !orphan_paths.is_empty() {
            crate::search::cleanup_orphan_vec_memory_files(&connection, &orphan_paths)?;
        }
    }

    // T-21.7: RTM FR-21.7 community detection timing includes cleanup and incremental-state reset.
    let community_start = Instant::now();
    let abandoned_sessions = sessions::cleanup_abandoned_sessions(&context.config)? as i64;
    let relation_count: i64 =
        connection.query_row("SELECT COUNT(*) FROM concept_graph", [], |row| row.get(0))?;
    let communities_detected =
        community::detect_and_persist(&context.config.database_path, community_gamma(), None)?
            as i64;
    reset_incremental_community_state(&connection, &sync_started_at)?;
    let community_elapsed = community_start.elapsed();
    let total_elapsed = sync_start.elapsed();

    let vector_section = if vector_disabled {
        Some(
            build_vector_unavailable_report(
                &vector_diagnostics,
                VectorUnavailableReasonCodes::DISABLED_BY_CONFIG,
            )
            .join("\n"),
        )
    } else if !vector_ready {
        Some(
            build_vector_unavailable_report(
                &vector_diagnostics,
                VectorUnavailableReasonCodes::UNKNOWN,
            )
            .join("\n"),
        )
    } else if vector_diagnostics.total_failures() > 0 || vector_diagnostics.total_skipped() > 0 {
        Some(build_vector_failure_report(&vector_diagnostics).join("\n"))
    } else {
        // T-21.3: RTM-150 — render post-commit vec0 verification from live table contents.
        let verification = build_vector_verification_report(&connection);
        if verification.is_empty() {
            None
        } else {
            Some(verification.join("\n"))
        }
    };

    Ok(render_sync_markdown(
        SyncSummary {
            status: if file_uris.is_empty() {
                "degraded"
            } else if !file_errors.is_empty() {
                "partial"
            } else {
                "success"
            },
            files_processed: file_uris.len() as i64,
            concept_mentions: total_mentions as i64,
            concepts_filtered: total_filtered,
            concept_relations: relation_count,
            abandoned_sessions,
            communities_detected,
            file_errors,
            vector_warning_code: if file_uris.is_empty() {
                Some("VECTOR_TABLES_EMPTY_AFTER_SYNC")
            } else {
                None
            },
            // T-21.7: RTM FR-21.7 phase timing telemetry for [[sync]] markdown output.
            files_changed,
            file_processing_secs: file_processing_elapsed.as_secs_f64(),
            graph_rebuild_secs: graph_elapsed.as_secs_f64(),
            embeddings_secs: embeddings_elapsed.as_secs_f64(),
            community_secs: community_elapsed.as_secs_f64(),
            total_secs: total_elapsed.as_secs_f64(),
        },
        vector_section.as_deref(),
    ))
}
