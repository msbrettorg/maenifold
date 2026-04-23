// T-01.16: subgraph traversal types + walker extracted from graph/analyzer.rs.
// No behavioural changes.

use rusqlite::{Connection, params};
use std::collections::HashSet;

use crate::error::Result;

#[derive(Debug, Clone)]
pub(super) struct LocalSubgraph {
    pub(super) center: String,
    pub(super) nodes: Vec<LocalSubgraphNode>,
    pub(super) edges: Vec<LocalSubgraphEdge>,
}

#[derive(Debug, Clone)]
pub(super) struct LocalSubgraphNode {
    pub(super) concept: String,
    pub(super) depth: u32,
}

#[derive(Debug, Clone)]
pub(super) struct LocalSubgraphEdge {
    pub(super) source: String,
    pub(super) target: String,
    pub(super) weight: f64,
}

pub(super) fn walk_local_subgraph(
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
