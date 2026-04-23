// T-01.16: render helpers (mermaid + JSON graph) extracted from graph/analyzer.rs.
// No behavioural changes.

use serde::Serialize;

use super::subgraph::{LocalSubgraph, LocalSubgraphEdge, LocalSubgraphNode};

pub(super) fn render_mermaid(subgraph: &LocalSubgraph) -> String {
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct GraphVisualization {
    center: String,
    nodes: Vec<GraphVisualizationNode>,
    edges: Vec<GraphVisualizationEdge>,
}

impl GraphVisualization {
    pub(super) fn from_subgraph(subgraph: &LocalSubgraph) -> Self {
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

fn format_co_occurrence_weight(weight: f64) -> String {
    if (weight.fract()).abs() < f64::EPSILON {
        format!("{weight:.0}")
    } else {
        format!("{weight:.1}")
    }
}
