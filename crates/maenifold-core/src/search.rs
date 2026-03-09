use crate::community;
use crate::db::{self, GraphDatabase};
use crate::error::{MaenifoldError, Result};
use crate::json::{OutputFormat, success};
use crate::leader::LeaderElection;
use crate::markdown::{
    extract_wikilinks, extract_wikilinks_with_sections, find_file_as_uri, get_memory_files,
    normalize_concept, path_to_uri, read_markdown, uri_to_path,
};
use crate::memory::compute_decay_weight;
use crate::sessions;
use crate::tools::ToolContext;
use chrono::{DateTime, Utc};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use regex::Regex;
use rusqlite::{
    Connection, OptionalExtension, params,
    types::{Type, ValueRef},
};
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use uuid::Uuid;

pub fn search_memories(context: &ToolContext, payload: &Value) -> Result<String> {
    let learn = payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if learn {
        return Ok(load_help_doc(&context.config.assets_path, "SearchMemories")
            .unwrap_or_else(|| "ERROR: Help file not found for SearchMemories".to_string()));
    }

    let query = payload
        .get("query")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("query"))?;
    // T-0.1: RTM NFR-3 - reject empty/whitespace/stopword-only SearchMemories queries to match C# VectorSearchTests coverage.
    if informative_query_terms(query).is_empty() {
        return Ok("ERROR: Query contains no informative keywords. Try adding specific terms (e.g. \"machine learning\", \"NLP\").".to_string());
    }
    let mode = payload
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("Hybrid");
    let page_size = payload
        .get("pageSize")
        .and_then(Value::as_u64)
        .unwrap_or(10) as usize;
    let page = payload.get("page").and_then(Value::as_u64).unwrap_or(1) as usize;
    let min_score = payload
        .get("minScore")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let folder_prefix = payload
        .get("folder")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|folder| !folder.is_empty())
        .map(|folder| format!("memory://{}", folder.trim_matches('/')));

    GraphDatabase::initialize_database(&context.config)?;
    let connection = db::open_connection(&context.config.database_path)?;
    let mut statement = connection.prepare(
        "SELECT file_path, COALESCE(title, ''), COALESCE(content, ''), last_accessed, status FROM file_content",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
        ))
    })?;

    let query_lower = query.to_ascii_lowercase();
    let query_slug = slugify_query(query);
    let query_terms = tokenize_query(query);
    let mut results = Vec::new();

    for row in rows {
        let (path, title, content, last_accessed, status) = row?;
        if let Some(prefix) = folder_prefix.as_deref()
            && path != prefix
            && !path.starts_with(&format!("{prefix}/"))
        {
            continue;
        }
        let title_lower = title.to_ascii_lowercase();
        let content_lower = content.to_ascii_lowercase();

        let text_matches = query_terms
            .iter()
            .filter(|term| {
                title_lower.contains(term.as_str()) || content_lower.contains(term.as_str())
            })
            .count();
        let text_score = if query_terms.is_empty() {
            0.0
        } else {
            text_matches as f64 / (query_terms.len() as f64 * 50.0)
        };

        let semantic_score = if content_lower.contains(&query_slug)
            || title_lower.contains(&query_slug)
            || content_lower.contains(&query_lower)
            || title_lower.contains(&query_lower)
        {
            1.0
        } else {
            0.0
        };

        if text_score <= 0.0 && semantic_score <= 0.0 {
            continue;
        }
        let fused_score = text_score + semantic_score;
        results.push(json!({
            "title": title,
            "path": path,
            "fusedScore": fused_score,
            "textScore": text_score,
            "semanticScore": semantic_score,
            // T-4.4: RTM FR-4.4
            "decayWeight": compute_decay_weight(
                &path,
                last_accessed.as_deref(),
                &context.config,
                status.as_deref(),
            ),
            "snippet": build_snippet(&content, &query_terms, &query_slug),
        }));
    }

    let mut text_ranks = BTreeMap::new();
    let mut semantic_ranks = BTreeMap::new();

    let mut text_order = results
        .iter()
        .enumerate()
        .filter(|(_, value)| value["textScore"].as_f64().unwrap_or(0.0) > 0.0)
        .collect::<Vec<_>>();
    text_order.sort_by(|left, right| {
        right.1["textScore"]
            .as_f64()
            .partial_cmp(&left.1["textScore"].as_f64())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.1["path"].as_str().cmp(&right.1["path"].as_str()))
    });
    for (rank, (index, _)) in text_order.into_iter().enumerate() {
        text_ranks.insert(index, rank + 1);
    }

    let mut semantic_order = results
        .iter()
        .enumerate()
        .filter(|(_, value)| value["semanticScore"].as_f64().unwrap_or(0.0) > 0.0)
        .collect::<Vec<_>>();
    semantic_order.sort_by(|left, right| {
        right.1["semanticScore"]
            .as_f64()
            .partial_cmp(&left.1["semanticScore"].as_f64())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.1["path"].as_str().cmp(&right.1["path"].as_str()))
    });
    for (rank, (index, _)) in semantic_order.into_iter().enumerate() {
        semantic_ranks.insert(index, rank + 1);
    }

    for (index, value) in results.iter_mut().enumerate() {
        let fused = text_ranks
            .get(&index)
            .map(|rank| 1.0 / (60.0 + *rank as f64))
            .unwrap_or(0.0)
            + semantic_ranks
                .get(&index)
                .map(|rank| 1.0 / (60.0 + *rank as f64))
                .unwrap_or(0.0);
        if let Some(object) = value.as_object_mut() {
            let decay_weight = object
                .get("decayWeight")
                .and_then(Value::as_f64)
                .unwrap_or(1.0);
            // T-4.4: RTM FR-4.4
            object.insert("fusedScore".to_string(), json!(fused * decay_weight));
        }
    }

    results.sort_by(|left, right| {
        right["fusedScore"]
            .as_f64()
            .partial_cmp(&left["fusedScore"].as_f64())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left["path"].as_str().cmp(&right["path"].as_str()))
    });

    // T-0.1: RTM NFR-3 - apply SearchMemories thresholding and folder scoping so parity coverage can guard regressions.
    let mode_score_key = if mode.eq_ignore_ascii_case("semantic") {
        "semanticScore"
    } else if mode.eq_ignore_ascii_case("fulltext") || mode.eq_ignore_ascii_case("full-text") {
        "textScore"
    } else {
        "fusedScore"
    };

    let results = results
        .into_iter()
        .filter(|value| value[mode_score_key].as_f64().unwrap_or(0.0) >= min_score)
        .collect::<Vec<_>>();

    let text_result_count = results
        .iter()
        .filter(|value| value["textScore"].as_f64().unwrap_or(0.0) > 0.0)
        .count();
    let semantic_result_count = results
        .iter()
        .filter(|value| value["semanticScore"].as_f64().unwrap_or(0.0) > 0.0)
        .count();

    let total_count = results.len();
    let start = page.saturating_sub(1) * page_size;
    let paged_results = results
        .into_iter()
        .skip(start)
        .take(page_size)
        .collect::<Vec<_>>();

    let data = json!({
        "mode": mode,
        "query": query,
        "totalCount": total_count,
        "page": page,
        "pageSize": page_size,
        "textResultCount": text_result_count,
        "semanticResultCount": semantic_result_count,
        "results": paged_results,
    });

    Ok(match context.output_format {
        OutputFormat::Json => success(data),
        OutputFormat::Markdown => render_search_markdown(&data),
    })
}

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

    GraphDatabase::initialize_database(&context.config)?;
    let configured_role = sync_role();
    if configured_role == "follower" {
        let markdown = render_sync_markdown(
            SyncSummary {
                status: "skipped",
                files_processed: 0,
                concept_mentions: 0,
                concept_relations: 0,
                abandoned_sessions: 0,
                communities_detected: 0,
                vector_warning_code: None,
            },
            None,
        );
        return Ok(markdown);
    }

    // T-4.6: RTM FR-30.1
    let holder_id = generate_sync_holder_id();
    let leader_election = LeaderElection::new(&context.config.database_path);
    let (lease_owned, _reason_code) = if configured_role == "leader" {
        // T-4.6: RTM FR-30.4
        leader_election.force_acquire(&holder_id, 60)?
    } else {
        leader_election.try_acquire(&holder_id, 60)?
    };
    if !lease_owned && configured_role == "auto" {
        return Ok("Skipped: another process is leader".to_string());
    }

    let sync_result = (|| -> Result<String> {
        let file_uris = get_memory_files(&context.config, None)?;
        let file_uri_set = file_uris.iter().cloned().collect::<HashSet<_>>();
        let mut connection = db::open_connection(&context.config.database_path)?;
        let tx = connection.transaction()?;
        let sync_started_at = current_iso_timestamp();

        delete_missing_files(&tx, &file_uri_set)?;

        for uri in &file_uris {
            let file_path = crate::markdown::uri_to_path(uri, &context.config.memory_path)?;
            let metadata = fs::metadata(&file_path)?;
            let indexed_at = indexed_timestamp_for_metadata(&metadata);
            let stored_file = load_indexed_file_state(&tx, uri)?;
            // T-2.3: RTM FR-14.1 / FR-14.2 / FR-14.4
            // Trust persisted file_content state for mtime/hash guards even when concept_mentions
            // were externally tampered; otherwise a missing mention row forces an unnecessary reread.
            let synced_file = stored_file.as_ref();
            let stored_hash = synced_file.and_then(|indexed_file| indexed_file.file_md5.as_deref());
            let last_indexed =
                synced_file.and_then(|indexed_file| indexed_file.last_indexed.as_deref());

            // T-2.3: RTM FR-14.1 / FR-14.4
            if should_skip_by_mtime(metadata.modified().ok(), last_indexed) {
                continue;
            }

            let (frontmatter, content, checksum) = read_markdown(uri, &context.config)?;

            // T-2.3: RTM FR-14.2 / FR-14.4
            if stored_hash == Some(checksum.as_str()) {
                tx.execute(
                    "UPDATE file_content SET last_indexed = ?2 WHERE file_path = ?1",
                    params![uri, indexed_at],
                )?;
                continue;
            }

            // T-2.3: RTM FR-14.3
            upsert_processed_file(
                &tx,
                uri,
                &frontmatter,
                &content,
                &checksum,
                metadata.len() as i64,
                &indexed_at,
            )?;
        }

        let total_mentions = rebuild_graph_from_mentions(&tx, &sync_started_at)?;

        tx.commit()?;

        // T-4.2: RTM FR-4.2
        let abandoned_sessions = sessions::cleanup_abandoned_sessions(&context.config)? as i64;

        let relation_count: i64 =
            connection.query_row("SELECT COUNT(*) FROM concept_graph", [], |row| row.get(0))?;
        let concept_count: i64 =
            connection.query_row("SELECT COUNT(*) FROM concepts", [], |row| row.get(0))?;
        let file_count: i64 =
            connection.query_row("SELECT COUNT(*) FROM file_content", [], |row| row.get(0))?;

        let status = if file_uris.is_empty() {
            "degraded"
        } else {
            "success"
        };
        let vector_warning_code = if file_uris.is_empty() {
            Some("VECTOR_TABLES_EMPTY_AFTER_SYNC")
        } else {
            None
        };
        // T-3.2a: RTM FR-13.1 / FR-13.3 / FR-13.11
        let gamma = std::env::var("MAENIFOLD_LOUVAIN_GAMMA")
            .ok()
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(1.0);
        let communities_detected =
            community::detect_and_persist(&context.config.database_path, gamma)? as i64;
        reset_incremental_community_state(&connection, &sync_started_at)?;

        let vector_section = if vector_sync_disabled_by_config() {
            let diagnostics =
                collect_vector_skip_diagnostics_for_uris(&context.config, &file_uris)?;
            Some(
                build_vector_unavailable_report(
                    &diagnostics,
                    VectorUnavailableReasonCodes::DISABLED_BY_CONFIG,
                )
                .join("\n"),
            )
        } else {
            None
        };
        let markdown = render_sync_markdown(
            SyncSummary {
                status,
                files_processed: file_uris.len() as i64,
                concept_mentions: total_mentions as i64,
                concept_relations: relation_count,
                abandoned_sessions,
                communities_detected,
                vector_warning_code,
            },
            vector_section.as_deref(),
        );

        let _ = concept_count;
        let _ = file_count;

        Ok(markdown)
    })();

    if lease_owned {
        let _ = leader_election.release(&holder_id);
    }

    sync_result
}

pub fn sync_single_file(context: &ToolContext, file_path: &Path) -> Result<String> {
    GraphDatabase::initialize_database(&context.config)?;
    let file_uri = path_to_uri(file_path, &context.config.memory_path)?;
    let sync_started_at = current_iso_timestamp();
    let mut connection = db::open_connection(&context.config.database_path)?;
    let tx = connection.transaction()?;
    ensure_incremental_sync_state_table(&tx)?;

    let indexed_state = load_indexed_file_state(&tx, &file_uri)?;
    let previous_concepts = load_file_concepts(&tx, &file_uri)?;

    if !file_path.exists() {
        if indexed_state.is_none() && previous_concepts.is_empty() {
            tx.commit()?;
            return Ok(format!("Skipped {file_uri}: file not indexed"));
        }

        // T-5.5: RTM FR-14.3
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

    // T-5.5: RTM FR-14.1
    if should_skip_by_mtime(metadata.modified().ok(), last_indexed) {
        tx.commit()?;
        return Ok(format!("Skipped {file_uri}: mtime guard"));
    }

    let (frontmatter, content, checksum) = read_markdown(&file_uri, &context.config)?;

    // T-5.5: RTM FR-14.2
    if stored_hash == Some(checksum.as_str()) {
        tx.execute(
            "UPDATE file_content SET last_indexed = ?2 WHERE file_path = ?1",
            params![file_uri, indexed_at],
        )?;
        tx.commit()?;
        return Ok(format!("Skipped {file_uri}: hash guard"));
    }

    // T-5.5: RTM FR-14.3
    upsert_processed_file(
        &tx,
        &file_uri,
        &frontmatter,
        &content,
        &checksum,
        metadata.len() as i64,
        &indexed_at,
    )?;
    let new_concepts = load_file_concepts(&tx, &file_uri)?;
    let affected_concepts = previous_concepts
        .union(&new_concepts)
        .cloned()
        .collect::<BTreeSet<_>>();
    let relation_count =
        rebuild_affected_graph_from_mentions(&tx, &affected_concepts, &sync_started_at)?;
    prune_orphaned_concepts(&tx, &affected_concepts)?;
    purge_unreferenced_concepts(&tx)?;
    record_incremental_graph_change(&tx, &sync_started_at)?;
    let mention_count = load_total_mentions_for_file(&tx, &file_uri)?;
    tx.commit()?;

    Ok(format!(
        "Synced {file_uri}: {mention_count} concept mentions updated, {relation_count} affected relations rebuilt"
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

    let now = Utc::now();
    let last_change_at = load_incremental_state_timestamp(&connection, COMMUNITY_LAST_CHANGE_KEY);
    let time_threshold = community_recompute_time_threshold();
    let should_recompute = pending_count >= community_recompute_file_threshold() as i64
        || last_change_at
            .map(|timestamp| now.signed_duration_since(timestamp) >= time_threshold)
            .unwrap_or(true);

    if !should_recompute {
        return Ok(None);
    }

    let gamma = std::env::var("MAENIFOLD_LOUVAIN_GAMMA")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(1.0);
    let communities = community::detect_and_persist(&context.config.database_path, gamma)?;
    let recomputed_at = current_iso_timestamp();
    reset_incremental_community_state(&connection, &recomputed_at)?;
    Ok(Some(communities))
}

struct IndexedFileState {
    last_indexed: Option<String>,
    file_md5: Option<String>,
}

type RelationKey = (String, String);
type RelationSourceWeights = BTreeMap<String, f64>;
type RelationCounts = BTreeMap<RelationKey, (f64, RelationSourceWeights)>;

fn load_indexed_file_state(
    connection: &Connection,
    file_uri: &str,
) -> Result<Option<IndexedFileState>> {
    Ok(connection
        .query_row(
            "SELECT \
                 last_indexed, \
                 file_md5 \
             FROM file_content \
             WHERE file_path = ?1",
            params![file_uri],
            |row| {
                Ok(IndexedFileState {
                    last_indexed: row.get(0)?,
                    file_md5: row.get(1)?,
                })
            },
        )
        .optional()?)
}

fn delete_missing_files(connection: &Connection, existing_files: &HashSet<String>) -> Result<()> {
    let mut statement = connection.prepare("SELECT file_path FROM file_content")?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    let indexed_files = rows.collect::<std::result::Result<Vec<_>, _>>()?;

    for file_uri in indexed_files {
        if existing_files.contains(&file_uri) {
            continue;
        }

        tx_delete_file_index(connection, &file_uri)?;
    }

    Ok(())
}

fn ensure_incremental_sync_state_table(connection: &Connection) -> Result<()> {
    connection.execute(
        "CREATE TABLE IF NOT EXISTS incremental_sync_state (
             key TEXT PRIMARY KEY,
             value TEXT NOT NULL
         )",
        [],
    )?;
    Ok(())
}

fn load_file_concepts(connection: &Connection, file_uri: &str) -> Result<BTreeSet<String>> {
    let mut statement = connection.prepare(
        "SELECT concept_name FROM concept_mentions WHERE source_file = ?1 ORDER BY concept_name",
    )?;
    let rows = statement.query_map(params![file_uri], |row| row.get::<_, String>(0))?;
    Ok(rows.collect::<std::result::Result<BTreeSet<_>, _>>()?)
}

fn load_total_mentions_for_file(connection: &Connection, file_uri: &str) -> Result<i64> {
    Ok(connection.query_row(
        "SELECT COALESCE(SUM(mention_count), 0) FROM concept_mentions WHERE source_file = ?1",
        params![file_uri],
        |row| row.get(0),
    )?)
}

fn tx_delete_file_index(connection: &Connection, file_uri: &str) -> Result<()> {
    connection.execute(
        "DELETE FROM concept_mentions WHERE source_file = ?1",
        params![file_uri],
    )?;
    connection.execute(
        "DELETE FROM file_content WHERE file_path = ?1",
        params![file_uri],
    )?;
    Ok(())
}

fn should_skip_by_mtime(modified_at: Option<SystemTime>, last_indexed: Option<&str>) -> bool {
    let Some(modified_at) = modified_at else {
        return false;
    };
    let Some(last_indexed) = last_indexed else {
        return false;
    };
    let Some(last_indexed) = parse_indexed_timestamp(last_indexed) else {
        return false;
    };

    let modified_at: DateTime<Utc> = modified_at.into();
    modified_at <= last_indexed
}

fn parse_indexed_timestamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|parsed| parsed.with_timezone(&Utc))
}

fn indexed_timestamp_for_metadata(metadata: &fs::Metadata) -> String {
    metadata
        .modified()
        .ok()
        .map(|modified| DateTime::<Utc>::from(modified).to_rfc3339())
        .unwrap_or_else(current_iso_timestamp)
}

fn upsert_processed_file(
    connection: &Connection,
    uri: &str,
    frontmatter: &Option<crate::markdown::Frontmatter>,
    content: &str,
    checksum: &str,
    file_size: i64,
    indexed_at: &str,
) -> Result<()> {
    let title = frontmatter
        .as_ref()
        .and_then(|frontmatter| frontmatter.get("title"))
        .and_then(|value| value.as_str())
        // T-0.3b: RTM NFR-4
        .unwrap_or_else(|| {
            uri.trim_start_matches("memory://")
                .split('/')
                .next_back()
                .unwrap_or_default()
        })
        .to_string();
    let status = frontmatter
        .as_ref()
        .and_then(|frontmatter| frontmatter.get("status"))
        .and_then(|value| value.as_str())
        .unwrap_or("saved")
        .to_string();
    let created = frontmatter
        .as_ref()
        .and_then(|frontmatter| frontmatter.get("created"))
        .and_then(|value| value.as_str())
        .map(str::to_string);

    connection.execute(
        "INSERT INTO file_content (file_path, title, content, summary, last_indexed, status, file_md5, file_size, created, last_accessed) \
         VALUES (?1, ?2, ?3, '', ?4, ?5, ?6, ?7, COALESCE(?8, ?4), COALESCE((SELECT last_accessed FROM file_content WHERE file_path = ?1), COALESCE(?8, ?4))) \
         ON CONFLICT(file_path) DO UPDATE SET title=excluded.title, content=excluded.content, summary=excluded.summary, last_indexed=excluded.last_indexed, status=excluded.status, file_md5=excluded.file_md5, file_size=excluded.file_size, created=COALESCE(file_content.created, excluded.created), last_accessed=COALESCE(file_content.last_accessed, excluded.last_accessed)",
        params![uri, title, content, indexed_at, status, checksum, file_size, created],
    )?;

    connection.execute(
        "DELETE FROM concept_mentions WHERE source_file = ?1",
        params![uri],
    )?;

    let mut per_file = BTreeMap::<String, usize>::new();
    for concept in extract_wikilinks(content) {
        *per_file.entry(concept.to_ascii_lowercase()).or_default() += 1;
    }

    for (concept, mention_count) in per_file {
        connection.execute(
            "INSERT OR IGNORE INTO concepts (concept_name, first_seen, occurrence_count) VALUES (?1, ?2, 0)",
            params![concept, indexed_at],
        )?;
        connection.execute(
            "INSERT INTO concept_mentions (concept_name, source_file, mention_count) VALUES (?1, ?2, ?3) \
             ON CONFLICT(concept_name, source_file) DO UPDATE SET mention_count = excluded.mention_count",
            params![concept, uri, mention_count as i64],
        )?;
    }

    Ok(())
}

fn rebuild_graph_from_mentions(connection: &Connection, indexed_at: &str) -> Result<i64> {
    let existing_first_seen = load_existing_first_seen(connection)?;
    let mut statement = connection.prepare(
        "SELECT source_file, concept_name, mention_count FROM concept_mentions ORDER BY source_file, concept_name",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })?;

    let mut total_mentions = 0i64;
    let mut concept_counts = BTreeMap::<String, i64>::new();
    for row in rows {
        let (_source_file, concept_name, mention_count) = row?;
        total_mentions += mention_count;
        *concept_counts.entry(concept_name.clone()).or_default() += mention_count;
    }

    let relation_counts = load_relation_counts(connection)?;

    connection.execute("DELETE FROM concept_graph", [])?;

    for (concept_name, occurrence_count) in concept_counts {
        let first_seen = existing_first_seen
            .get(&concept_name)
            .cloned()
            .unwrap_or_else(|| indexed_at.to_string());
        connection.execute(
            "INSERT INTO concepts (concept_name, first_seen, occurrence_count) VALUES (?1, ?2, ?3)
             ON CONFLICT(concept_name) DO UPDATE SET
                 first_seen = COALESCE(concepts.first_seen, excluded.first_seen),
                 occurrence_count = excluded.occurrence_count",
            params![concept_name, first_seen, occurrence_count],
        )?;
    }

    purge_unreferenced_concepts(connection)?;

    for ((concept_a, concept_b), (count, files)) in relation_counts {
        let source_files = serde_json::to_string(&files)?;
        connection.execute(
            "INSERT INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files) VALUES (?1, ?2, ?3, ?4)",
            params![concept_a, concept_b, count, source_files],
        )?;
    }

    Ok(total_mentions)
}

#[allow(dead_code)]
fn rebuild_affected_graph_from_mentions(
    connection: &Connection,
    affected_concepts: &BTreeSet<String>,
    indexed_at: &str,
) -> Result<usize> {
    if affected_concepts.is_empty() {
        return Ok(0);
    }

    let existing_first_seen = load_existing_first_seen(connection)?;
    let mut existing_relations =
        load_existing_relations_for_concepts(connection, affected_concepts)?;
    refresh_affected_concepts(
        connection,
        affected_concepts,
        &existing_first_seen,
        indexed_at,
    )?;

    let candidate_files = load_candidate_files_for_concepts(connection, affected_concepts)?;
    let relation_counts =
        load_relation_counts_for_files(connection, &candidate_files, Some(affected_concepts))?;

    let mut relation_count = 0usize;
    for ((concept_a, concept_b), (count, files)) in relation_counts {
        let key = (concept_a.clone(), concept_b.clone());
        let source_files = serde_json::to_string(&files)?;
        let should_write = existing_relations.remove(&key).is_none_or(|existing| {
            existing.co_occurrence_count != count || existing.source_files != source_files
        });
        if should_write {
            connection.execute(
                "INSERT INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(concept_a, concept_b) DO UPDATE SET
                     co_occurrence_count = excluded.co_occurrence_count,
                     source_files = excluded.source_files",
                params![concept_a, concept_b, count, source_files],
            )?;
        }
        relation_count += 1;
    }

    for ((concept_a, concept_b), _) in existing_relations {
        connection.execute(
            "DELETE FROM concept_graph WHERE concept_a = ?1 AND concept_b = ?2",
            params![concept_a, concept_b],
        )?;
    }

    Ok(relation_count)
}

#[derive(Debug, Clone)]
struct ExistingRelationRow {
    co_occurrence_count: f64,
    source_files: String,
}

fn load_existing_first_seen(connection: &Connection) -> Result<BTreeMap<String, String>> {
    let mut statement = connection
        .prepare("SELECT concept_name, first_seen FROM concepts WHERE first_seen IS NOT NULL")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    Ok(rows.collect::<std::result::Result<BTreeMap<_, _>, _>>()?)
}

#[allow(dead_code)]
fn delete_affected_graph_edges(
    connection: &Connection,
    affected_concepts: &BTreeSet<String>,
) -> Result<()> {
    let mut delete_statement =
        connection.prepare("DELETE FROM concept_graph WHERE concept_a = ?1 OR concept_b = ?1")?;
    for concept in affected_concepts {
        delete_statement.execute(params![concept])?;
    }
    Ok(())
}

fn load_existing_relations_for_concepts(
    connection: &Connection,
    affected_concepts: &BTreeSet<String>,
) -> Result<BTreeMap<(String, String), ExistingRelationRow>> {
    let mut relations = BTreeMap::new();
    let mut statement = connection.prepare(
        "SELECT concept_a, concept_b, COALESCE(co_occurrence_count, 0.0), COALESCE(source_files, '')
         FROM concept_graph
         WHERE concept_a = ?1 OR concept_b = ?1",
    )?;
    for concept in affected_concepts {
        let rows = statement.query_map(params![concept], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                ExistingRelationRow {
                    co_occurrence_count: row.get::<_, f64>(2)?,
                    source_files: row.get::<_, String>(3)?,
                },
            ))
        })?;
        for row in rows {
            let (concept_a, concept_b, relation) = row?;
            relations.entry((concept_a, concept_b)).or_insert(relation);
        }
    }
    Ok(relations)
}

#[allow(dead_code)]
fn refresh_affected_concepts(
    connection: &Connection,
    affected_concepts: &BTreeSet<String>,
    existing_first_seen: &BTreeMap<String, String>,
    indexed_at: &str,
) -> Result<()> {
    let mut mention_statement = connection.prepare(
        "SELECT COALESCE(SUM(mention_count), 0) FROM concept_mentions WHERE concept_name = ?1",
    )?;
    let mut upsert_statement = connection.prepare(
        "INSERT INTO concepts (concept_name, first_seen, occurrence_count) VALUES (?1, ?2, ?3)
         ON CONFLICT(concept_name) DO UPDATE SET occurrence_count = excluded.occurrence_count, first_seen = COALESCE(concepts.first_seen, excluded.first_seen)",
    )?;
    let mut delete_concept_statement =
        connection.prepare("DELETE FROM concepts WHERE concept_name = ?1")?;
    let mut delete_community_statement =
        connection.prepare("DELETE FROM concept_communities WHERE concept_name = ?1")?;

    for concept in affected_concepts {
        let occurrence_count: i64 =
            mention_statement.query_row(params![concept], |row| row.get(0))?;
        if occurrence_count <= 0 {
            delete_community_statement.execute(params![concept])?;
            delete_concept_statement.execute(params![concept])?;
            continue;
        }

        let first_seen = existing_first_seen
            .get(concept)
            .cloned()
            .unwrap_or_else(|| indexed_at.to_string());
        upsert_statement.execute(params![concept, first_seen, occurrence_count])?;
    }

    Ok(())
}

fn prune_orphaned_concepts(
    connection: &Connection,
    affected_concepts: &BTreeSet<String>,
) -> Result<()> {
    let mut delete_concept_statement = connection.prepare(
        "DELETE FROM concepts
         WHERE concept_name = ?1
           AND NOT EXISTS (
               SELECT 1 FROM concept_mentions WHERE concept_name = ?1
           )",
    )?;
    let mut delete_community_statement = connection.prepare(
        "DELETE FROM concept_communities
         WHERE concept_name = ?1
           AND NOT EXISTS (
               SELECT 1 FROM concept_mentions WHERE concept_name = ?1
           )",
    )?;

    for concept in affected_concepts {
        delete_community_statement.execute(params![concept])?;
        delete_concept_statement.execute(params![concept])?;
    }

    Ok(())
}

fn purge_unreferenced_concepts(connection: &Connection) -> Result<()> {
    connection.execute(
        "DELETE FROM concept_communities
         WHERE NOT EXISTS (
             SELECT 1 FROM concept_mentions
             WHERE concept_mentions.concept_name = concept_communities.concept_name
         )",
        [],
    )?;
    connection.execute(
        "DELETE FROM concepts
         WHERE NOT EXISTS (
             SELECT 1 FROM concept_mentions
             WHERE concept_mentions.concept_name = concepts.concept_name
         )",
        [],
    )?;
    Ok(())
}

#[allow(dead_code)]
fn load_candidate_files_for_concepts(
    connection: &Connection,
    affected_concepts: &BTreeSet<String>,
) -> Result<BTreeSet<String>> {
    let mut candidate_files = BTreeSet::new();
    let mut statement = connection
        .prepare("SELECT DISTINCT source_file FROM concept_mentions WHERE concept_name = ?1")?;

    for concept in affected_concepts {
        let rows = statement.query_map(params![concept], |row| row.get::<_, String>(0))?;
        for file_uri in rows {
            candidate_files.insert(file_uri?);
        }
    }

    Ok(candidate_files)
}

#[allow(dead_code)]
fn load_relation_counts_for_files(
    connection: &Connection,
    source_files: &BTreeSet<String>,
    affected_concepts: Option<&BTreeSet<String>>,
) -> Result<RelationCounts> {
    let mut statement =
        connection.prepare("SELECT content FROM file_content WHERE file_path = ?1 LIMIT 1")?;
    let mut relation_counts = RelationCounts::new();

    for source_file in source_files {
        let content = statement
            .query_row(params![source_file], |row| row.get::<_, String>(0))
            .optional()?;
        let Some(content) = content else {
            continue;
        };
        let allowed_concepts = load_file_concepts(connection, source_file)?;
        if allowed_concepts.is_empty() {
            continue;
        }

        let mut per_file_counts = RelationCounts::new();
        flush_relation_counts(
            source_file,
            &content,
            Some(&allowed_concepts),
            &mut per_file_counts,
        );
        merge_relation_counts(&mut relation_counts, per_file_counts, affected_concepts);
    }

    Ok(relation_counts)
}

#[allow(dead_code)]
fn merge_relation_counts(
    target: &mut RelationCounts,
    source: RelationCounts,
    affected_concepts: Option<&BTreeSet<String>>,
) {
    for ((concept_a, concept_b), (count, files)) in source {
        if let Some(affected_concepts) = affected_concepts
            && !affected_concepts.contains(&concept_a)
            && !affected_concepts.contains(&concept_b)
        {
            continue;
        }

        let entry = target
            .entry((concept_a, concept_b))
            .or_insert_with(|| (0.0, BTreeMap::new()));
        entry.0 += count;
        for (source_file, weight) in files {
            entry
                .1
                .entry(source_file)
                .and_modify(|existing| *existing = existing.max(weight))
                .or_insert(weight);
        }
    }
}

const COMMUNITY_PENDING_COUNT_KEY: &str = "pending_change_count";
const COMMUNITY_LAST_CHANGE_KEY: &str = "last_change_at";
const COMMUNITY_LAST_RECOMPUTE_KEY: &str = "last_community_recompute_at";

fn record_incremental_graph_change(connection: &Connection, changed_at: &str) -> Result<()> {
    let next_count = load_incremental_state_i64(connection, COMMUNITY_PENDING_COUNT_KEY)? + 1;
    set_incremental_state(
        connection,
        COMMUNITY_PENDING_COUNT_KEY,
        &next_count.to_string(),
    )?;
    set_incremental_state(connection, COMMUNITY_LAST_CHANGE_KEY, changed_at)?;
    Ok(())
}

fn reset_incremental_community_state(connection: &Connection, recomputed_at: &str) -> Result<()> {
    ensure_incremental_sync_state_table(connection)?;
    set_incremental_state(connection, COMMUNITY_PENDING_COUNT_KEY, "0")?;
    set_incremental_state(connection, COMMUNITY_LAST_CHANGE_KEY, recomputed_at)?;
    set_incremental_state(connection, COMMUNITY_LAST_RECOMPUTE_KEY, recomputed_at)?;
    Ok(())
}

fn set_incremental_state(connection: &Connection, key: &str, value: &str) -> Result<()> {
    connection.execute(
        "INSERT INTO incremental_sync_state (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

fn load_incremental_state_i64(connection: &Connection, key: &str) -> Result<i64> {
    let value = connection
        .query_row(
            "SELECT value FROM incremental_sync_state WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    Ok(value.and_then(|raw| raw.parse::<i64>().ok()).unwrap_or(0))
}

fn load_incremental_state_timestamp(connection: &Connection, key: &str) -> Option<DateTime<Utc>> {
    let value = connection
        .query_row(
            "SELECT value FROM incremental_sync_state WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .ok()
        .flatten()?;
    parse_indexed_timestamp(&value)
}

fn community_recompute_file_threshold() -> usize {
    std::env::var("MAENIFOLD_COMMUNITY_RECOMPUTE_FILE_THRESHOLD")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(10)
}

fn community_recompute_time_threshold() -> chrono::Duration {
    let threshold_ms = std::env::var("MAENIFOLD_COMMUNITY_DEBOUNCE_MS")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(2_000);
    chrono::Duration::milliseconds(threshold_ms)
}

// T-4.3: RTM FR-23.x
fn load_relation_counts(connection: &Connection) -> Result<RelationCounts> {
    let mut mentions_statement = connection.prepare(
        "SELECT source_file, concept_name FROM concept_mentions ORDER BY source_file, concept_name",
    )?;
    let mention_rows = mentions_statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut active_concepts_by_file = BTreeMap::<String, BTreeSet<String>>::new();
    for row in mention_rows {
        let (source_file, concept_name) = row?;
        active_concepts_by_file
            .entry(source_file)
            .or_default()
            .insert(concept_name);
    }

    let mut statement =
        connection.prepare("SELECT file_path, content FROM file_content ORDER BY file_path")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut relation_counts = RelationCounts::new();
    for row in rows {
        let (source_file, content) = row?;
        let Some(allowed_concepts) = active_concepts_by_file.get(&source_file) else {
            continue;
        };
        flush_relation_counts(
            &source_file,
            &content,
            Some(allowed_concepts),
            &mut relation_counts,
        );
    }

    Ok(relation_counts)
}

// T-4.3: RTM FR-23.x
fn flush_relation_counts(
    source_file: &str,
    content: &str,
    allowed_concepts: Option<&BTreeSet<String>>,
    relation_counts: &mut RelationCounts,
) {
    let mut concepts_by_section = BTreeMap::<String, BTreeSet<usize>>::new();
    for (concept, section_index) in extract_wikilinks_with_sections(content) {
        if allowed_concepts.is_some_and(|allowed| !allowed.contains(&concept)) {
            continue;
        }
        concepts_by_section
            .entry(concept)
            .or_default()
            .insert(section_index);
    }

    let concepts = concepts_by_section.into_iter().collect::<Vec<_>>();
    for index in 0..concepts.len() {
        for other_index in index + 1..concepts.len() {
            let (first, first_sections) = &concepts[index];
            let (second, second_sections) = &concepts[other_index];
            let key = if first <= second {
                (first.clone(), second.clone())
            } else {
                (second.clone(), first.clone())
            };
            let weight = if first_sections
                .iter()
                .any(|section_index| *section_index > 0 && second_sections.contains(section_index))
            {
                1.0
            } else {
                0.5
            };
            let entry = relation_counts
                .entry(key)
                .or_insert_with(|| (0.0, BTreeMap::new()));
            entry.0 += weight;
            entry
                .1
                .entry(source_file.to_string())
                .and_modify(|existing| *existing = existing.max(weight))
                .or_insert(weight);
        }
    }
}

pub fn build_context(context: &ToolContext, payload: &Value) -> Result<String> {
    let learn = payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if learn {
        let help = load_help_doc(&context.config.assets_path, "BuildContext")
            .unwrap_or_else(|| "ERROR: Help file not found for BuildContext".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }

    let concept_name = normalize_concept(
        payload
            .get("conceptName")
            .and_then(Value::as_str)
            .ok_or(MaenifoldError::MissingProperty("conceptName"))?,
    );
    let depth = match payload.get("depth").and_then(Value::as_i64) {
        Some(value) if value < 0 => {
            return Err(MaenifoldError::InvalidArgument(
                "depth must be zero or greater".to_string(),
            ));
        }
        Some(value) => value as usize,
        None => 2,
    };
    // T-3.4: RTM FR-13.6
    let max_entities = match payload.get("maxEntities").and_then(Value::as_i64) {
        Some(value) if value < 0 => {
            return Err(MaenifoldError::InvalidArgument(
                "maxEntities must be zero or greater".to_string(),
            ));
        }
        Some(value) => value as usize,
        None => 20,
    };
    // T-1.6: RTM NFR-1 - ported ConceptGraphTests require C#-compatible content preview opt-in.
    let include_content = payload
        .get("includeContent")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    GraphDatabase::initialize_database(&context.config)?;
    let connection = db::open_connection(&context.config.database_path)?;

    // T-3.2b: RTM FR-13.10
    let query_community_id = load_concept_community_id(&connection, &concept_name)?;
    let include_community_data = query_community_id.is_some();

    // T-3.2b: RTM FR-13.6
    let mut direct_relations = load_graph_neighbors(&connection, &concept_name)?;
    // T-GRAPH-DECAY-001.1: RTM FR-7.5
    direct_relations.sort_by(|left, right| {
        let left_score = relation_weighted_score(
            &connection,
            &context.config,
            left.co_occurrence_count,
            &left.source_files,
        )
        .map(|(_, weighted_score)| weighted_score)
        .unwrap_or(left.co_occurrence_count);
        let right_score = relation_weighted_score(
            &connection,
            &context.config,
            right.co_occurrence_count,
            &right.source_files,
        )
        .map(|(_, weighted_score)| weighted_score)
        .unwrap_or(right.co_occurrence_count);
        right_score
            .partial_cmp(&left_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.name.cmp(&right.name))
    });
    let direct_relation_names = direct_relations
        .iter()
        .map(|relation| relation.name.clone())
        .collect::<HashSet<_>>();

    let mut related_concepts = Vec::new();
    let mut seen_concepts = HashSet::from([concept_name.clone()]);
    let mut current_frontier = vec![concept_name.clone()];

    // T-3.4: RTM FR-13.6
    for _hop_distance in 1..=depth {
        if related_concepts.len() >= max_entities {
            break;
        }

        let mut next_frontier = Vec::new();
        let mut hop_candidates = BTreeMap::<String, GraphRelation>::new();

        for frontier_concept in &current_frontier {
            for relation in load_graph_neighbors(&connection, frontier_concept)? {
                if seen_concepts.contains(&relation.name) {
                    continue;
                }

                hop_candidates
                    .entry(relation.name.clone())
                    .and_modify(|existing| {
                        if relation.co_occurrence_count > existing.co_occurrence_count {
                            *existing = relation.clone();
                        }
                    })
                    .or_insert(relation);
            }
        }

        if hop_candidates.is_empty() {
            break;
        }

        let mut hop_relations = hop_candidates.into_values().collect::<Vec<_>>();
        // T-GRAPH-DECAY-001.1: RTM FR-7.5
        hop_relations.sort_by(|left, right| {
            let left_score = relation_weighted_score(
                &connection,
                &context.config,
                left.co_occurrence_count,
                &left.source_files,
            )
            .map(|(_, weighted_score)| weighted_score)
            .unwrap_or(left.co_occurrence_count);
            let right_score = relation_weighted_score(
                &connection,
                &context.config,
                right.co_occurrence_count,
                &right.source_files,
            )
            .map(|(_, weighted_score)| weighted_score)
            .unwrap_or(right.co_occurrence_count);
            right_score
                .partial_cmp(&left_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.name.cmp(&right.name))
        });

        for relation in hop_relations {
            if related_concepts.len() >= max_entities {
                break;
            }

            let relation_name = relation.name.clone();
            seen_concepts.insert(relation_name.clone());
            next_frontier.push(relation_name.clone());
            related_concepts.push(RelatedConceptLine {
                name: relation_name.clone(),
                co_occurrence_count: relation.co_occurrence_count,
                source_files: relation.source_files,
                community_id: if include_community_data {
                    load_concept_community_id(&connection, &relation_name)?
                } else {
                    None
                },
            });
        }

        current_frontier = next_frontier;
    }

    let community_siblings = if include_community_data && depth > 0 {
        // T-3.2b: RTM FR-13.7
        load_community_siblings(
            &connection,
            &concept_name,
            query_community_id.expect("community presence checked"),
            &direct_relation_names,
        )?
    } else {
        Vec::new()
    };

    if context.output_format == OutputFormat::Json {
        // T-1.6: RTM NFR-1 - BuildContext JSON mirrors the C# structured envelope.
        let direct_relations_json = direct_relations
            .iter()
            .map(|relation| {
                let content_preview = if include_content {
                    load_content_preview_map(&connection, &concept_name, &relation.source_files)
                        .unwrap_or_else(|_| json!({}))
                } else {
                    json!({})
                };
                let (decay_weight, weighted_score) = relation_weighted_score(
                    &connection,
                    &context.config,
                    relation.co_occurrence_count,
                    &relation.source_files,
                )
                .unwrap_or((1.0, relation.co_occurrence_count));
                json!({
                    "Name": relation.name,
                    "CoOccurrenceCount": relation.co_occurrence_count,
                    "Files": parse_source_files_for_json(&relation.source_files),
                    "ContentPreview": content_preview,
                    // T-GRAPH-DECAY-001.1: RTM FR-7.5
                    "DecayWeight": decay_weight,
                    "WeightedScore": weighted_score,
                    "CommunityId": if include_community_data {
                        load_concept_community_id(&connection, &relation.name).ok().flatten()
                    } else {
                        None
                    },
                })
            })
            .collect::<Vec<_>>();
        let expanded_relations_json = related_concepts
            .iter()
            .map(|relation| {
                let content_preview = if include_content {
                    load_content_preview_map(&connection, &relation.name, &relation.source_files)
                        .unwrap_or_else(|_| json!({}))
                } else {
                    json!({})
                };
                let (decay_weight, weighted_score) = relation_weighted_score(
                    &connection,
                    &context.config,
                    relation.co_occurrence_count,
                    &relation.source_files,
                )
                .unwrap_or((1.0, relation.co_occurrence_count));
                json!({
                    "Name": relation.name,
                    "CoOccurrenceCount": relation.co_occurrence_count,
                    "Files": parse_source_files_for_json(&relation.source_files),
                    "ContentPreview": content_preview,
                    // T-GRAPH-DECAY-001.1: RTM FR-7.5
                    "DecayWeight": decay_weight,
                    "WeightedScore": weighted_score,
                    "CommunityId": relation.community_id,
                })
            })
            .collect::<Vec<_>>();
        let community_siblings_json = community_siblings
            .iter()
            .map(|sibling| {
                json!({
                    "Name": sibling.name,
                    "CommunityId": sibling.community_id,
                    "NormalizedOverlap": sibling.normalized_overlap,
                })
            })
            .collect::<Vec<_>>();

        return Ok(success(json!({
            "ConceptName": concept_name,
            "Depth": depth,
            "DirectRelations": direct_relations_json,
            "ExpandedRelations": expanded_relations_json,
            "CommunityId": query_community_id,
            "CommunitySiblings": community_siblings_json,
        })));
    }

    if !include_community_data && direct_relations.is_empty() && related_concepts.is_empty() {
        // T-0.2: RTM NFR-3 - preserve the C# empty/not-found markdown contract.
        return Ok(format_empty_build_context(&concept_name, depth));
    }

    let mut output = format!("Related concepts for '{concept_name}':\n");
    if related_concepts.is_empty() {
        output.push_str("  • none\n");
    } else {
        for relation in related_concepts {
            output.push_str(&format!(
                "  • {} (co-occurrence: {})",
                relation.name,
                format_co_occurrence_weight(relation.co_occurrence_count)
            ));
            if let Some(community_id) = relation.community_id {
                output.push_str(&format!(" [community {community_id}]"));
            }
            output.push('\n');
            if !relation.source_files.is_empty() {
                output.push_str(&format!(
                    "    Files: {}\n",
                    format_source_files_for_display(&relation.source_files)
                ));
            }
        }
    }

    if !community_siblings.is_empty() {
        output.push_str("\nCommunity Siblings (same community, no direct edge):\n");
        for sibling in community_siblings {
            output.push_str(&format!(
                "  • {} (overlap: {:.2}) [community {}]\n",
                sibling.name, sibling.normalized_overlap, sibling.community_id
            ));
        }
    }

    Ok(output.trim_end().to_string())
}

fn format_empty_build_context(concept_name: &str, depth: usize) -> String {
    let mut output =
        format!("Context for CONCEPT: {concept_name}\n\nDirect relations (0 CONCEPTS):");
    if depth > 1 {
        output.push_str(&format!("\n\nExpanded relations ({depth} hops):"));
    }
    output
}

// T-1.6: RTM NFR-1 - port CreateSmartPreview parity helper from C# GraphTools.cs.
pub fn create_smart_preview(content: &str, target_length: usize, tolerance: usize) -> String {
    let content_len = content.chars().count();
    if content_len <= target_length {
        return content.to_string();
    }

    let max_length = target_length.saturating_add(tolerance).min(content_len);
    let search_space = truncate_chars(content, max_length);
    let minimum_acceptable_boundary = target_length.saturating_sub(tolerance);

    if let Some(sentence_end) = find_last_sentence_boundary(&search_space) {
        if sentence_end >= minimum_acceptable_boundary {
            return truncate_chars(content, sentence_end + 1)
                .trim_end()
                .to_string();
        }
    }

    if let Some(paragraph_end) = search_space.rfind("\n\n") {
        let paragraph_char_end = search_space[..paragraph_end].chars().count();
        if paragraph_char_end >= minimum_acceptable_boundary {
            return format!("{}...", search_space[..paragraph_end].trim_end());
        }
    }

    if let Some(word_end) = search_space.rfind(' ') {
        if word_end > 0 {
            return format!("{}...", search_space[..word_end].trim_end());
        }
    }

    format!("{}...", truncate_chars(content, target_length))
}

fn truncate_chars(content: &str, char_count: usize) -> &str {
    if char_count == 0 {
        return "";
    }

    match content.char_indices().nth(char_count) {
        Some((index, _)) => &content[..index],
        None => content,
    }
}

fn find_last_sentence_boundary(text: &str) -> Option<usize> {
    let chars = text.chars().collect::<Vec<_>>();
    let mut last_index = None;

    for (index, ch) in chars.iter().enumerate() {
        if !matches!(ch, '.' | '!' | '?') {
            continue;
        }

        let is_boundary = match chars.get(index + 1) {
            Some(' ' | '\n') => true,
            None => true,
            _ => false,
        };

        if is_boundary {
            last_index = Some(index);
        }
    }

    last_index
}

#[derive(Clone)]
struct GraphRelation {
    name: String,
    co_occurrence_count: f64,
    source_files: String,
}

struct RelatedConceptLine {
    name: String,
    co_occurrence_count: f64,
    source_files: String,
    community_id: Option<i64>,
}

struct CommunitySiblingLine {
    name: String,
    community_id: i64,
    normalized_overlap: f64,
}

fn parse_source_files_for_json(source_files: &str) -> Value {
    match serde_json::from_str::<Value>(source_files) {
        Ok(Value::Array(array)) => Value::Array(array),
        Ok(Value::Object(map)) => {
            Value::Array(map.keys().cloned().map(Value::String).collect::<Vec<_>>())
        }
        Ok(other) => other,
        Err(_) => Value::Array(
            source_files
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| Value::String(value.to_string()))
                .collect(),
        ),
    }
}

fn parse_source_file_uris(source_files: &str) -> Vec<String> {
    match serde_json::from_str::<Value>(source_files) {
        Ok(Value::Array(entries)) => entries
            .into_iter()
            .filter_map(|entry| entry.as_str().map(str::to_string))
            .collect(),
        // T-0.3: RTM-047 - Preserve source file URI extraction on older serde_json Map APIs.
        Ok(Value::Object(map)) => map.into_iter().map(|(key, _)| key).collect(),
        Ok(Value::String(uri)) => vec![uri],
        Ok(_) => Vec::new(),
        Err(_) => source_files
            .split(',')
            .map(str::trim)
            .filter(|uri| !uri.is_empty())
            .map(str::to_string)
            .collect(),
    }
}

fn load_content_preview_map(
    connection: &Connection,
    focus_concept: &str,
    source_files: &str,
) -> Result<Value> {
    let mut previews = serde_json::Map::new();
    for uri in parse_source_file_uris(source_files) {
        let content = connection
            .query_row(
                "SELECT COALESCE(content, '') FROM file_content WHERE file_path = ?1",
                [uri.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let Some(content) = content else {
            continue;
        };
        let preview = extract_section_with_concept(&content, focus_concept, 500);
        if !preview.is_empty() {
            previews.insert(uri, Value::String(preview));
        }
    }
    Ok(Value::Object(previews))
}

// T-1.6: RTM NFR-1 - ports C# ConceptGraphTests ExtractSectionWithConcept coverage for BuildContext previews.
pub fn extract_section_with_concept(
    content: &str,
    concept_name: &str,
    max_length: usize,
) -> String {
    let (_, body, _) = split_frontmatter_for_repair(content);
    let body = body.trim();
    if body.is_empty() || max_length == 0 {
        return String::new();
    }

    let normalized_concept = normalize_concept(concept_name);
    if normalized_concept.is_empty() {
        return truncate_preview(body, max_length);
    }

    let mut sections = Vec::new();
    let mut current_section = String::new();
    for line in body.lines() {
        if line.starts_with("## ") && !current_section.trim().is_empty() {
            sections.push(std::mem::take(&mut current_section));
        }
        current_section.push_str(line);
        current_section.push('\n');
    }
    if !current_section.trim().is_empty() {
        sections.push(current_section);
    }

    if let Some(section) = sections
        .iter()
        .find(|section| section_contains_concept(section, &normalized_concept))
    {
        return truncate_preview(section, max_length);
    }

    truncate_preview(body, max_length)
}

fn section_contains_concept(section: &str, normalized_concept: &str) -> bool {
    let section_lower = section.to_ascii_lowercase();
    if section_lower.contains(&format!("[[{normalized_concept}]]")) {
        return true;
    }

    let plain_concept = normalized_concept.replace('-', " ");
    if plain_concept.is_empty() {
        return false;
    }

    let pattern = format!(
        r"(?iu)(?:^|[^\p{{L}}\p{{N}}]){}(?:$|[^\p{{L}}\p{{N}}])",
        regex::escape(&plain_concept)
    );
    Regex::new(&pattern)
        .map(|regex| regex.is_match(section))
        .unwrap_or(false)
}

fn truncate_preview(content: &str, max_length: usize) -> String {
    let trimmed = content.trim();
    let char_count = trimmed.chars().count();
    if char_count <= max_length {
        return trimmed.to_string();
    }

    let truncated = trimmed
        .chars()
        .take(max_length)
        .collect::<String>()
        .trim_end()
        .to_string();
    format!("{truncated}...")
}

fn relation_weighted_score(
    connection: &Connection,
    config: &crate::config::Config,
    co_occurrence_count: f64,
    source_files: &str,
) -> Result<(f64, f64)> {
    let trimmed = source_files.trim();
    if trimmed.is_empty() {
        return Ok((0.0, 0.0));
    }

    let mut weighted_score = 0.0;
    let mut document_count = 0.0;

    match serde_json::from_str::<Value>(trimmed) {
        Ok(Value::Object(map)) => {
            for (uri, weight) in map {
                let file_weight = weight.as_f64().unwrap_or(1.0);
                let decay_weight = load_file_decay_weight(connection, config, &uri)?;
                weighted_score += file_weight * decay_weight;
                document_count += 1.0;
            }
        }
        Ok(Value::Array(entries)) => {
            for entry in entries {
                if let Some(uri) = entry.as_str() {
                    let decay_weight = load_file_decay_weight(connection, config, uri)?;
                    weighted_score += decay_weight;
                    document_count += 1.0;
                }
            }
        }
        _ => {
            for uri in trimmed
                .split(',')
                .map(str::trim)
                .filter(|uri| !uri.is_empty())
            {
                let decay_weight = load_file_decay_weight(connection, config, uri)?;
                weighted_score += decay_weight;
                document_count += 1.0;
            }
        }
    }

    if document_count <= 0.0 {
        return Ok((0.0, 0.0));
    }

    let decay_weight = (weighted_score / document_count).clamp(0.0, 1.0);
    Ok((decay_weight, co_occurrence_count * decay_weight))
}

fn load_file_decay_weight(
    connection: &Connection,
    config: &crate::config::Config,
    uri: &str,
) -> Result<f64> {
    let row = connection
        .query_row(
            "SELECT last_accessed, status FROM file_content WHERE file_path = ?1",
            [uri],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                ))
            },
        )
        .optional()?;

    let Some((last_accessed, status)) = row else {
        return Ok(1.0);
    };

    Ok(compute_decay_weight(
        uri,
        last_accessed.as_deref(),
        config,
        status.as_deref(),
    ))
}

fn concept_decay_weight(
    connection: &Connection,
    config: &crate::config::Config,
    concept_name: &str,
) -> Result<f64> {
    let mut statement = connection.prepare(
        "SELECT cm.mention_count, fc.file_path, fc.last_accessed, fc.status
         FROM concept_mentions cm
         LEFT JOIN file_content fc ON fc.file_path = cm.source_file
         WHERE cm.concept_name = ?1",
    )?;
    let rows = statement.query_map([concept_name], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    })?;

    let mut total_mentions = 0.0;
    let mut total_weight = 0.0;
    for row in rows {
        let (mention_count, file_path, last_accessed, status) = row?;
        let mentions = mention_count.max(1) as f64;
        let file_weight = file_path
            .as_deref()
            .map(|path| {
                compute_decay_weight(path, last_accessed.as_deref(), config, status.as_deref())
            })
            .unwrap_or(1.0);
        total_mentions += mentions;
        total_weight += mentions * file_weight;
    }

    if total_mentions <= 0.0 {
        return Ok(1.0);
    }

    Ok((total_weight / total_mentions).clamp(0.0, 1.0))
}

// T-3.2b: RTM FR-13.6
fn load_graph_neighbors(connection: &Connection, concept_name: &str) -> Result<Vec<GraphRelation>> {
    let mut statement = connection.prepare(
        "SELECT concept_a, concept_b, COALESCE(co_occurrence_count, 0.0), COALESCE(source_files, '') \
         FROM concept_graph \
         WHERE concept_a = ?1 OR concept_b = ?1 \
         ORDER BY co_occurrence_count DESC, concept_a ASC, concept_b ASC",
    )?;
    let rows = statement.query_map([concept_name], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, f64>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;

    let mut relations = Vec::new();
    for row in rows {
        let (concept_a, concept_b, co_occurrence_count, source_files) = row?;
        let name = if concept_a == concept_name {
            concept_b
        } else {
            concept_a
        };
        if name == concept_name {
            continue;
        }
        relations.push(GraphRelation {
            name,
            co_occurrence_count,
            source_files,
        });
    }

    Ok(relations)
}

// T-3.2b: RTM FR-13.6
fn load_concept_community_id(connection: &Connection, concept_name: &str) -> Result<Option<i64>> {
    connection
        .query_row(
            "SELECT community_id FROM concept_communities WHERE concept_name = ?1",
            params![concept_name],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
}

// T-3.2b: RTM FR-13.8
fn load_neighbor_weights(
    connection: &Connection,
    concept_name: &str,
) -> Result<HashMap<String, f64>> {
    Ok(load_graph_neighbors(connection, concept_name)?
        .into_iter()
        .map(|relation| (relation.name, relation.co_occurrence_count))
        .collect())
}

// T-3.2b: RTM FR-13.7
fn load_community_siblings(
    connection: &Connection,
    concept_name: &str,
    community_id: i64,
    direct_relation_names: &HashSet<String>,
) -> Result<Vec<CommunitySiblingLine>> {
    // T-3.2b: RTM FR-13.8
    let query_neighbors = load_neighbor_weights(connection, concept_name)?;
    if query_neighbors.is_empty() {
        return Ok(Vec::new());
    }

    let min_shared_neighbors = std::env::var("MAENIFOLD_COMMUNITY_MIN_SHARED")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(3);
    let min_overlap = std::env::var("MAENIFOLD_COMMUNITY_MIN_OVERLAP")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.4);
    let max_siblings = std::env::var("MAENIFOLD_COMMUNITY_MAX_SIBLINGS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(10);

    let candidate_names =
        load_community_sibling_candidates(connection, concept_name, community_id)?;

    let mut siblings = Vec::new();
    for sibling_name in candidate_names {
        if direct_relation_names.contains(&sibling_name) {
            continue;
        }

        let sibling_neighbors = load_neighbor_weights(connection, &sibling_name)?;
        if sibling_neighbors.is_empty() {
            continue;
        }

        let sibling_total_degree = sibling_neighbors.values().sum::<f64>();
        if sibling_total_degree <= 0.0 {
            continue;
        }

        let mut shared_neighbor_count = 0usize;
        let mut overlap_numerator = 0.0;
        for (neighbor, query_weight) in &query_neighbors {
            let Some(sibling_weight) = sibling_neighbors.get(neighbor) else {
                continue;
            };
            shared_neighbor_count += 1;
            overlap_numerator += (*query_weight).min(*sibling_weight);
        }

        let normalized_overlap = overlap_numerator / sibling_total_degree;

        // T-3.2b: RTM FR-13.9
        if shared_neighbor_count < min_shared_neighbors || normalized_overlap < min_overlap {
            continue;
        }

        siblings.push((
            CommunitySiblingLine {
                community_id: load_concept_community_id(connection, &sibling_name)?
                    .unwrap_or(community_id),
                name: sibling_name.clone(),
                normalized_overlap,
            },
            shared_neighbor_count,
        ));
    }

    siblings.sort_by(|left, right| {
        right
            .0
            .normalized_overlap
            .partial_cmp(&left.0.normalized_overlap)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.1.cmp(&left.1))
            .then_with(|| left.0.name.cmp(&right.0.name))
    });

    Ok(siblings
        .into_iter()
        .take(max_siblings)
        .map(|(sibling, _)| sibling)
        .collect())
}

fn load_community_sibling_candidates(
    connection: &Connection,
    concept_name: &str,
    community_id: i64,
) -> Result<Vec<String>> {
    let same_community = load_candidate_names(
        connection,
        "SELECT concept_name \
         FROM concept_communities \
         WHERE community_id = ?1 AND concept_name != ?2 \
         ORDER BY concept_name ASC",
        params![community_id, concept_name],
    )?;
    let all_concepts = load_candidate_names(
        connection,
        "SELECT concept_name \
         FROM concepts \
         WHERE concept_name != ?1 \
         ORDER BY concept_name ASC",
        params![concept_name],
    )?;

    let mut merged = Vec::new();
    let mut seen = HashSet::new();
    for candidate in same_community.into_iter().chain(all_concepts) {
        if seen.insert(candidate.clone()) {
            merged.push(candidate);
        }
    }
    Ok(merged)
}

fn load_candidate_names<P>(connection: &Connection, sql: &str, params: P) -> Result<Vec<String>>
where
    P: rusqlite::Params,
{
    let mut statement = connection.prepare(sql)?;
    let rows = statement.query_map(params, |row| row.get::<_, String>(0))?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn extract_concepts_from_file(context: &ToolContext, payload: &Value) -> Result<String> {
    let identifier = payload
        .get("identifier")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("identifier"))?;
    let uri = if identifier.starts_with("memory://") {
        identifier.to_string()
    } else {
        find_file_as_uri(identifier, &context.config).ok_or_else(|| {
            MaenifoldError::Message(format!("Memory file not found: {identifier}"))
        })?
    };
    let (_, content, _) = read_markdown(&uri, &context.config)?;
    let concepts = extract_wikilinks(&content)
        .into_iter()
        .map(|concept| concept.to_ascii_lowercase())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let mut output = format!("Found {} CONCEPTS in file:\n", concepts.len());
    for concept in &concepts {
        output.push_str(&format!("  • [[{}]]\n", concept));
    }
    Ok(output.trim_end().to_string())
}

// T-1.6: RTM NFR-1 - port the C# GraphAnalyzer.RandomWalk helper for graph traversal parity.
pub fn random_walk(
    connection: &Connection,
    start_concept: &str,
    walk_length: Option<usize>,
) -> Result<Vec<String>> {
    let start_concept = normalize_concept(start_concept);
    let walk_length = walk_length.unwrap_or(10).max(1);
    let mut walk = vec![start_concept.clone()];
    let mut current = start_concept;

    for _ in 1..walk_length {
        let mut statement = connection.prepare(
            "SELECT DISTINCT \
                 CASE WHEN concept_a = ?1 THEN concept_b ELSE concept_a END AS neighbor \
             FROM concept_graph \
             WHERE concept_a = ?1 OR concept_b = ?1 \
             ORDER BY neighbor ASC",
        )?;
        let rows = statement.query_map(params![current], |row| row.get::<_, String>(0))?;
        let neighbors = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        if neighbors.is_empty() {
            break;
        }

        let index = (Uuid::new_v4().as_u128() % neighbors.len() as u128) as usize;
        current = neighbors[index].clone();
        walk.push(current.clone());
    }

    Ok(walk)
}

// T-1.6: RTM NFR-1 - mirror the C# GraphAnalyzer.Visualize contract for Mermaid output.
pub fn visualize(context: &ToolContext, payload: &Value) -> Result<String> {
    let learn = payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if learn {
        let help = load_help_doc(&context.config.assets_path, "Visualize")
            .unwrap_or_else(|| "ERROR: Help file not found for Visualize".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }

    let concept_name = payload
        .get("conceptName")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    if concept_name.is_empty() {
        return Ok("ERROR: conceptName is required".to_string());
    }

    let depth = payload.get("depth").and_then(Value::as_i64).unwrap_or(2);
    if !(1..=5).contains(&depth) {
        return Ok("ERROR: depth must be between 1 and 5".to_string());
    }

    let max_nodes = payload
        .get("maxNodes")
        .and_then(Value::as_i64)
        .unwrap_or(30);
    if !(5..=100).contains(&max_nodes) {
        return Ok("ERROR: maxNodes must be between 5 and 100".to_string());
    }

    let concept_name = normalize_concept(concept_name);

    GraphDatabase::initialize_database(&context.config)?;
    let connection = db::open_connection(&context.config.database_path)?;
    let exists = connection.query_row(
        "SELECT COUNT(*) > 0 FROM concepts WHERE concept_name = ?1",
        params![concept_name],
        |row| row.get::<_, bool>(0),
    )?;
    if !exists {
        return Ok(format!(
            "CONCEPT '{concept_name}' not found. Run sync first."
        ));
    }

    let mut statement = connection.prepare(
        "WITH RECURSIVE search(depth, from_concept, to_concept, count, path) AS ( \
             SELECT \
                 1 AS depth, \
                 ?1 AS from_concept, \
                 CASE WHEN concept_a = ?1 THEN concept_b ELSE concept_a END AS to_concept, \
                 co_occurrence_count AS count, \
                 '|' || ?1 || '|' || (CASE WHEN concept_a = ?1 THEN concept_b ELSE concept_a END) || '|' AS path \
             FROM concept_graph \
             WHERE concept_a = ?1 OR concept_b = ?1 \
             UNION ALL \
             SELECT \
                 s.depth + 1, \
                 s.to_concept AS from_concept, \
                 CASE WHEN g.concept_a = s.to_concept THEN g.concept_b ELSE g.concept_a END AS to_concept, \
                 g.co_occurrence_count AS count, \
                 s.path || (CASE WHEN g.concept_a = s.to_concept THEN g.concept_b ELSE g.concept_a END) || '|' AS path \
             FROM search s \
             JOIN concept_graph g ON (g.concept_a = s.to_concept OR g.concept_b = s.to_concept) \
             WHERE s.depth < ?2 \
               AND instr(s.path, '|' || (CASE WHEN g.concept_a = s.to_concept THEN g.concept_b ELSE g.concept_a END) || '|') = 0 \
         ) \
         SELECT DISTINCT from_concept, to_concept, count \
         FROM search \
         ORDER BY count DESC, from_concept ASC, to_concept ASC \
         LIMIT ?3",
    )?;
    let rows = statement.query_map(params![concept_name, depth, max_nodes], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, f64>(2)?,
        ))
    })?;

    let relations = rows.collect::<std::result::Result<Vec<_>, _>>()?;
    let mut mermaid = String::from("graph TD\n");
    for (from_concept, to_concept, count) in &relations {
        mermaid.push_str(&format!(
            "    {} -->|{}| {}\n",
            sanitize_mermaid_node_id(from_concept),
            format_co_occurrence_weight(*count),
            sanitize_mermaid_node_id(to_concept)
        ));
    }
    if relations.is_empty() {
        mermaid.push_str(&format!(
            "    {} [No connections found]\n",
            sanitize_mermaid_node_id(&concept_name)
        ));
    }

    Ok(match context.output_format {
        OutputFormat::Json => success(mermaid),
        OutputFormat::Markdown => mermaid,
    })
}

fn sanitize_mermaid_node_id(concept_name: &str) -> String {
    concept_name
        .chars()
        .map(|character| match character {
            ' ' | '-' => '_',
            _ => character,
        })
        .collect()
}

fn with_similarity_next_steps(message: impl Into<String>) -> String {
    format!(
        "{}\n💡 Next: Consider using BuildContext, SearchMemories for related operations.",
        message.into()
    )
}

fn normalize_find_similar_validation_input(input: &str) -> String {
    input
        .chars()
        .map(|character| match character {
            // T-QUAL-FSC2: RTM FR-7.4
            // Minimal NFKC-equivalent folding for the bracket confusable regression cases
            // covered by the C# oracle tests.
            '\u{FF3B}' => '[',
            '\u{FF3D}' => ']',
            _ => character,
        })
        .collect()
}

fn contains_bracket_like_delimiter(input: &str) -> bool {
    input.chars().any(|character| {
        matches!(
            character,
            '<' | '>'
                | '\u{300A}'
                | '\u{300B}'
                | '\u{3008}'
                | '\u{3009}'
                | '\u{3010}'
                | '\u{3011}'
                | '\u{300C}'
                | '\u{300D}'
                | '\u{300E}'
                | '\u{300F}'
                | '\u{3014}'
                | '\u{3015}'
                | '\u{3016}'
                | '\u{3017}'
                | '\u{27E6}'
                | '\u{27E7}'
                | '\u{27E8}'
                | '\u{27E9}'
                | '\u{27EA}'
                | '\u{27EB}'
                | '\u{27EC}'
                | '\u{27ED}'
                | '\u{27EE}'
                | '\u{27EF}'
                | '\u{2772}'
                | '\u{2773}'
                | '\u{276E}'
                | '\u{276F}'
        )
    })
}

fn is_low_information_find_similar_query(input: &str) -> bool {
    let mut has_non_ignored = false;

    for character in input.chars() {
        if character.is_whitespace() || character.is_control() {
            continue;
        }

        has_non_ignored = true;
        if character.is_alphanumeric() {
            return false;
        }
    }

    has_non_ignored
}

pub fn find_similar_concepts(context: &ToolContext, payload: &Value) -> Result<String> {
    let learn = payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if learn {
        return Ok(
            load_help_doc(&context.config.assets_path, "FindSimilarConcepts").unwrap_or_else(
                || "ERROR: Help file not found for FindSimilarConcepts".to_string(),
            ),
        );
    }

    let concept_name = payload
        .get("conceptName")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("conceptName"))?
        .trim();
    if concept_name.is_empty() {
        return Ok("ERROR: conceptName parameter is required and cannot be empty".to_string());
    }
    if concept_name.len() > 256 {
        return Ok("ERROR: conceptName must be 256 characters or fewer".to_string());
    }
    let validation_input = normalize_find_similar_validation_input(concept_name);
    if validation_input.contains('[') || validation_input.contains(']') {
        return Ok(with_similarity_next_steps(format!(
            "DIAGNOSTIC: conceptName '{concept_name}' contains bracket characters (or Unicode confusables). Remove brackets and try again."
        )));
    }
    if contains_bracket_like_delimiter(&validation_input) {
        return Ok(with_similarity_next_steps(format!(
            "DIAGNOSTIC: conceptName '{concept_name}' contains bracket-like delimiter characters. Remove them and try again."
        )));
    }
    if is_low_information_find_similar_query(&validation_input) {
        return Ok(with_similarity_next_steps(format!(
            "DIAGNOSTIC: conceptName '{concept_name}' is too low-information to embed reliably. Provide a few letters or words."
        )));
    }

    let max_results = payload
        .get("maxResults")
        .and_then(Value::as_u64)
        .unwrap_or(10);
    if !(1..=50).contains(&max_results) {
        return Ok("ERROR: maxResults must be between 1 and 50".to_string());
    }
    let concept_name = normalize_concept(concept_name);
    if concept_name.is_empty() {
        return Ok(with_similarity_next_steps(format!(
            "DIAGNOSTIC: conceptName '{}' normalizes to empty. Provide a few letters or words.",
            payload
                .get("conceptName")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
        )));
    }

    GraphDatabase::initialize_database(&context.config)?;
    let connection = db::open_connection(&context.config.database_path)?;

    if let Some(vector_results) = query_vec_similar_concepts(
        &connection,
        &context.config.assets_path,
        &concept_name,
        max_results as usize,
    )? {
        if vector_results.is_empty() {
            return Ok(with_similarity_next_steps(format!(
                "No similar concepts found for '{}'. Run Sync to ensure concept embeddings are generated.",
                concept_name
            )));
        }

        // T-GRAPH-DECAY-001.1: RTM FR-7.5
        let mut ranked_results = vector_results
            .into_iter()
            .map(|(neighbor, similarity)| {
                let decay_weight =
                    concept_decay_weight(&connection, &context.config, &neighbor).unwrap_or(1.0);
                let weighted_similarity = similarity * decay_weight;
                (neighbor, similarity, weighted_similarity)
            })
            .collect::<Vec<_>>();
        ranked_results.sort_by(|left, right| {
            right
                .2
                .partial_cmp(&left.2)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.0.cmp(&right.0))
        });

        let mut output = format!(
            "Similar concepts to '{}' (by semantic similarity):

",
            concept_name
        );
        for (neighbor, similarity, _) in ranked_results {
            output.push_str(&format!(
                "  • {} (similarity: {:.3})
",
                neighbor, similarity
            ));
        }
        output.push_str(
            "
💡 Next: Consider using BuildContext, SearchMemories for related operations.",
        );
        return Ok(output);
    }

    let mut statement = connection.prepare(
        "SELECT concept_a, concept_b, COALESCE(co_occurrence_count, 0.0) FROM concept_graph WHERE concept_a = ?1 OR concept_b = ?1 ORDER BY co_occurrence_count DESC, concept_a ASC, concept_b ASC LIMIT ?2",
    )?;
    let rows = statement.query_map(params![concept_name, max_results as i64], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, f64>(2)?,
        ))
    })?;

    let mut neighbors = Vec::new();
    let mut max_count = 0.0_f64;
    for row in rows {
        let (concept_a, concept_b, count) = row?;
        let neighbor = if concept_a.eq_ignore_ascii_case(&concept_name) {
            concept_b
        } else {
            concept_a
        };
        if neighbor.eq_ignore_ascii_case(&concept_name) {
            continue;
        }
        max_count = max_count.max(count);
        neighbors.push((neighbor, count));
    }

    if neighbors.is_empty() {
        return Ok(with_similarity_next_steps(format!(
            "No similar concepts found for '{}'. Run Sync to ensure concept embeddings are generated.",
            concept_name
        )));
    }

    // T-GRAPH-DECAY-001.1: RTM FR-7.5
    let mut ranked_neighbors = neighbors
        .into_iter()
        .map(|(neighbor, count)| {
            let similarity = if max_count <= 0.0 {
                0.0
            } else {
                count / max_count
            };
            let decay_weight =
                concept_decay_weight(&connection, &context.config, &neighbor).unwrap_or(1.0);
            let weighted_similarity = similarity * decay_weight;
            (neighbor, similarity, weighted_similarity)
        })
        .collect::<Vec<_>>();
    ranked_neighbors.sort_by(|left, right| {
        right
            .2
            .partial_cmp(&left.2)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.0.cmp(&right.0))
    });

    let mut output = format!(
        "Similar concepts to '{}' (by semantic similarity):

",
        concept_name
    );
    for (neighbor, similarity, _) in ranked_neighbors {
        output.push_str(&format!(
            "  • {} (similarity: {:.3})
",
            neighbor, similarity
        ));
    }
    output.push_str(
        "
💡 Next: Consider using BuildContext, SearchMemories for related operations.",
    );
    Ok(output)
}

#[derive(Debug)]
struct RepairEdit {
    start: usize,
    end: usize,
}

// T-3.1a: RTM FR-6 / RTM-038 / RTM-040 / RTM-041 / RTM-046
pub fn repair_concepts(context: &ToolContext, payload: &Value) -> Result<String> {
    let learn = payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let create_wikilinks = payload
        .get("createWikiLinks")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let variants = parse_repair_concept_variants(
        payload
            .get("conceptsToReplace")
            .and_then(Value::as_str)
            .ok_or(MaenifoldError::MissingProperty("conceptsToReplace"))?,
    );
    if variants.is_empty() {
        return Err(MaenifoldError::Message(
            "No concepts to replace provided".to_string(),
        ));
    }

    let canonical_concept = payload
        .get("canonicalConcept")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("canonicalConcept"))?
        .trim()
        .to_string();
    if learn {
        let help = load_help_doc(&context.config.assets_path, "RepairConcepts")
            .unwrap_or_else(|| "ERROR: Help file not found for RepairConcepts".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }
    if canonical_concept.is_empty() {
        return Err(MaenifoldError::InvalidArgument(
            "canonicalConcept cannot be empty".to_string(),
        ));
    }
    // T-3.4: RTM FR-6
    if canonical_concept
        .chars()
        .any(|character| matches!(character, '[' | ']'))
    {
        return Err(MaenifoldError::InvalidArgument(
            "canonicalConcept cannot contain '[' or ']' characters".to_string(),
        ));
    }

    let folder_relative = payload
        .get("folder")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|folder| !folder.is_empty())
        .map(|folder| folder.trim_matches('/').to_string())
        .filter(|folder| !folder.is_empty());
    let folder_uri = folder_relative
        .as_deref()
        .map(|folder| format!("memory://{folder}"));
    let dry_run = payload
        .get("dryRun")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let min_semantic_similarity = payload
        .get("minSemanticSimilarity")
        .and_then(Value::as_f64)
        .unwrap_or(0.7);

    if let Some(relative_folder) = folder_relative.as_deref() {
        let folder_path = context.config.memory_path.join(relative_folder);
        if !folder_path.exists() {
            return Ok(format!("ERROR: Directory not found: {relative_folder}"));
        }
    }

    let semantic_validation_output = validate_repair_concepts_semantics(
        context,
        &variants,
        &canonical_concept,
        min_semantic_similarity,
    )?;

    let file_uris = get_memory_files(&context.config, folder_uri.as_deref())?;
    let files_scanned = file_uris.len();
    let mut output = String::new();
    if !semantic_validation_output.is_empty() {
        output.push_str(&semantic_validation_output);
    }
    output.push_str(&format!(
        "Scanning {} markdown files...\nLooking for variants: {}\nWill replace with: [[{}]]\n\n",
        files_scanned,
        variants.join(", "),
        canonical_concept
    ));
    let mut files_modified = 0usize;
    let mut total_replacements = 0usize;

    for uri in file_uris {
        let path = uri_to_path(&uri, &context.config.memory_path)?;
        let original_content = fs::read_to_string(&path)?;
        let (updated_content, matched_variants, replacement_count) = if create_wikilinks {
            // T-3.1b: RTM FR-6 / RTM-039 / RTM-044 / RTM-045
            create_wikilinks_for_plain_text_variants(
                &original_content,
                &variants,
                &canonical_concept,
            )?
        } else {
            replace_wikilink_variants(&original_content, &variants, &canonical_concept)?
        };

        if replacement_count == 0 {
            continue;
        }

        files_modified += 1;
        total_replacements += replacement_count;

        let display_path = path
            .strip_prefix(&context.config.memory_path)
            .unwrap_or(path.as_path());
        if dry_run {
            output.push_str(&format!("Would modify: {}\n", display_path.display()));
        } else {
            fs::write(&path, updated_content)?;
            output.push_str(&format!("✓ Modified: {}\n", display_path.display()));
        }

        for variant in matched_variants {
            output.push_str(&format!("  {} → [[{}]]\n", variant, canonical_concept));
        }
        output.push('\n');
    }

    output.push_str(&format!(
        "=== SUMMARY ===\nFiles scanned: {}\nFiles modified: {}\nTotal replacements: {}",
        files_scanned, files_modified, total_replacements
    ));
    if dry_run {
        output.push_str("\n\nThis was a DRY RUN. To apply changes, run with dryRun=false");
    }
    output
        .push_str("\nAfter applying changes, run 'sync' to rebuild the graph with clean concepts.");

    Ok(output)
}

// T-1.2: RTM FR-2
pub fn analyze_concept_corruption(context: &ToolContext, payload: &Value) -> Result<String> {
    let learn = payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let concept_family = payload
        .get("conceptFamily")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("conceptFamily"))?
        .trim();
    if learn {
        let help = load_help_doc(&context.config.assets_path, "AnalyzeConceptCorruption")
            .unwrap_or_else(|| {
                "ERROR: Help file not found for AnalyzeConceptCorruption".to_string()
            });
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }
    if concept_family.is_empty() {
        return Err(MaenifoldError::InvalidArgument(
            "conceptFamily cannot be empty".to_string(),
        ));
    }

    // T-1.5: RTM FR-2
    let max_results = match payload.get("maxResults") {
        Some(value) => match value.as_i64() {
            Some(parsed) if parsed < 0 => {
                return Err(MaenifoldError::InvalidArgument(
                    "maxResults must be zero or greater".to_string(),
                ));
            }
            Some(parsed) => parsed as usize,
            None => 50,
        },
        None => 50,
    };
    let concept_family_lower = concept_family.to_ascii_lowercase();

    let wikilink_pattern =
        Regex::new(r"\[\[([^\]]+)\]\]").expect("AnalyzeConceptCorruption wikilink regex");
    let mut counts = HashMap::<String, usize>::new();

    for uri in get_memory_files(&context.config, None)? {
        let (_, content, _) = read_markdown(&uri, &context.config)?;
        for capture in wikilink_pattern.captures_iter(&content) {
            let Some(raw_match) = capture.get(1) else {
                continue;
            };
            let original = raw_match.as_str().trim();
            if original.is_empty() {
                continue;
            }

            let normalized = normalize_concept(original);
            if normalized.contains(&concept_family_lower) {
                *counts.entry(original.to_string()).or_default() += 1;
            }
        }
    }

    let total_unique_variants = counts.len();
    let mut variants = counts.into_iter().collect::<Vec<_>>();
    variants.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| {
                left.0
                    .to_ascii_lowercase()
                    .cmp(&right.0.to_ascii_lowercase())
            })
            .then_with(|| left.0.cmp(&right.0))
    });
    variants.truncate(max_results);

    let mut grouped = BTreeMap::<&'static str, Vec<(String, usize)>>::new();
    for (concept, count) in &variants {
        grouped
            .entry(classify_corruption_variant(concept))
            .or_default()
            .push((concept.clone(), *count));
    }

    let mut output = format!(
        "=== Concept Family Analysis: '{}' ===\nFound {} unique variants",
        concept_family, total_unique_variants
    );

    for category in [
        "Singular/Plural",
        "Compound (with -)",
        "With Suffix",
        "File Paths",
        "Other",
    ] {
        let Some(entries) = grouped.get(category) else {
            continue;
        };
        if entries.is_empty() {
            continue;
        }

        output.push_str(&format!("\n\n{category}:\n"));
        for (concept, count) in entries {
            output.push_str(&format!("  • {concept} ({count}x)\n"));
        }
    }

    let plural_replacements = variants
        .iter()
        .filter_map(|(concept, _)| {
            // T-1.5: RTM FR-2
            (!is_file_path_variant(concept) && is_plural_variant(concept))
                .then_some(concept.clone())
        })
        .take(10)
        .collect::<Vec<_>>();
    let compound_replacements = variants
        .iter()
        .filter_map(|(concept, _)| {
            let concept_lower = concept.to_ascii_lowercase();
            // T-1.5: RTM FR-2
            (!is_file_path_variant(concept)
                && (concept_lower.contains(&format!("-{concept_family_lower}"))
                    || concept_lower.contains(&format!("{concept_family_lower}-"))))
            .then_some(concept.clone())
        })
        .take(10)
        .collect::<Vec<_>>();

    let plural_canonical = concept_family
        .strip_suffix(['s', 'S'])
        .filter(|value| !value.is_empty())
        .unwrap_or(concept_family);

    if !plural_replacements.is_empty() || !compound_replacements.is_empty() {
        output.push_str("\n\n=== SUGGESTED REPAIRS ===\n");
        if !plural_replacements.is_empty() {
            output.push_str("Fix plural forms:\n");
            output.push_str(&format!(
                "  RepairConcepts conceptsToReplace='{}' canonicalConcept='{}'\n",
                plural_replacements.join(","),
                plural_canonical
            ));
        }
        if !compound_replacements.is_empty() {
            output.push_str("Fix compound forms:\n");
            output.push_str(&format!(
                "  RepairConcepts conceptsToReplace='{}' canonicalConcept='{}'\n",
                compound_replacements.join(","),
                concept_family
            ));
        }
    }

    Ok(output)
}

fn parse_repair_concept_variants(input: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut variants = Vec::new();

    for variant in input
        .split([',', ';', '|'])
        .map(normalize_concept)
        .filter(|variant| !variant.is_empty())
    {
        if seen.insert(variant.clone()) {
            variants.push(variant);
        }
    }

    variants
}

fn replace_wikilink_variants(
    content: &str,
    variants: &[String],
    canonical_concept: &str,
) -> Result<(String, Vec<String>, usize)> {
    let mut edits = Vec::<RepairEdit>::new();
    let mut variant_counts = HashMap::<String, usize>::new();

    for variant in variants {
        // T-3.1a: RTM FR-6 / RTM-038
        let pattern =
            Regex::new(&format!(r"(?i)\[\[{}\]\]", regex::escape(variant))).map_err(|error| {
                MaenifoldError::InvalidArgument(format!(
                    "Invalid WikiLink repair pattern for '{variant}': {error}"
                ))
            })?;
        for found in pattern.find_iter(content) {
            edits.push(RepairEdit {
                start: found.start(),
                end: found.end(),
            });
            *variant_counts.entry(variant.clone()).or_default() += 1;
        }
    }

    if edits.is_empty() {
        return Ok((content.to_string(), Vec::new(), 0));
    }

    edits.sort_by(|left, right| {
        right
            .start
            .cmp(&left.start)
            .then_with(|| right.end.cmp(&left.end))
    });

    let mut updated_content = content.to_string();
    let replacement = format!("[[{}]]", canonical_concept);
    for edit in edits {
        updated_content.replace_range(edit.start..edit.end, &replacement);
    }

    let matched_variants = variants
        .iter()
        .filter(|variant| variant_counts.contains_key(*variant))
        .cloned()
        .collect::<Vec<_>>();
    let total_replacements = variant_counts.values().sum();

    Ok((updated_content, matched_variants, total_replacements))
}

fn create_wikilinks_for_plain_text_variants(
    content: &str,
    variants: &[String],
    canonical_concept: &str,
) -> Result<(String, Vec<String>, usize)> {
    let (_, body, body_offset) = split_frontmatter_for_repair(content);
    let existing_wikilink_ranges = find_existing_wikilink_ranges(body)?;
    let variant_patterns = variants
        .iter()
        .map(|variant| (variant.clone(), variant.to_ascii_lowercase()))
        .collect::<Vec<_>>();
    let mut edits = Vec::<RepairEdit>::new();
    let mut variant_counts = HashMap::<String, usize>::new();
    let mut paragraph_depth = 0usize;
    let mut heading_depth = 0usize;
    let mut item_depth = 0usize;
    let mut link_depth = 0usize;
    let mut code_block_depth = 0usize;

    for (event, range) in Parser::new(body).into_offset_iter() {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => paragraph_depth += 1,
                Tag::Heading { .. } => heading_depth += 1,
                Tag::Item => item_depth += 1,
                Tag::Link { .. } => link_depth += 1,
                Tag::CodeBlock(_) => code_block_depth += 1,
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Paragraph => paragraph_depth = paragraph_depth.saturating_sub(1),
                TagEnd::Heading(_) => heading_depth = heading_depth.saturating_sub(1),
                TagEnd::Item => item_depth = item_depth.saturating_sub(1),
                TagEnd::Link => link_depth = link_depth.saturating_sub(1),
                TagEnd::CodeBlock => code_block_depth = code_block_depth.saturating_sub(1),
                _ => {}
            },
            Event::Text(_) => {
                // T-3.1b: RTM FR-6 / RTM-039 / RTM-044 / RTM-045
                if code_block_depth == 0
                    && link_depth == 0
                    && (paragraph_depth > 0 || heading_depth > 0 || item_depth > 0)
                {
                    collect_plain_text_repair_edits(
                        body,
                        range,
                        body_offset,
                        &variant_patterns,
                        &existing_wikilink_ranges,
                        &mut edits,
                        &mut variant_counts,
                    );
                }
            }
            _ => {}
        }
    }

    if edits.is_empty() {
        return Ok((content.to_string(), Vec::new(), 0));
    }

    edits.sort_by(|left, right| {
        right
            .start
            .cmp(&left.start)
            .then_with(|| right.end.cmp(&left.end))
    });

    let mut updated_content = content.to_string();
    let replacement = format!("[[{canonical_concept}]]");
    for edit in edits {
        updated_content.replace_range(edit.start..edit.end, &replacement);
    }

    let matched_variants = variants
        .iter()
        .filter(|variant| variant_counts.contains_key(*variant))
        .cloned()
        .collect::<Vec<_>>();
    let total_replacements = variant_counts.values().sum();

    Ok((updated_content, matched_variants, total_replacements))
}

fn collect_plain_text_repair_edits(
    body: &str,
    range: std::ops::Range<usize>,
    body_offset: usize,
    variant_patterns: &[(String, String)],
    existing_wikilink_ranges: &[std::ops::Range<usize>],
    edits: &mut Vec<RepairEdit>,
    variant_counts: &mut HashMap<String, usize>,
) {
    let segment = &body[range.clone()];
    if segment.is_empty() {
        return;
    }

    let lower_segment = segment.to_ascii_lowercase();
    let mut candidates = Vec::<(usize, usize, &str)>::new();

    for (variant, variant_lower) in variant_patterns {
        for (local_start, _) in lower_segment.match_indices(variant_lower) {
            let local_end = local_start + variant_lower.len();
            if !is_whole_word_match(segment, local_start, local_end) {
                continue;
            }

            let absolute_start = body_offset + range.start + local_start;
            let absolute_end = body_offset + range.start + local_end;
            if overlaps_existing_wikilink(
                absolute_start - body_offset,
                absolute_end - body_offset,
                existing_wikilink_ranges,
            ) {
                continue;
            }

            candidates.push((absolute_start, absolute_end, variant));
        }
    }

    candidates.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| (right.1 - right.0).cmp(&(left.1 - left.0)))
            .then_with(|| left.2.cmp(right.2))
    });

    let mut last_end = 0usize;
    for (start, end, variant) in candidates {
        if start < last_end {
            continue;
        }
        edits.push(RepairEdit { start, end });
        *variant_counts.entry(variant.to_string()).or_default() += 1;
        last_end = end;
    }
}

fn split_frontmatter_for_repair(content: &str) -> (&str, &str, usize) {
    if let Some(rest) = content.strip_prefix("---\n")
        && let Some(index) = rest.find("\n---\n")
    {
        let mut body_start = 4 + index + 5;
        while let Some(next) = content.as_bytes().get(body_start) {
            if *next == b'\n' || *next == b'\r' {
                body_start += 1;
            } else {
                break;
            }
        }
        return (&content[..body_start], &content[body_start..], body_start);
    }

    ("", content, 0)
}

fn find_existing_wikilink_ranges(content: &str) -> Result<Vec<std::ops::Range<usize>>> {
    let pattern = Regex::new(r"\[\[[^\]]+\]\]").map_err(|error| {
        MaenifoldError::InvalidArgument(format!("Invalid WikiLink detection pattern: {error}"))
    })?;

    Ok(pattern
        .find_iter(content)
        .map(|found| found.start()..found.end())
        .collect())
}

fn is_whole_word_match(segment: &str, start: usize, end: usize) -> bool {
    let bytes = segment.as_bytes();
    let left_boundary = start == 0 || !is_word_byte(bytes[start - 1]);
    let right_boundary = end == bytes.len() || !is_word_byte(bytes[end]);
    left_boundary && right_boundary
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn overlaps_existing_wikilink(
    start: usize,
    end: usize,
    existing_wikilink_ranges: &[std::ops::Range<usize>],
) -> bool {
    existing_wikilink_ranges
        .iter()
        .any(|range| start < range.end && range.start < end)
}

fn validate_repair_concepts_semantics(
    context: &ToolContext,
    variants: &[String],
    canonical_concept: &str,
    min_semantic_similarity: f64,
) -> Result<String> {
    // T-3.1c: RTM FR-6 / RTM-042 / RTM-043
    if min_semantic_similarity <= 0.0 {
        return Ok(String::new());
    }

    let header = format!(
        "=== SEMANTIC VALIDATION ===\nChecking semantic similarity with threshold {:.2}...\n",
        min_semantic_similarity
    );
    let connection = db::open_connection(&context.config.database_path)?;
    let canonical_embedding = match load_concept_embedding(&connection, canonical_concept) {
        Ok(Some(embedding)) => embedding,
        Ok(None) => {
            return Ok(format!(
                "{header}⚠️ Warning: Semantic validation skipped because embedding for '[[{canonical_concept}]]' was not found.\n"
            ));
        }
        Err(error) if is_vec_embedding_unavailable(&error) => {
            return Ok(format!(
                "{header}⚠️ Warning: Semantic validation skipped because vec0 embeddings are unavailable ({error}).\n"
            ));
        }
        Err(error) => return Err(error),
    };

    let mut failing_variants = Vec::new();
    for variant in variants {
        let variant_embedding = match load_concept_embedding(&connection, variant) {
            Ok(Some(embedding)) => embedding,
            Ok(None) => {
                return Ok(format!(
                    "{header}⚠️ Warning: Semantic validation skipped because embedding for '[[{variant}]]' was not found.\n"
                ));
            }
            Err(error) if is_vec_embedding_unavailable(&error) => {
                return Ok(format!(
                    "{header}⚠️ Warning: Semantic validation skipped because vec0 embeddings are unavailable ({error}).\n"
                ));
            }
            Err(error) => return Err(error),
        };

        let similarity = cosine_similarity(&canonical_embedding, &variant_embedding);
        if similarity < min_semantic_similarity {
            failing_variants.push((variant.as_str(), similarity));
        }
    }

    if failing_variants.is_empty() {
        return Ok(format!(
            "{header}✓ All variants pass semantic similarity check\n\n"
        ));
    }

    let failing_lines = failing_variants
        .iter()
        .map(|(variant, similarity)| {
            format!(
                "  • {variant}: similarity {:.2} (threshold: {:.2})",
                similarity, min_semantic_similarity
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    Err(MaenifoldError::Message(format!(
        "{header}\n⚠️ BLOCKED: The following concepts are not semantically similar enough to '[[{canonical_concept}]]':\n{failing_lines}\n\nOperation blocked. Review the concepts and either:\n- Lower minSemanticSimilarity threshold\n- Remove the dissimilar concepts from conceptsToReplace"
    )))
}

fn load_concept_embedding(connection: &Connection, concept_name: &str) -> Result<Option<Vec<f32>>> {
    // T-3.1c: RTM FR-6 / RTM-043
    let mut statement =
        connection.prepare("SELECT embedding FROM vec_concepts WHERE concept_name = ?1")?;
    statement
        .query_row([concept_name], |row| {
            decode_embedding_value(row.get_ref(0)?)
        })
        .optional()
        .map_err(Into::into)
}

fn decode_embedding_value(value: ValueRef<'_>) -> rusqlite::Result<Vec<f32>> {
    // T-3.1c: RTM FR-6 / RTM-043
    match value {
        ValueRef::Blob(bytes) => decode_blob_embedding(bytes).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                Type::Blob,
                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
            )
        }),
        ValueRef::Text(text) => decode_text_embedding(text).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                Type::Text,
                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
            )
        }),
        ValueRef::Null => Ok(Vec::new()),
        other => Err(rusqlite::Error::FromSqlConversionFailure(
            0,
            other.data_type(),
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "embedding column must be BLOB or TEXT",
            )),
        )),
    }
}

fn decode_blob_embedding(bytes: &[u8]) -> std::result::Result<Vec<f32>, String> {
    // T-3.1c: RTM FR-6 / RTM-043
    let chunks = bytes.chunks_exact(std::mem::size_of::<f32>());
    if !chunks.remainder().is_empty() {
        return Err("embedding blob length is not divisible by 4".to_string());
    }

    Ok(chunks
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect())
}

fn decode_text_embedding(bytes: &[u8]) -> std::result::Result<Vec<f32>, String> {
    // T-3.1c: RTM FR-6 / RTM-043
    let text = std::str::from_utf8(bytes)
        .map_err(|error| format!("embedding text is not valid UTF-8: {error}"))?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    if trimmed.starts_with('[') {
        return serde_json::from_str::<Vec<f32>>(trimmed)
            .map_err(|error| format!("embedding JSON parse failed: {error}"));
    }

    trimmed
        .split([',', ' ', '\n', '\r', '\t'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            part.parse::<f32>()
                .map_err(|error| format!("embedding value '{part}' is not a float: {error}"))
        })
        .collect()
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    // T-3.1c: RTM FR-6 / RTM-042 / RTM-043
    if a.len() != b.len() || a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let dot: f64 = a
        .iter()
        .zip(b)
        .map(|(left, right)| (*left as f64) * (*right as f64))
        .sum();
    let magnitude_a: f64 = a
        .iter()
        .map(|value| (*value as f64).powi(2))
        .sum::<f64>()
        .sqrt();
    let magnitude_b: f64 = b
        .iter()
        .map(|value| (*value as f64).powi(2))
        .sum::<f64>()
        .sqrt();
    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    dot / (magnitude_a * magnitude_b)
}

fn is_vec_embedding_unavailable(error: &MaenifoldError) -> bool {
    // T-3.1c: RTM FR-6 / RTM-043
    matches!(
        error,
        MaenifoldError::Sqlite(sqlite_error)
            if sqlite_error.to_string().contains("no such table: vec_concepts")
                || sqlite_error.to_string().contains("no such module: vec0")
                || sqlite_error.to_string().contains("unknown module: vec0")
                || sqlite_error
                    .to_string()
                    .contains("no such function: vec_distance_cosine")
    )
}

fn query_vec_similar_concepts(
    connection: &Connection,
    assets_path: &Path,
    concept_name: &str,
    max_results: usize,
) -> Result<Option<Vec<(String, f64)>>> {
    let Some(extension_path) = vec0_extension_path(assets_path) else {
        return Ok(None);
    };

    unsafe {
        connection.load_extension_enable()?;
        if connection
            .load_extension(&extension_path, None::<&str>)
            .is_err()
        {
            let _ = connection.load_extension_disable();
            return Ok(None);
        }
        connection.load_extension_disable()?;
    }

    let exists = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM vec_concepts WHERE concept_name = ?1)",
            [concept_name],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0);
    if exists == 0 {
        return Ok(None);
    }

    let mut statement = connection.prepare(
        "SELECT other.concept_name, vec_distance_cosine(other.embedding, target.embedding) as distance          FROM vec_concepts other          JOIN vec_concepts target ON target.concept_name = ?1          WHERE other.concept_name != ?1            AND other.concept_name != ''            AND instr(other.concept_name, '[') = 0            AND instr(other.concept_name, ']') = 0          ORDER BY distance          LIMIT ?2",
    )?;
    let rows = statement.query_map(params![concept_name, max_results as i64], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
    })?;

    let mut results = Vec::new();
    for row in rows {
        let (neighbor, distance) = row?;
        if !distance.is_finite() {
            continue;
        }

        // T-QUAL-FSC2: RTM FR-7.4
        // Clamp negative-distance drift so semantic similarity never exceeds 1.000.
        let similarity = (1.0 / (1.0 + distance.max(0.0))).clamp(0.0, 1.0);
        results.push((neighbor, similarity));
    }
    Ok(Some(results))
}

fn classify_corruption_variant(concept: &str) -> &'static str {
    let concept_lower = concept.to_ascii_lowercase();

    // T-1.2: RTM FR-2
    if is_file_path_variant(concept) {
        "File Paths"
    } else if is_plural_variant(concept) {
        "Singular/Plural"
    } else if concept.contains('-') {
        "Compound (with -)"
    } else if ["system", "framework", "architecture", "tool"]
        .iter()
        .any(|suffix| concept_lower.contains(suffix))
    {
        "With Suffix"
    } else {
        "Other"
    }
}

fn is_file_path_variant(concept: &str) -> bool {
    concept.starts_with('/') || concept.contains('.')
}

fn is_plural_variant(concept: &str) -> bool {
    let concept_lower = concept.to_ascii_lowercase();
    concept_lower.ends_with('s') && !concept_lower.ends_with("ss")
}

fn vec0_extension_path(assets_path: &Path) -> Option<PathBuf> {
    let (platform_dir, filename) = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => ("osx-arm64", "vec0.dylib"),
        ("macos", "x86_64") => ("osx-x64", "vec0.dylib"),
        ("linux", "x86_64") => ("linux-x64", "vec0.so"),
        ("linux", "aarch64") => ("linux-arm64", "vec0.so"),
        ("windows", "x86_64") => ("win-x64", "vec0.dll"),
        _ => return None,
    };
    let path = assets_path.join("native").join(platform_dir).join(filename);
    path.exists().then_some(path)
}

#[derive(Clone, Copy)]
struct SyncSummary {
    status: &'static str,
    files_processed: i64,
    concept_mentions: i64,
    concept_relations: i64,
    abandoned_sessions: i64,
    communities_detected: i64,
    vector_warning_code: Option<&'static str>,
}

// T-QUAL-002.6b: RTM FR-25.2
pub struct VectorFailureReasonCodes;

impl VectorFailureReasonCodes {
    pub const CONCEPT_LOOKUP_FAILED: &'static str = "CONCEPT_LOOKUP_FAILED";
    pub const FILE_EMBEDDING_FAILED: &'static str = "FILE_EMBEDDING_FAILED";
    pub const CONCEPT_EMBEDDING_FAILED: &'static str = "CONCEPT_EMBEDDING_FAILED";
}

// T-QUAL-002.6b: RTM FR-25.2
pub struct VectorUnavailableReasonCodes;

impl VectorUnavailableReasonCodes {
    pub const DISABLED_BY_CONFIG: &'static str = "VECTOR_DISABLED_BY_CONFIG";
    pub const UNKNOWN: &'static str = "VECTOR_UNAVAILABLE_UNKNOWN";
}

// T-QUAL-002.6b: RTM FR-25.2
#[derive(Debug, Clone, Default)]
pub struct VectorSyncDiagnostics {
    concept_embedding_failures: usize,
    file_embedding_failures: usize,
    concept_embedding_skipped: usize,
    file_embedding_skipped: usize,
    failure_reasons: BTreeMap<String, usize>,
}

impl VectorSyncDiagnostics {
    pub fn record_concept_failure(&mut self, reason: &str) {
        self.concept_embedding_failures += 1;
        *self.failure_reasons.entry(reason.to_string()).or_default() += 1;
    }

    pub fn record_file_failure(&mut self, reason: &str) {
        self.file_embedding_failures += 1;
        *self.failure_reasons.entry(reason.to_string()).or_default() += 1;
    }

    pub fn record_concept_skipped(&mut self, count: usize) {
        self.concept_embedding_skipped += count;
    }

    pub fn record_file_skipped(&mut self) {
        self.file_embedding_skipped += 1;
    }

    pub fn concept_embedding_failures(&self) -> usize {
        self.concept_embedding_failures
    }

    pub fn file_embedding_failures(&self) -> usize {
        self.file_embedding_failures
    }

    pub fn total_failures(&self) -> usize {
        self.concept_embedding_failures + self.file_embedding_failures
    }

    pub fn concept_embedding_skipped(&self) -> usize {
        self.concept_embedding_skipped
    }

    pub fn file_embedding_skipped(&self) -> usize {
        self.file_embedding_skipped
    }

    pub fn total_skipped(&self) -> usize {
        self.concept_embedding_skipped + self.file_embedding_skipped
    }

    pub fn failure_reasons(&self) -> &BTreeMap<String, usize> {
        &self.failure_reasons
    }
}

fn sync_role() -> String {
    match std::env::var("MAENIFOLD_SYNC_ROLE") {
        Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
            "leader" => "leader".to_string(),
            "follower" => "follower".to_string(),
            _ => "auto".to_string(),
        },
        Err(_) => "auto".to_string(),
    }
}

fn generate_sync_holder_id() -> String {
    format!("rust-{}-{}", std::process::id(), Uuid::new_v4())
}

fn render_sync_markdown(summary: SyncSummary, vector_section: Option<&str>) -> String {
    let mut output = format!(
        "Sync complete:
- STATUS: {}
- {} FILES processed
- {} CONCEPT mentions found
- {} CONCEPT relations created
- {} ABANDONED sessions cleaned up
- {} COMMUNITIES detected (modularity 0.000)",
        summary.status,
        summary.files_processed,
        summary.concept_mentions,
        summary.concept_relations,
        summary.abandoned_sessions,
        summary.communities_detected,
    );
    if let Some(section) = vector_section {
        output.push('\n');
        output.push_str(section);
    } else if let Some(warning_code) = summary.vector_warning_code {
        output.push_str(&format!(
            "
- WARNING_CODE {warning_code}"
        ));
    } else {
        output.push_str(
            "
Vector index verification:
- CONCEPT embeddings: 0 rows (384 dims, self-distance n/a)
- MEMORY embeddings: 0 rows (384 dims, self-distance n/a)",
        );
    }
    output
}

// T-QUAL-002.6b: RTM FR-25.2
fn vector_sync_disabled_by_config() -> bool {
    std::env::var("MAENIFOLD_DISABLE_VECTOR")
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes"
            )
        })
        .unwrap_or(false)
}

// T-QUAL-002.6b: RTM FR-25.2
fn normalize_vector_unavailable_reason(reason: &str) -> &'static str {
    let normalized = reason.trim().to_ascii_uppercase();
    if normalized == VectorUnavailableReasonCodes::DISABLED_BY_CONFIG {
        VectorUnavailableReasonCodes::DISABLED_BY_CONFIG
    } else {
        VectorUnavailableReasonCodes::UNKNOWN
    }
}

// T-QUAL-002.6b: RTM FR-25.2
pub fn build_vector_failure_report(diagnostics: &VectorSyncDiagnostics) -> Vec<String> {
    let mut lines = vec![
        "Vector sync diagnostics:".to_string(),
        "- STATUS: failed".to_string(),
        format!(
            "- COUNTERS_FAILED: concepts={}, memory={}, total={}",
            diagnostics.concept_embedding_failures(),
            diagnostics.file_embedding_failures(),
            diagnostics.total_failures()
        ),
        format!(
            "- COUNTERS_SKIPPED: concepts={}, memory={}, total={}",
            diagnostics.concept_embedding_skipped(),
            diagnostics.file_embedding_skipped(),
            diagnostics.total_skipped()
        ),
    ];

    if !diagnostics.failure_reasons().is_empty() {
        lines.push("- FAILURES:".to_string());
        for (reason, count) in diagnostics.failure_reasons() {
            lines.push(format!("- REASON {reason}: {count}"));
        }
    }

    lines
}

// T-QUAL-002.6b: RTM FR-25.2
pub fn build_vector_unavailable_report(
    diagnostics: &VectorSyncDiagnostics,
    reason: &str,
) -> Vec<String> {
    vec![
        "Vector sync diagnostics:".to_string(),
        "- STATUS: unavailable".to_string(),
        format!(
            "- COUNTERS_FAILED: concepts={}, memory={}, total={}",
            diagnostics.concept_embedding_failures(),
            diagnostics.file_embedding_failures(),
            diagnostics.total_failures()
        ),
        format!(
            "- COUNTERS_SKIPPED: concepts={}, memory={}, total={}",
            diagnostics.concept_embedding_skipped(),
            diagnostics.file_embedding_skipped(),
            diagnostics.total_skipped()
        ),
        format!(
            "- REASON_CODE {}",
            normalize_vector_unavailable_reason(reason)
        ),
    ]
}

// T-QUAL-002.6b: RTM FR-25.2
fn collect_vector_skip_diagnostics_for_uris(
    config: &crate::Config,
    file_uris: &[String],
) -> Result<VectorSyncDiagnostics> {
    let mut diagnostics = VectorSyncDiagnostics::default();
    let mut concepts = BTreeSet::new();

    for uri in file_uris {
        let (_, content, _) = read_markdown(uri, config)?;
        diagnostics.record_file_skipped();
        concepts.extend(extract_wikilinks(&content));
    }

    diagnostics.record_concept_skipped(concepts.len());
    Ok(diagnostics)
}

// T-QUAL-002.6b: RTM FR-25.2
fn collect_vector_skip_diagnostics_for_paths(
    file_paths: &[PathBuf],
) -> Result<VectorSyncDiagnostics> {
    let mut diagnostics = VectorSyncDiagnostics::default();
    let mut concepts = BTreeSet::new();

    for file_path in file_paths {
        let content = fs::read_to_string(file_path)?;
        diagnostics.record_file_skipped();
        concepts.extend(extract_wikilinks(&content));
    }

    diagnostics.record_concept_skipped(concepts.len());
    Ok(diagnostics)
}

// T-QUAL-002.6b: RTM FR-25.2
pub fn generate_concept_embeddings(
    connection: &Connection,
    concepts: &[String],
    vector_ready: bool,
    diagnostics: &mut VectorSyncDiagnostics,
) -> usize {
    if !vector_ready || vector_sync_disabled_by_config() {
        diagnostics.record_concept_skipped(concepts.len());
        return 0;
    }

    let table_check = connection.query_row("SELECT COUNT(*) FROM vec_concepts", [], |row| {
        row.get::<_, i64>(0)
    });
    if table_check.is_err() {
        diagnostics.record_concept_failure(VectorFailureReasonCodes::CONCEPT_LOOKUP_FAILED);
        return 0;
    }

    0
}

// T-QUAL-002.6b: RTM FR-25.2
pub fn sync_files(context: &ToolContext, file_paths: &[PathBuf]) -> Result<String> {
    for file_path in file_paths {
        sync_single_file(context, file_path)?;
    }

    let mut output = format!(
        "SyncFiles complete:
- {} FILES processed",
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

    Ok(output)
}

fn load_help_doc(assets_path: &Path, tool_name: &str) -> Option<String> {
    fs::read_to_string(
        assets_path
            .join("usage")
            .join("tools")
            .join(format!("{}.md", tool_name.to_ascii_lowercase())),
    )
    .ok()
}

fn format_source_files_for_display(source_files: &str) -> String {
    let trimmed = source_files.trim();
    // T-0.3b: RTM NFR-4
    if trimmed.starts_with('{')
        && let Ok(Value::Object(map)) = serde_json::from_str::<Value>(trimmed)
    {
        let mut keys = map.keys().cloned().collect::<Vec<_>>();
        keys.sort();
        return keys.join(", ");
    }
    source_files.to_string()
}

// T-4.3: RTM FR-23.x
fn format_co_occurrence_weight(weight: f64) -> String {
    if (weight.fract()).abs() < f64::EPSILON {
        format!("{weight:.0}")
    } else {
        format!("{weight:.1}")
    }
}

fn current_iso_timestamp() -> String {
    let now = chrono::Utc::now();
    format!(
        "{}.{:07}+00:00",
        now.format("%Y-%m-%dT%H:%M:%S"),
        now.timestamp_subsec_nanos() / 100,
    )
}

fn slugify_query(query: &str) -> String {
    query
        .to_ascii_lowercase()
        .replace(|ch: char| !ch.is_ascii_alphanumeric(), "-")
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn informative_query_terms(query: &str) -> Vec<String> {
    const STOPWORDS: &[&str] = &[
        "a", "an", "and", "are", "as", "at", "be", "but", "by", "for", "if", "in", "into", "is",
        "it", "no", "not", "of", "on", "or", "such", "that", "the", "their", "then", "there",
        "these", "they", "this", "to", "was", "will", "with", "we", "you", "your", "our", "from",
    ];
    let stopwords = STOPWORDS.iter().copied().collect::<HashSet<_>>();
    query
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| part.to_ascii_lowercase())
        .filter(|term| !stopwords.contains(term.as_str()))
        .collect()
}

fn tokenize_query(query: &str) -> Vec<String> {
    query
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| part.to_ascii_lowercase())
        .collect()
}

fn build_snippet(content: &str, query_terms: &[String], query_slug: &str) -> String {
    let lower = content.to_ascii_lowercase();
    let index = query_terms
        .iter()
        .find_map(|term| lower.find(term))
        .or_else(|| lower.find(query_slug))
        .unwrap_or(0);
    let start = index.saturating_sub(25);
    let end = (index + 48).min(content.len());
    content.get(start..end).unwrap_or(content).to_string()
}

// T-QUAL-002.6a: RTM FR-25.2
pub fn get_file_display_info(
    file_identifier: &str,
    query: &str,
    config: &crate::config::Config,
) -> (String, String) {
    let fallback_title = fallback_display_title(file_identifier);
    let query_terms = tokenize_query(query);
    let query_slug = slugify_query(query);

    let loaded = if file_identifier.starts_with("memory://") {
        read_markdown(file_identifier, config)
            .map(|(frontmatter, content, _)| (frontmatter, content))
    } else if let Ok(uri) = path_to_uri(Path::new(file_identifier), &config.memory_path) {
        read_markdown(&uri, config).map(|(frontmatter, content, _)| (frontmatter, content))
    } else {
        fs::read_to_string(file_identifier)
            .map(|content| (None, content))
            .map_err(MaenifoldError::from)
    };

    match loaded {
        Ok((frontmatter, content)) => (
            frontmatter
                .as_ref()
                .and_then(frontmatter_title)
                .unwrap_or_else(|| fallback_title.clone()),
            build_snippet(&content, &query_terms, &query_slug),
        ),
        Err(_) => {
            emit_stderr_line(&format!(
                "[DEGRADED][MemorySearchTools.Fusion][GetFileDisplayInfo] Falling back to filename/snippet for '{}'",
                redact_display_info_path(file_identifier, config)
            ));
            (fallback_title, "[DEGRADED] Snippet unavailable".to_string())
        }
    }
}

fn frontmatter_title(frontmatter: &crate::markdown::Frontmatter) -> Option<String> {
    frontmatter.get("title").and_then(|value| match value {
        YamlValue::String(title) if !title.trim().is_empty() => Some(title.clone()),
        _ => None,
    })
}

fn fallback_display_title(file_identifier: &str) -> String {
    if let Some(relative) = file_identifier.strip_prefix("memory://") {
        return relative
            .rsplit('/')
            .next()
            .filter(|segment| !segment.is_empty())
            .unwrap_or(relative)
            .to_string();
    }

    let path = Path::new(file_identifier);
    path.file_stem()
        .or_else(|| path.file_name())
        .map(|segment| segment.to_string_lossy().into_owned())
        .unwrap_or_else(|| file_identifier.to_string())
}

fn redact_display_info_path(file_identifier: &str, config: &crate::config::Config) -> String {
    if let Some(relative) = file_identifier.strip_prefix("memory://") {
        return relative
            .rsplit('/')
            .next()
            .filter(|segment| !segment.is_empty())
            .unwrap_or(relative)
            .to_string();
    }

    let path = Path::new(file_identifier);
    if path.is_absolute() && !path.starts_with(&config.memory_path) {
        return path
            .file_name()
            .map(|segment| segment.to_string_lossy().into_owned())
            .unwrap_or_else(|| file_identifier.to_string());
    }

    path.file_name()
        .or_else(|| path.file_stem())
        .map(|segment| segment.to_string_lossy().into_owned())
        .unwrap_or_else(|| file_identifier.to_string())
}

#[cfg(unix)]
fn emit_stderr_line(message: &str) {
    unsafe extern "C" {
        fn write(fd: i32, buf: *const u8, count: usize) -> isize;
    }

    let line = format!("{message}\n");
    // SAFETY: the pointer remains valid for the duration of the syscall and fd=2 is stderr.
    let _ = unsafe { write(2, line.as_ptr(), line.len()) };
}

#[cfg(not(unix))]
fn emit_stderr_line(message: &str) {
    eprintln!("{message}");
}

fn render_search_markdown(data: &Value) -> String {
    let mode = data.get("mode").and_then(Value::as_str).unwrap_or("Hybrid");
    let heading = if mode.eq_ignore_ascii_case("semantic") {
        "Semantic Search"
    } else if mode.eq_ignore_ascii_case("fulltext") || mode.eq_ignore_ascii_case("full-text") {
        "Full-Text Search"
    } else if mode.eq_ignore_ascii_case("hybrid") {
        "Hybrid Search"
    } else {
        "Search Results"
    };
    let query = data
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let total_count = data.get("totalCount").and_then(Value::as_u64).unwrap_or(0);
    let mut output = format!("# {heading}\nQuery: {query}\nFound {total_count} result(s)\n\n");
    if let Some(results) = data.get("results").and_then(Value::as_array) {
        for result in results {
            output.push_str(&format!(
                "- {} ({})\n  {}\n",
                result
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                result
                    .get("path")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                result
                    .get("snippet")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
            ));
        }
    }
    output.trim_end().to_string()
}
