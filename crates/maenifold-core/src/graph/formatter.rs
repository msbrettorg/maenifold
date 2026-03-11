use super::{
    CommunitySiblingLine, GraphRelation, RelatedConceptLine, emit_graph_tools_diagnostic,
    format_co_occurrence_weight, load_concept_community_id, relation_weighted_score,
};
use crate::config::Config;
use crate::graph::preview;
use crate::util::json::success;
use rusqlite::Connection;
use serde_json::{Value, json};

pub(crate) fn render_build_context_json(
    config: &Config,
    connection: &Connection,
    concept_name: &str,
    depth: usize,
    direct_relations: &[GraphRelation],
    related_concepts: &[RelatedConceptLine],
    query_community_id: Option<i64>,
    community_siblings: &[CommunitySiblingLine],
    include_content: bool,
    include_community_data: bool,
) -> String {
    let direct_relations_json = direct_relations
        .iter()
        .map(|relation| {
            render_relation_json(
                config,
                connection,
                concept_name,
                relation,
                include_content,
                if include_community_data {
                    load_concept_community_id(connection, &relation.name)
                        .ok()
                        .flatten()
                } else {
                    None
                },
            )
        })
        .collect::<Vec<_>>();
    // T-15.6: RTM FR-15 — C# ExpandedRelations is List<string> (concept names only),
    // not rich objects. Emit a flat string array to match the API contract.
    let expanded_relations_json: Vec<&str> = related_concepts
        .iter()
        .map(|relation| relation.name.as_str())
        .collect();
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

    success(json!({
        "ConceptName": concept_name,
        "Depth": depth,
        "DirectRelations": direct_relations_json,
        "ExpandedRelations": expanded_relations_json,
        "CommunityId": query_community_id,
        "CommunitySiblings": community_siblings_json,
    }))
}

pub(crate) fn format_empty_build_context(concept_name: &str, depth: usize) -> String {
    let mut output =
        format!("Context for CONCEPT: {concept_name}\n\nDirect relations (0 CONCEPTS):");
    if depth > 1 {
        output.push_str(&format!("\n\nExpanded relations ({depth} hops):"));
    }
    output
}

// T-PARITY-B01: RTM NFR-1 — BuildContext markdown matches C# BuildContextResult.ToString()
pub(crate) fn format_build_context_markdown(
    concept_name: &str,
    depth: usize,
    direct_relations: &[RelatedConceptLine],
    related_concepts: &[RelatedConceptLine],
    community_siblings: &[CommunitySiblingLine],
    query_community_id: Option<i64>,
) -> String {
    // T-PARITY-B01: Header matches C# "Context for CONCEPT: {ConceptName}"
    let mut output = format!("Context for CONCEPT: {concept_name}\n\n");

    // T-PARITY-B01: Direct relations section — rich format with co-occurrence + files
    output.push_str(&format!(
        "Direct relations ({} CONCEPTS):\n",
        direct_relations.len()
    ));
    for relation in direct_relations {
        // T-PARITY-B01: Label matches C# "(co-occurs in N files)"
        output.push_str(&format!(
            "  • {} (co-occurs in {} files)",
            relation.name,
            format_co_occurrence_weight(relation.co_occurrence_count)
        ));
        if let Some(community_id) = relation.community_id {
            output.push_str(&format!(" [community {community_id}]"));
        }
        output.push('\n');
        if !relation.source_files.is_empty() {
            // T-PARITY-B01: File list truncated to first 3, matching C# .Take(3)
            output.push_str(&format!(
                "    Files: {}\n",
                format_source_files_truncated(&relation.source_files, 3)
            ));
        }
    }

    // T-PARITY-RES003: Expanded relations section — concept names only, up to 20, depth > 1
    if depth > 1 {
        output.push_str(&format!("\nExpanded relations ({depth} hops):\n"));
        for concept in related_concepts.iter().take(20) {
            output.push_str(&format!("  • {}\n", concept.name));
        }
    }

    // T-PARITY-B01: Community siblings section matches C# format
    if !community_siblings.is_empty() {
        let community_label = query_community_id
            .map(|id| format!(" in community {id}"))
            .unwrap_or_default();
        output.push_str(&format!(
            "\nCommunity siblings ({} concepts{community_label}):\n",
            community_siblings.len()
        ));
        for sibling in community_siblings {
            output.push_str(&format!(
                "  • {} (overlap: {:.3}) [community {}]\n",
                sibling.name, sibling.normalized_overlap, sibling.community_id
            ));
        }
    }

    output.trim_end().to_string()
}

// T-PARITY-B01: RTM NFR-1 — Truncate source file list to first N entries, matching C# .Take(3)
fn format_source_files_truncated(source_files: &str, max: usize) -> String {
    let trimmed = source_files.trim();
    if trimmed.starts_with('{') {
        if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(trimmed) {
            let mut keys = map.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            return keys.into_iter().take(max).collect::<Vec<_>>().join(", ");
        }
    }
    // Fallback: comma-separated string, take first max entries
    source_files
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .take(max)
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_relation_json(
    config: &Config,
    connection: &Connection,
    concept_name: &str,
    relation: &GraphRelation,
    include_content: bool,
    community_id: Option<i64>,
) -> Value {
    let content_preview = load_content_preview(
        connection,
        concept_name,
        &relation.source_files,
        include_content,
    );
    let (decay_weight, weighted_score) = relation_weighted_score(
        connection,
        config,
        relation.co_occurrence_count,
        &relation.source_files,
    )
    .unwrap_or((1.0, relation.co_occurrence_count));
    json!({
        "Name": relation.name,
        "CoOccurrenceCount": relation.co_occurrence_count,
        "Files": preview::parse_source_files_for_json(&relation.source_files),
        "ContentPreview": content_preview,
        // T-GRAPH-DECAY-001.1: RTM FR-7.5
        "DecayWeight": decay_weight,
        "WeightedScore": weighted_score,
        "CommunityId": community_id,
    })
}

fn load_content_preview(
    connection: &Connection,
    concept_name: &str,
    source_files: &str,
    include_content: bool,
) -> Value {
    if !include_content {
        return json!({});
    }

    match preview::load_content_preview_map(connection, concept_name, source_files) {
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
}
