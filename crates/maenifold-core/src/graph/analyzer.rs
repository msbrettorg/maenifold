use crate::db::{self, GraphDatabase};
use crate::error::{MaenifoldError, Result};
use crate::markdown::normalize_concept;
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use rusqlite::{Connection, params};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use uuid::Uuid;

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
#[allow(clippy::too_many_lines)]
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
        return Err(MaenifoldError::Message(
            "conceptName is required".to_string(),
        ));
    }

    let depth = payload.get("depth").and_then(Value::as_i64).unwrap_or(2);
    if !(1..=5).contains(&depth) {
        return Err(MaenifoldError::Message(
            "depth must be between 1 and 5".to_string(),
        ));
    }

    let max_nodes = payload
        .get("maxNodes")
        .and_then(Value::as_i64)
        .unwrap_or(30);
    if !(5..=100).contains(&max_nodes) {
        return Err(MaenifoldError::Message(
            "maxNodes must be between 5 and 100".to_string(),
        ));
    }

    let output_mode = match payload
        .get("format")
        .and_then(Value::as_str)
        .unwrap_or("mermaid")
    {
        "mermaid" => VisualizeOutputFormat::Mermaid,
        "graph" => VisualizeOutputFormat::Graph,
        other => {
            return Err(MaenifoldError::Message(format!(
                "format must be one of: mermaid, graph (received '{other}')"
            )));
        }
    };

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

    let subgraph = walk_local_subgraph(&connection, &concept_name, depth, max_nodes)?;

    Ok(match output_mode {
        VisualizeOutputFormat::Mermaid => {
            let mermaid = render_mermaid(&subgraph);
            match context.output_format {
                OutputFormat::Json => success(mermaid),
                OutputFormat::Markdown => mermaid,
            }
        }
        VisualizeOutputFormat::Graph => {
            let graph = GraphVisualization::from_subgraph(&subgraph);
            match context.output_format {
                OutputFormat::Json => success(graph),
                OutputFormat::Markdown => {
                    serde_json::to_string_pretty(&graph).map_err(MaenifoldError::Json)?
                }
            }
        }
    })
}

fn walk_local_subgraph(
    connection: &Connection,
    concept_name: &str,
    depth: i64,
    max_nodes: i64,
) -> Result<LocalSubgraph> {
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
         SELECT DISTINCT depth, from_concept, to_concept, count \
         FROM search \
         ORDER BY count DESC, from_concept ASC, to_concept ASC, depth ASC \
         LIMIT ?3",
    )?;
    let rows = statement.query_map(params![concept_name, depth, max_nodes], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, f64>(3)?,
        ))
    })?;

    let relations = rows.collect::<std::result::Result<Vec<_>, _>>()?;
    let mut edges = Vec::new();
    let mut seen_edges = HashSet::new();
    let mut nodes = vec![LocalSubgraphNode {
        concept: concept_name.to_string(),
        depth: 0,
    }];
    let mut seen_nodes = HashSet::from([concept_name.to_string()]);

    for (relation_depth, from_concept, to_concept, count) in relations {
        let edge_key = (from_concept.clone(), to_concept.clone(), count.to_bits());
        if seen_edges.insert(edge_key) {
            edges.push(LocalSubgraphEdge {
                source: from_concept.clone(),
                target: to_concept.clone(),
                weight: count,
            });
        }

        let from_depth = relation_depth.saturating_sub(1) as u32;
        if seen_nodes.insert(from_concept.clone()) {
            nodes.push(LocalSubgraphNode {
                concept: from_concept,
                depth: from_depth,
            });
        } else {
            update_node_depth(&mut nodes, &from_concept, from_depth);
        }

        let to_depth = relation_depth.max(0) as u32;
        if seen_nodes.insert(to_concept.clone()) {
            nodes.push(LocalSubgraphNode {
                concept: to_concept,
                depth: to_depth,
            });
        } else {
            update_node_depth(&mut nodes, &to_concept, to_depth);
        }
    }

    nodes.sort_by(|left, right| {
        left.depth
            .cmp(&right.depth)
            .then_with(|| left.concept.cmp(&right.concept))
    });

    Ok(LocalSubgraph {
        center: concept_name.to_string(),
        nodes,
        edges,
    })
}

fn update_node_depth(nodes: &mut [LocalSubgraphNode], concept: &str, candidate_depth: u32) {
    if let Some(node) = nodes.iter_mut().find(|node| node.concept == concept)
        && candidate_depth < node.depth
    {
        node.depth = candidate_depth;
    }
}

fn render_mermaid(subgraph: &LocalSubgraph) -> String {
    let mut mermaid = String::from("graph TD\n");
    for edge in &subgraph.edges {
        mermaid.push_str(&format!(
            "    {} -->|{}| {}\n",
            sanitize_mermaid_node_id(&edge.source),
            format_co_occurrence_weight(edge.weight),
            sanitize_mermaid_node_id(&edge.target)
        ));
    }
    if subgraph.edges.is_empty() {
        mermaid.push_str(&format!(
            "    {} [No connections found]\n",
            sanitize_mermaid_node_id(&subgraph.center)
        ));
    }
    mermaid
}

#[derive(Debug, Clone, Copy)]
enum VisualizeOutputFormat {
    Mermaid,
    Graph,
}

#[derive(Debug, Clone)]
struct LocalSubgraph {
    center: String,
    nodes: Vec<LocalSubgraphNode>,
    edges: Vec<LocalSubgraphEdge>,
}

#[derive(Debug, Clone)]
struct LocalSubgraphNode {
    concept: String,
    depth: u32,
}

#[derive(Debug, Clone)]
struct LocalSubgraphEdge {
    source: String,
    target: String,
    weight: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GraphVisualization {
    center: String,
    nodes: Vec<GraphVisualizationNode>,
    edges: Vec<GraphVisualizationEdge>,
}

impl GraphVisualization {
    fn from_subgraph(subgraph: &LocalSubgraph) -> Self {
        Self {
            center: subgraph.center.clone(),
            nodes: subgraph
                .nodes
                .iter()
                .map(GraphVisualizationNode::from_subgraph_node)
                .collect(),
            edges: subgraph
                .edges
                .iter()
                .map(GraphVisualizationEdge::from_subgraph_edge)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GraphVisualizationNode {
    id: String,
    label: String,
    depth: u32,
}

impl GraphVisualizationNode {
    fn from_subgraph_node(node: &LocalSubgraphNode) -> Self {
        Self {
            id: node.concept.clone(),
            label: node.concept.clone(),
            depth: node.depth,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GraphVisualizationEdge {
    source: String,
    target: String,
    weight: f64,
}

impl GraphVisualizationEdge {
    fn from_subgraph_edge(edge: &LocalSubgraphEdge) -> Self {
        Self {
            source: edge.source.clone(),
            target: edge.target.clone(),
            weight: edge.weight,
        }
    }
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

fn load_help_doc(assets_path: &Path, tool_name: &str) -> Option<String> {
    fs::read_to_string(
        assets_path
            .join("usage")
            .join("tools")
            .join(format!("{}.md", tool_name.to_ascii_lowercase())),
    )
    .ok()
}

fn format_co_occurrence_weight(weight: f64) -> String {
    if (weight.fract()).abs() < f64::EPSILON {
        format!("{weight:.0}")
    } else {
        format!("{weight:.1}")
    }
}

#[cfg(test)]
mod tests {
    use super::visualize;
    use crate::config::Config;
    use crate::db::{self, GraphDatabase};
    use crate::test_util::ConfigResetGuard;
    use crate::tools::ToolContext;
    use crate::util::json::OutputFormat;
    use serde_json::{Value, json};
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    fn visualize_test_context(
        output_format: OutputFormat,
    ) -> (TempDir, ToolContext, ConfigResetGuard) {
        let tmp = TempDir::new().expect("temp dir");
        Config::override_root(tmp.path());
        Config::set_database_path(tmp.path().join("memory.db"));
        let config = Config::current();
        config.ensure_directories().expect("ensure directories");
        fs::create_dir_all(config.assets_path.join("usage/tools")).expect("create usage/tools");
        fs::write(
            config.assets_path.join("usage/tools/visualize.md"),
            "# Visualize help",
        )
        .expect("write visualize help");
        (
            tmp,
            ToolContext::new(config, output_format),
            ConfigResetGuard,
        )
    }

    fn seed_graph(context: &ToolContext, concepts: &[&str], edges: &[(&str, &str, i64)]) {
        GraphDatabase::initialize_database(&context.config).expect("initialize db");
        let connection = db::open_connection(&context.config.database_path).expect("open db");

        for concept in concepts {
            connection
                .execute(
                    "INSERT OR IGNORE INTO concepts (concept_name, first_seen, occurrence_count) VALUES (?1, '2026-01-01T00:00:00Z', 1)",
                    [*concept],
                )
                .expect("insert concept");
        }

        for (source, target, weight) in edges {
            connection
                .execute(
                    "INSERT OR REPLACE INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files) VALUES (?1, ?2, ?3, '')",
                    rusqlite::params![source, target, weight],
                )
                .expect("insert edge");
        }
    }

    #[test]
    #[serial]
    fn visualize_mermaid_output_stays_stable() {
        let (_tmp, context, _guard) = visualize_test_context(OutputFormat::Markdown);
        seed_graph(
            &context,
            &["machine-learning", "neural-networks", "deep-learning"],
            &[
                ("machine-learning", "neural-networks", 12),
                ("machine-learning", "deep-learning", 8),
            ],
        );

        let result = visualize(
            &context,
            &json!({
                "conceptName": "machine-learning",
                "depth": 1,
                "maxNodes": 30
            }),
        )
        .expect("visualize should succeed");

        assert_eq!(
            result,
            "graph TD\n    machine_learning -->|12| neural_networks\n    machine_learning -->|8| deep_learning\n"
        );
    }

    #[test]
    #[serial]
    fn visualize_graph_format_returns_structured_payload() {
        let (_tmp, context, _guard) = visualize_test_context(OutputFormat::Json);
        seed_graph(
            &context,
            &["machine-learning", "neural-networks", "deep-learning"],
            &[
                ("machine-learning", "neural-networks", 12),
                ("machine-learning", "deep-learning", 8),
            ],
        );

        let result = visualize(
            &context,
            &json!({
                "conceptName": "machine-learning",
                "depth": 1,
                "maxNodes": 30,
                "format": "graph"
            }),
        )
        .expect("visualize should succeed");

        let payload: Value = serde_json::from_str(&result).expect("json payload");
        let data = payload.get("data").expect("data field");
        assert_eq!(data.get("center"), Some(&json!("machine-learning")));

        let nodes = data
            .get("nodes")
            .and_then(Value::as_array)
            .expect("nodes array");
        assert!(nodes.iter().any(|node| {
            node.get("id") == Some(&json!("machine-learning"))
                && node.get("label") == Some(&json!("machine-learning"))
                && node.get("depth") == Some(&json!(0))
        }));
        assert!(nodes.iter().any(|node| {
            node.get("id") == Some(&json!("neural-networks"))
                && node.get("label") == Some(&json!("neural-networks"))
                && node.get("depth") == Some(&json!(1))
        }));

        let edges = data
            .get("edges")
            .and_then(Value::as_array)
            .expect("edges array");
        assert!(edges.iter().any(|edge| {
            edge.get("source") == Some(&json!("machine-learning"))
                && edge.get("target") == Some(&json!("neural-networks"))
                && edge.get("weight") == Some(&json!(12.0))
        }));
    }

    #[test]
    #[serial]
    fn visualize_graph_format_includes_center_when_no_neighbors() {
        let (_tmp, context, _guard) = visualize_test_context(OutputFormat::Json);
        seed_graph(&context, &["solo-concept"], &[]);

        let result = visualize(
            &context,
            &json!({
                "conceptName": "solo-concept",
                "depth": 2,
                "maxNodes": 30,
                "format": "graph"
            }),
        )
        .expect("visualize should succeed");

        let payload: Value = serde_json::from_str(&result).expect("json payload");
        let data = payload.get("data").expect("data field");
        assert_eq!(data.get("center"), Some(&json!("solo-concept")));
        assert_eq!(
            data.get("nodes"),
            Some(&json!([{
                "id": "solo-concept",
                "label": "solo-concept",
                "depth": 0
            }]))
        );
        assert_eq!(data.get("edges"), Some(&json!([])));
    }
}
