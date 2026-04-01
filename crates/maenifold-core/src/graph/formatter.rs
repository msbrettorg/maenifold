use super::{
    CommunitySiblingLine, GraphRelation, RelatedConceptLine, emit_graph_tools_diagnostic,
    format_co_occurrence_weight, load_concept_community_id, relation_weighted_score,
};
use crate::config::Config;
use crate::graph::preview;
use crate::util::json::success;
use rusqlite::Connection;
use serde_json::{Value, json};

pub(crate) struct BuildContextJsonArgs<'a> {
    pub(crate) config: &'a Config,
    pub(crate) connection: &'a Connection,
    pub(crate) concept_name: &'a str,
    pub(crate) depth: usize,
    pub(crate) direct_relations: &'a [GraphRelation],
    pub(crate) related_concepts: &'a [RelatedConceptLine],
    pub(crate) query_community_id: Option<i64>,
    pub(crate) community_siblings: &'a [CommunitySiblingLine],
    pub(crate) include_content: bool,
    pub(crate) include_community_data: bool,
}

pub(crate) fn render_build_context_json(args: BuildContextJsonArgs<'_>) -> String {
    let direct_relations_json = args
        .direct_relations
        .iter()
        .map(|relation| {
            render_relation_json(RelationJsonArgs {
                config: args.config,
                connection: args.connection,
                concept_name: args.concept_name,
                relation,
                include_content: args.include_content,
                community_id: if args.include_community_data {
                    load_concept_community_id(args.connection, &relation.name)
                        .ok()
                        .flatten()
                } else {
                    None
                },
            })
        })
        .collect::<Vec<_>>();
    // T-15.6: RTM FR-15 — C# ExpandedRelations is List<string> (concept names only),
    // not rich objects. Emit a flat string array to match the API contract.
    let expanded_relations_json: Vec<&str> = args
        .related_concepts
        .iter()
        .map(|relation| relation.name.as_str())
        .collect();
    let community_siblings_json = args
        .community_siblings
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
        "ConceptName": args.concept_name,
        "Depth": args.depth,
        "DirectRelations": direct_relations_json,
        "ExpandedRelations": expanded_relations_json,
        "CommunityId": args.query_community_id,
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
pub(crate) struct BuildContextMarkdownArgs<'a> {
    pub(crate) concept_name: &'a str,
    pub(crate) depth: usize,
    pub(crate) direct_relations: &'a [RelatedConceptLine],
    pub(crate) related_concepts: &'a [RelatedConceptLine],
    pub(crate) community_siblings: &'a [CommunitySiblingLine],
    pub(crate) query_community_id: Option<i64>,
}

pub(crate) fn format_build_context_markdown(args: BuildContextMarkdownArgs<'_>) -> String {
    // T-PARITY-B01: Header matches C# "Context for CONCEPT: {ConceptName}"
    let mut output = format!("Context for CONCEPT: {}\n\n", args.concept_name);

    // T-PARITY-B01: Direct relations section — rich format with co-occurrence + files
    output.push_str(&format!(
        "Direct relations ({} CONCEPTS):\n",
        args.direct_relations.len()
    ));
    for relation in args.direct_relations {
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
    if args.depth > 1 {
        output.push_str(&format!("\nExpanded relations ({} hops):\n", args.depth));
        for concept in args.related_concepts.iter().take(20) {
            output.push_str(&format!("  • {}\n", concept.name));
        }
    }

    // T-PARITY-B01: Community siblings section matches C# format
    if !args.community_siblings.is_empty() {
        let community_label = args
            .query_community_id
            .map(|id| format!(" in community {id}"))
            .unwrap_or_default();
        output.push_str(&format!(
            "\nCommunity siblings ({} concepts{community_label}):\n",
            args.community_siblings.len()
        ));
        for sibling in args.community_siblings {
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
    if trimmed.starts_with('{')
        && let Ok(Value::Object(map)) = serde_json::from_str::<Value>(trimmed)
    {
        let mut keys = map.keys().cloned().collect::<Vec<_>>();
        keys.sort();
        return keys.into_iter().take(max).collect::<Vec<_>>().join(", ");
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

#[allow(clippy::too_many_arguments)]
struct RelationJsonArgs<'a> {
    config: &'a Config,
    connection: &'a Connection,
    concept_name: &'a str,
    relation: &'a GraphRelation,
    include_content: bool,
    community_id: Option<i64>,
}

fn render_relation_json(args: RelationJsonArgs<'_>) -> Value {
    let content_preview = load_content_preview(
        args.connection,
        args.concept_name,
        &args.relation.source_files,
        args.include_content,
    );
    let (decay_weight, weighted_score) = relation_weighted_score(
        args.connection,
        args.config,
        args.relation.co_occurrence_count,
        &args.relation.source_files,
    )
    .unwrap_or((1.0, args.relation.co_occurrence_count));
    json!({
        "Name": args.relation.name,
        "CoOccurrenceCount": args.relation.co_occurrence_count,
        "Files": preview::parse_source_files_for_json(&args.relation.source_files),
        "ContentPreview": content_preview,
        // T-GRAPH-DECAY-001.1: RTM FR-7.5
        "DecayWeight": decay_weight,
        "WeightedScore": weighted_score,
        "CommunityId": args.community_id,
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
