use crate::community;
use crate::db;
use crate::error::Result;
use crate::markdown::{WikiLinkFilter, get_memory_files, read_markdown, uri_to_path};
use crate::sessions;
use crate::tools::ToolContext;
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::time::Instant;

use super::file_index::{
    delete_missing_files, indexed_timestamp_for_metadata, load_indexed_file_state,
    should_bootstrap_graph_state, should_skip_by_mtime, upsert_processed_file,
};
use super::graph_state::rebuild_graph_from_mentions;
use super::sync_state::{
    SyncSummary, community_gamma, current_iso_timestamp, render_sync_markdown,
    reset_incremental_community_state,
};
use super::vector_diagnostics::{
    VectorSyncDiagnostics, VectorUnavailableReasonCodes, build_vector_failure_report,
    build_vector_unavailable_report, build_vector_verification_report,
    vector_sync_disabled_by_config,
};
use super::vector_embeddings::{
    generate_concept_embeddings, generate_file_embedding, load_all_concept_names,
    vec0_extension_available,
};

#[allow(clippy::too_many_lines)]
pub(super) fn run_full_sync(context: &ToolContext) -> Result<String> {
    let sync_start = Instant::now();
    let mut connection = db::open_connection(&context.config.database_path)?;
    let vector_disabled = vector_sync_disabled_by_config();
    let vector_ready =
        !vector_disabled && vec0_extension_available(&connection, &context.config.assets_path);
    let file_uris = get_memory_files(&context.config, None)?;
    let file_uri_set = file_uris.iter().cloned().collect::<HashSet<_>>();
    let file_processing_start = Instant::now();
    let tx = connection.transaction()?;
    let sync_started_at = current_iso_timestamp();
    let mut processed_files = Vec::new();
    let mut file_errors = Vec::new();
    let wikilink_filter = WikiLinkFilter::new(&context.config.memory_path);
    let mut total_filtered = 0i64;

    delete_missing_files(&tx, &file_uri_set)?;
    for uri in &file_uris {
        tx.execute_batch("SAVEPOINT sync_file")?;
        let file_result: Result<()> = (|| {
            let file_path = uri_to_path(uri, &context.config.memory_path)?;
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

    let files_changed = processed_files.len() as i64;
    let file_processing_elapsed = file_processing_start.elapsed();
    let graph_start = Instant::now();
    let total_mentions = rebuild_graph_from_mentions(&tx, &sync_started_at)?;
    let graph_elapsed = graph_start.elapsed();
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
    cleanup_orphan_vec_memory_files(&connection, &context.config.memory_path)?;

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
    let vector_section = render_vector_section(
        &connection,
        &vector_diagnostics,
        vector_disabled,
        vector_ready,
        file_uris.is_empty(),
    );

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
            vector_warning_code: file_uris
                .is_empty()
                .then_some("VECTOR_TABLES_EMPTY_AFTER_SYNC"),
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

fn cleanup_orphan_vec_memory_files(
    connection: &rusqlite::Connection,
    memory_path: &std::path::Path,
) -> Result<()> {
    let vec_memory_files_exists = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'vec_memory_files'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count > 0)?;
    if !vec_memory_files_exists {
        return Ok(());
    }

    let mut statement = connection.prepare("SELECT file_path FROM vec_memory_files")?;
    let paths = statement.query_map([], |row| row.get::<_, String>(0))?;
    let mut orphan_paths = BTreeSet::new();
    for path in paths {
        let path = path?;
        let exists_on_disk = uri_to_path(&path, memory_path)
            .map(|file_path| file_path.exists())
            .unwrap_or(false);
        if !exists_on_disk {
            orphan_paths.insert(path);
        }
    }
    if !orphan_paths.is_empty() {
        crate::search::cleanup_orphan_vec_memory_files(connection, &orphan_paths)?;
    }
    Ok(())
}

fn render_vector_section(
    connection: &rusqlite::Connection,
    diagnostics: &VectorSyncDiagnostics,
    vector_disabled: bool,
    vector_ready: bool,
    no_files_found: bool,
) -> Option<String> {
    if vector_disabled {
        return Some(
            build_vector_unavailable_report(
                diagnostics,
                VectorUnavailableReasonCodes::DISABLED_BY_CONFIG,
            )
            .join("\n"),
        );
    }
    if !vector_ready {
        return Some(
            build_vector_unavailable_report(diagnostics, VectorUnavailableReasonCodes::UNKNOWN)
                .join("\n"),
        );
    }
    if diagnostics.total_failures() > 0 || diagnostics.total_skipped() > 0 {
        return Some(build_vector_failure_report(diagnostics).join("\n"));
    }
    if no_files_found {
        return None;
    }

    let verification = build_vector_verification_report(connection);
    (!verification.is_empty()).then(|| verification.join("\n"))
}
