use super::{
    SKIP_HASH_GUARD_MARKER, SKIP_MTIME_GUARD_MARKER, SYNCED_MENTIONS_MARKER,
    file_index::{
        indexed_timestamp_for_metadata, load_file_concepts, load_indexed_file_state,
        load_total_mentions_for_file, should_skip_by_mtime, tx_delete_file_index,
        upsert_processed_file,
    },
    graph_state::{
        prune_orphaned_concepts, purge_unreferenced_concepts, rebuild_affected_graph_from_mentions,
    },
    sync_state::{
        current_iso_timestamp, ensure_incremental_sync_state_table, record_incremental_graph_change,
    },
    vector_diagnostics::{
        VectorSyncDiagnostics, VectorUnavailableReasonCodes, build_vector_unavailable_report,
        vector_sync_disabled_by_config,
    },
    vector_embeddings::{
        generate_concept_embeddings, generate_file_embedding, vec0_extension_available,
    },
};
use crate::db::{self, GraphDatabase};
use crate::error::Result;
use crate::markdown::{WikiLinkFilter, extract_wikilinks, path_to_uri, read_markdown};
use crate::tools::ToolContext;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[allow(clippy::too_many_lines)]
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

// T-64.4: RTM NFR-1 moved here from vector_diagnostics — co-located with its only caller.
fn collect_vector_skip_diagnostics_for_paths(
    file_paths: &[PathBuf],
) -> Result<VectorSyncDiagnostics> {
    let mut diagnostics = VectorSyncDiagnostics::default();
    let mut concepts = BTreeSet::new();

    for file_path in file_paths {
        let content = std::fs::read_to_string(file_path)?;
        diagnostics.record_file_skipped();
        concepts.extend(extract_wikilinks(&content));
    }

    diagnostics.record_concept_skipped(concepts.len());
    Ok(diagnostics)
}
