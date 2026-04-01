use super::{
    GraphRelation, RelatedConceptLine, emit_graph_tools_diagnostic, formatter,
    load_community_siblings, load_concept_community_id, load_graph_neighbors, load_help_doc,
    relation_weighted_score,
};
use crate::config::Config;
use crate::db::{self, GraphDatabase};
use crate::error::{MaenifoldError, Result};
use crate::markdown::normalize_concept;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use rusqlite::Connection;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

// T-12.0: RTM NFR-1 keeps BuildContext behavior stable while the graph facade becomes a directory module.
#[allow(clippy::too_many_lines)]
pub fn build_context(context: &ToolContext, payload: &Value) -> Result<String> {
    let learn = payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if learn {
        let help = load_help_doc(&context.config.assets_path, "BuildContext")
            .unwrap_or_else(|| "ERROR: Help file not found for BuildContext".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => crate::util::json::success(help),
            OutputFormat::Markdown => help,
        });
    }

    let concept_name = normalize_concept(
        payload
            .get("conceptName")
            .and_then(Value::as_str)
            .ok_or(MaenifoldError::MissingProperty("conceptName"))?,
    );
    let depth = parse_non_negative(payload, "depth")?.unwrap_or(2);
    let max_entities = parse_non_negative(payload, "maxEntities")?.unwrap_or(20);
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
    let query_community_id = load_query_community_id(&connection, &concept_name)?;
    let include_community_data = query_community_id.is_some();

    let mut direct_relations = load_graph_neighbors(&connection, &concept_name)?;
    sort_relations_by_weighted_score(&connection, &context.config, &mut direct_relations);
    let direct_relation_names = direct_relations
        .iter()
        .map(|relation| relation.name.clone())
        .collect::<HashSet<_>>();

    let related_concepts = expand_related_concepts(ExpandRelatedConceptsArgs {
        connection: &connection,
        config: &context.config,
        concept_name: &concept_name,
        depth,
        max_entities,
        include_community_data,
    })?;
    let community_siblings = if include_community_data && depth > 0 {
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
        return Ok(formatter::render_build_context_json(
            formatter::BuildContextJsonArgs {
                config: &context.config,
                connection: &connection,
                concept_name: &concept_name,
                depth,
                direct_relations: &direct_relations,
                related_concepts: &related_concepts,
                query_community_id,
                community_siblings: &community_siblings,
                include_content,
                include_community_data,
            },
        ));
    }

    if !include_community_data && direct_relations.is_empty() && related_concepts.is_empty() {
        // T-0.2: RTM NFR-3 - preserve the C# empty/not-found markdown contract.
        return Ok(formatter::format_empty_build_context(&concept_name, depth));
    }

    // T-PARITY-B01: RTM NFR-1 — Convert GraphRelations to RelatedConceptLine for markdown rendering
    // T-PARITY-RES003: RTM NFR-1 — Pass direct_relations to formatter for "Direct relations" section
    let direct_lines: Vec<RelatedConceptLine> = direct_relations
        .iter()
        .map(|relation| RelatedConceptLine {
            name: relation.name.clone(),
            co_occurrence_count: relation.co_occurrence_count,
            source_files: relation.source_files.clone(),
            community_id: if include_community_data {
                load_concept_community_id(&connection, &relation.name)
                    .ok()
                    .flatten()
            } else {
                None
            },
        })
        .collect();

    Ok(formatter::format_build_context_markdown(
        formatter::BuildContextMarkdownArgs {
            concept_name: &concept_name,
            depth,
            direct_relations: &direct_lines,
            related_concepts: &related_concepts,
            community_siblings: &community_siblings,
            query_community_id,
        },
    ))
}

fn parse_non_negative(payload: &Value, property: &'static str) -> Result<Option<usize>> {
    match payload.get(property).and_then(Value::as_i64) {
        Some(value) if value < 0 => Err(MaenifoldError::InvalidArgument(format!(
            "{property} must be zero or greater"
        ))),
        Some(value) => Ok(Some(value as usize)),
        None => Ok(None),
    }
}

fn load_query_community_id(connection: &Connection, concept_name: &str) -> Result<Option<i64>> {
    match load_concept_community_id(connection, concept_name) {
        Ok(value) => Ok(value),
        Err(error) => {
            // T-0.1: RTM FR-25.2 preserves degraded diagnostics when community enrichment storage is unavailable.
            emit_graph_tools_diagnostic(
                "EnrichWithCommunityData",
                "Community enrichment unavailable; continuing without community data",
                &error.to_string(),
            );
            Ok(None)
        }
    }
}

#[allow(clippy::too_many_arguments)]
struct ExpandRelatedConceptsArgs<'a> {
    connection: &'a Connection,
    config: &'a Config,
    concept_name: &'a str,
    depth: usize,
    max_entities: usize,
    include_community_data: bool,
}

fn expand_related_concepts(args: ExpandRelatedConceptsArgs<'_>) -> Result<Vec<RelatedConceptLine>> {
    let mut related_concepts = Vec::new();
    let mut seen_concepts = HashSet::from([args.concept_name.to_string()]);
    let mut current_frontier = vec![args.concept_name.to_string()];

    for _hop_distance in 1..=args.depth {
        if related_concepts.len() >= args.max_entities {
            break;
        }

        let mut next_frontier = Vec::new();
        let mut hop_candidates = BTreeMap::<String, GraphRelation>::new();

        for frontier_concept in &current_frontier {
            for relation in load_graph_neighbors(args.connection, frontier_concept)? {
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
        sort_relations_by_weighted_score(args.connection, args.config, &mut hop_relations);

        for relation in hop_relations {
            if related_concepts.len() >= args.max_entities {
                break;
            }

            let relation_name = relation.name.clone();
            seen_concepts.insert(relation_name.clone());
            next_frontier.push(relation_name.clone());
            related_concepts.push(RelatedConceptLine {
                name: relation_name.clone(),
                co_occurrence_count: relation.co_occurrence_count,
                source_files: relation.source_files,
                community_id: if args.include_community_data {
                    load_concept_community_id(args.connection, &relation_name)?
                } else {
                    None
                },
            });
        }

        current_frontier = next_frontier;
    }

    Ok(related_concepts)
}

fn sort_relations_by_weighted_score(
    connection: &Connection,
    config: &Config,
    relations: &mut [GraphRelation],
) {
    // T-GRAPH-DECAY-001.1: RTM FR-7.5
    relations.sort_by(|left, right| {
        let left_score = relation_weighted_score(
            connection,
            config,
            left.co_occurrence_count,
            &left.source_files,
        )
        .map(|(_, weighted_score)| weighted_score)
        .unwrap_or(left.co_occurrence_count);
        let right_score = relation_weighted_score(
            connection,
            config,
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
}
