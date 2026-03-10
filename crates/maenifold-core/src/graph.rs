use crate::config::Config;
use crate::db::{self, GraphDatabase};
use crate::error::{MaenifoldError, Result};
use crate::json::{OutputFormat, success};
use crate::markdown::normalize_concept;
use crate::memory::compute_decay_weight;
use crate::search::{
    embedding_is_valid_for_similarity_search, is_vec_embedding_unavailable,
    load_concept_embedding, split_frontmatter_for_repair, vec0_extension_path,
};
use crate::tools::ToolContext;
use crate::vector;
use regex::Regex;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::Path;
use uuid::Uuid;

// T-11.7: RTM FR-13 preserves graph tool handler parity while extracting traversal logic from search.rs.
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

    if let Err(error) = GraphDatabase::initialize_database(&context.config) {
        let message = error.to_string();
        // T-0.1: RTM FR-25.2 preserves BuildContext availability when community enrichment schema is degraded.
        if message.contains("concept_communities") || message.contains("community_id") {
            emit_graph_tools_diagnostic(
                "EnrichWithCommunityData",
                "Community enrichment schema is degraded; continuing without community data",
                &message,
            );
        } else {
            return Err(error);
        }
    }
    let connection = db::open_connection(&context.config.database_path)?;

    // T-3.2b: RTM FR-13.10
    let query_community_id = match load_concept_community_id(&connection, &concept_name) {
        Ok(value) => value,
        Err(error) => {
            // T-0.1: RTM FR-25.2 preserves degraded diagnostics when community enrichment storage is unavailable.
            emit_graph_tools_diagnostic(
                "EnrichWithCommunityData",
                "Community enrichment unavailable; continuing without community data",
                &error.to_string(),
            );
            None
        }
    };
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
                    match load_content_preview_map(
                        &connection,
                        &concept_name,
                        &relation.source_files,
                    ) {
                        Ok(preview) => preview,
                        Err(error) => {
                            emit_graph_tools_diagnostic(
                                "BuildContext",
                                "Content preview unavailable; continuing without preview content",
                                &error.to_string(),
                            );
                            json!({})
                        }
                    }
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
                    match load_content_preview_map(
                        &connection,
                        &relation.name,
                        &relation.source_files,
                    ) {
                        Ok(preview) => preview,
                        Err(error) => {
                            emit_graph_tools_diagnostic(
                                "BuildContext",
                                "Content preview unavailable; continuing without preview content",
                                &error.to_string(),
                            );
                            json!({})
                        }
                    }
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
    config: &Config,
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

fn load_file_decay_weight(connection: &Connection, config: &Config, uri: &str) -> Result<f64> {
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
    config: &Config,
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
            .map(|path| compute_decay_weight(path, last_accessed.as_deref(), config, status.as_deref()))
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

pub(crate) fn emit_graph_tools_diagnostic(method: &str, message: &str, detail: &str) {
    emit_graph_tools_stderr_line(&format!(
        "[DEGRADED][GraphTools][{method}] {message}: {detail}"
    ));
}

#[cfg(unix)]
fn emit_graph_tools_stderr_line(message: &str) {
    unsafe extern "C" {
        fn write(fd: i32, buf: *const u8, count: usize) -> isize;
    }

    let mut line = String::with_capacity(message.len() + 1);
    line.push_str(message);
    line.push('\n');
    // SAFETY: fd=2 is the process stderr and the buffer is valid for the call duration.
    unsafe {
        let _ = write(2, line.as_ptr(), line.len());
    }
}

#[cfg(not(unix))]
fn emit_graph_tools_stderr_line(message: &str) {
    eprintln!("{message}");
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

#[derive(Debug, Clone)]
struct SimilarConceptMatch {
    concept: String,
    distance: f64,
    similarity: f64,
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
    let original_concept_name = concept_name.to_string();
    let low_info_query = concept_name.chars().count() < 2;
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
        let plateau_warning = similarity_plateau_warning(&original_concept_name, &vector_results);

        // T-GRAPH-DECAY-001.1: RTM FR-7.5
        let mut ranked_results = vector_results
            .into_iter()
            .map(|result| {
                let neighbor = result.concept;
                let similarity = result.similarity;
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
        if let Some(warning) = plateau_warning {
            // T-11.6: RTM FR-13 surfaces collapsed vec0 distance plateaus without hiding results.
            output.push_str(&format!("\n⚠️ {warning}\n"));
        }
        if low_info_query {
            output.push_str(&format!(
                "\n⚠️ conceptName '{}' is very short; similarity results may be noisy.\n",
                original_concept_name
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
    if low_info_query {
        // T-11.6: RTM FR-13 warns when fallback similarity is driven by a very short query.
        output.push_str(&format!(
            "\n⚠️ conceptName '{}' is very short; similarity results may be noisy.\n",
            original_concept_name
        ));
    }
    output.push_str(
        "
💡 Next: Consider using BuildContext, SearchMemories for related operations.",
    );
    Ok(output)
}

fn query_vec_similar_concepts(
    connection: &Connection,
    assets_path: &Path,
    concept_name: &str,
    max_results: usize,
) -> Result<Option<Vec<SimilarConceptMatch>>> {
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

    // T-8.3: RTM FR-33.2 — FindSimilarConcepts can probe [[embedding]] similarity even when
    // the query concept is not yet persisted in vec_concepts.
    let query_blob = match load_concept_embedding(connection, concept_name) {
        Ok(Some(embedding))
            if !embedding.is_empty() && embedding_is_valid_for_similarity_search(&embedding) =>
        {
            vector::to_sqlite_vector_blob(&embedding)
        }
        Ok(Some(_)) => {
            return Err(MaenifoldError::InvalidArgument(
                "unable to generate a valid embedding for conceptName".to_string(),
            ));
        }
        // T-8.5a: RTM FR-7.4 / FR-33.2 — on-the-fly query embedding failures must degrade
        // to graph-based fallback instead of crashing FindSimilarConcepts.
        Ok(_) => match vector::generate_embedding(concept_name) {
            // T-11.6: RTM FR-13 rejects near-zero embeddings before vec0 distance queries.
            Ok(embedding) if embedding_is_valid_for_similarity_search(&embedding) => {
                vector::to_sqlite_vector_blob(&embedding)
            }
            Ok(_) => {
                return Err(MaenifoldError::InvalidArgument(
                    "unable to generate a valid embedding for conceptName".to_string(),
                ));
            }
            Err(_) => return Ok(None),
        },
        Err(error) if is_vec_embedding_unavailable(&error) => return Ok(None),
        Err(error) => return Err(error),
    };

    let mut statement = connection.prepare(
        "SELECT concept_name, vec_distance_cosine(embedding, ?1) as distance          FROM vec_concepts          WHERE concept_name != ?2            AND concept_name != ''            AND instr(concept_name, '[') = 0            AND instr(concept_name, ']') = 0          ORDER BY distance          LIMIT ?3",
    )?;
    let rows = statement.query_map(
        params![query_blob, concept_name, max_results as i64],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?)),
    )?;

    let mut results = Vec::new();
    for row in rows {
        let (neighbor, distance) = row?;
        if !distance.is_finite() {
            continue;
        }

        // T-QUAL-FSC2: RTM FR-7.4
        // Clamp negative-distance drift so semantic similarity never exceeds 1.000.
        let similarity = (1.0 / (1.0 + distance.max(0.0))).clamp(0.0, 1.0);
        results.push(SimilarConceptMatch {
            concept: neighbor,
            distance,
            similarity,
        });
    }
    Ok(Some(results))
}

fn similarity_plateau_warning(query: &str, results: &[SimilarConceptMatch]) -> Option<String> {
    if results.len() < 2 {
        return None;
    }

    let mean = results.iter().map(|result| result.distance).sum::<f64>() / results.len() as f64;
    let variance = results
        .iter()
        .map(|result| {
            let delta = result.distance - mean;
            delta * delta
        })
        .sum::<f64>()
        / results.len() as f64;

    (variance <= 1e-9).then(|| {
        format!(
            "similarity plateau detected for '{}'; results may be degraded by collapsed embedding distances.",
            query
        )
    })
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
