// T-01.16: visualize tool entrypoint extracted from graph/analyzer.rs.
// No behavioural changes.

use rusqlite::params;
use serde_json::Value;
use std::fs;
use std::path::Path;

use crate::db::{self, GraphDatabase};
use crate::error::{MaenifoldError, Result};
use crate::markdown::normalize_concept;
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};

use super::render::{GraphVisualization, render_mermaid};
use super::subgraph::walk_local_subgraph;

#[derive(Debug, Clone, Copy)]
enum VisualizeOutputFormat {
    Mermaid,
    Graph,
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

fn load_help_doc(assets_path: &Path, tool_name: &str) -> Option<String> {
    fs::read_to_string(
        assets_path
            .join("usage")
            .join("tools")
            .join(format!("{}.md", tool_name.to_ascii_lowercase())),
    )
    .ok()
}
