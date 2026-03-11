use crate::db::{self, GraphDatabase};
use crate::error::Result;
use crate::markdown::normalize_concept;
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use rusqlite::{Connection, params};
use serde_json::Value;
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
