use crate::db;
use crate::error::Result;
use rusqlite::{Connection, params};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

const EPSILON: f64 = 1e-12;

#[derive(Debug)]
struct GraphData {
    concepts: Vec<String>,
    adjacency: Vec<Vec<(usize, f64)>>,
    total_edge_weight: f64,
}

// T-3.2a: RTM FR-13.1 / FR-13.3 / FR-13.11
pub fn detect_and_persist(db_path: &Path, gamma: f64) -> Result<usize> {
    let mut connection = db::open_connection(db_path)?;
    let graph = load_graph(&connection)?;
    let (communities, modularity) = run_louvain(&graph, gamma);
    let (communities, modularity) =
        fallback_to_connected_components(&graph, communities, modularity, gamma);
    persist_assignments(
        &mut connection,
        &graph.concepts,
        &communities,
        modularity,
        gamma,
    )?;
    Ok(communities.iter().copied().collect::<BTreeSet<_>>().len())
}

// T-3.2a: RTM FR-13.1 / FR-13.2
fn load_graph(connection: &Connection) -> Result<GraphData> {
    let mut concepts_stmt =
        connection.prepare("SELECT concept_name FROM concepts ORDER BY concept_name")?;
    let concepts = concepts_stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let index_by_name = concepts
        .iter()
        .enumerate()
        .map(|(index, name)| (name.clone(), index))
        .collect::<HashMap<_, _>>();

    let mut adjacency = vec![Vec::new(); concepts.len()];
    let mut edge_stmt = connection.prepare(
        "SELECT concept_a, concept_b, co_occurrence_count \
         FROM concept_graph \
         ORDER BY concept_a, concept_b",
    )?;
    let edge_rows = edge_stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            // T-4.3: RTM FR-23.x
            row.get::<_, f64>(2)?,
        ))
    })?;

    for edge_row in edge_rows {
        let (concept_a, concept_b, weight) = edge_row?;
        let Some(&index_a) = index_by_name.get(&concept_a) else {
            continue;
        };
        let Some(&index_b) = index_by_name.get(&concept_b) else {
            continue;
        };
        if weight <= 0.0 {
            continue;
        }
        adjacency[index_a].push((index_b, weight));
        adjacency[index_b].push((index_a, weight));
    }

    for neighbors in &mut adjacency {
        neighbors.sort_by_key(|(neighbor, _)| *neighbor);
    }

    let total_edge_weight = adjacency
        .iter()
        .flat_map(|neighbors| neighbors.iter().map(|(_, weight)| *weight))
        .sum::<f64>()
        / 2.0;

    Ok(GraphData {
        concepts,
        adjacency,
        total_edge_weight,
    })
}

// T-3.2a: RTM FR-13.1 / FR-13.11
fn run_louvain(graph: &GraphData, gamma: f64) -> (Vec<usize>, f64) {
    let node_count = graph.concepts.len();
    if node_count == 0 {
        return (Vec::new(), 0.0);
    }

    if graph.total_edge_weight <= EPSILON {
        let singleton_communities = (0..node_count).collect::<Vec<_>>();
        return (singleton_communities, 0.0);
    }

    let degree = graph
        .adjacency
        .iter()
        .map(|neighbors| neighbors.iter().map(|(_, weight)| *weight).sum::<f64>())
        .collect::<Vec<_>>();
    let mut communities = (0..node_count).collect::<Vec<_>>();
    let mut sum_tot = degree
        .iter()
        .enumerate()
        .map(|(community_id, weighted_degree)| (community_id, *weighted_degree))
        .collect::<BTreeMap<_, _>>();
    let mut sum_in = (0..node_count)
        .map(|community_id| (community_id, 0.0))
        .collect::<BTreeMap<_, _>>();

    loop {
        let mut moved_any = false;

        for node_index in 0..node_count {
            let current_community = communities[node_index];
            let node_degree = degree[node_index];
            let neighbor_weights =
                neighbor_community_weights(node_index, &communities, &graph.adjacency);
            let current_internal_weight = *neighbor_weights.get(&current_community).unwrap_or(&0.0);

            adjust_aggregate(&mut sum_tot, current_community, -node_degree);
            adjust_aggregate(&mut sum_in, current_community, -current_internal_weight);

            let current_gain = insertion_gain(
                *sum_in.get(&current_community).unwrap_or(&0.0),
                *sum_tot.get(&current_community).unwrap_or(&0.0),
                current_internal_weight,
                node_degree,
                graph.total_edge_weight,
                gamma,
            );

            let mut best_community = current_community;
            let mut best_net_gain = 0.0;

            for (&candidate_community, &candidate_internal_weight) in &neighbor_weights {
                if candidate_community == current_community {
                    continue;
                }

                let candidate_gain = insertion_gain(
                    *sum_in.get(&candidate_community).unwrap_or(&0.0),
                    *sum_tot.get(&candidate_community).unwrap_or(&0.0),
                    candidate_internal_weight,
                    node_degree,
                    graph.total_edge_weight,
                    gamma,
                );
                let net_gain = candidate_gain - current_gain;

                if net_gain > best_net_gain + EPSILON
                    || ((net_gain - best_net_gain).abs() <= EPSILON
                        && candidate_community < best_community)
                {
                    best_community = candidate_community;
                    best_net_gain = net_gain;
                }
            }

            let chosen_internal_weight = if best_community == current_community {
                current_internal_weight
            } else {
                *neighbor_weights.get(&best_community).unwrap_or(&0.0)
            };

            communities[node_index] = best_community;
            adjust_aggregate(&mut sum_tot, best_community, node_degree);
            adjust_aggregate(&mut sum_in, best_community, chosen_internal_weight);

            if best_community != current_community {
                moved_any = true;
            }
        }

        if !moved_any {
            break;
        }
    }

    normalize_communities(&mut communities);
    let modularity = compute_modularity(
        &graph.adjacency,
        &degree,
        &communities,
        graph.total_edge_weight,
        gamma,
    );
    (communities, modularity)
}

// T-3.5: RTM FR-13.1 / RTM FR-13.7
fn fallback_to_connected_components(
    graph: &GraphData,
    communities: Vec<usize>,
    modularity: f64,
    gamma: f64,
) -> (Vec<usize>, f64) {
    if graph.total_edge_weight <= EPSILON {
        return (communities, modularity);
    }

    let distinct_count = communities.iter().copied().collect::<BTreeSet<_>>().len();
    if distinct_count < graph.concepts.len() {
        return (communities, modularity);
    }

    let component_communities = connected_components(graph);
    let component_count = component_communities
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .len();
    if component_count >= distinct_count {
        return (communities, modularity);
    }

    let degree = graph
        .adjacency
        .iter()
        .map(|neighbors| neighbors.iter().map(|(_, weight)| *weight).sum::<f64>())
        .collect::<Vec<_>>();
    let component_modularity = compute_modularity(
        &graph.adjacency,
        &degree,
        &component_communities,
        graph.total_edge_weight,
        gamma,
    );
    (component_communities, component_modularity)
}

fn connected_components(graph: &GraphData) -> Vec<usize> {
    let mut assignments = vec![usize::MAX; graph.concepts.len()];
    let mut next_component = 0usize;

    for start in 0..graph.adjacency.len() {
        if assignments[start] != usize::MAX {
            continue;
        }

        let mut stack = vec![start];
        assignments[start] = next_component;

        while let Some(node_index) = stack.pop() {
            for &(neighbor_index, _) in &graph.adjacency[node_index] {
                if assignments[neighbor_index] != usize::MAX {
                    continue;
                }
                assignments[neighbor_index] = next_component;
                stack.push(neighbor_index);
            }
        }

        next_component += 1;
    }

    assignments
}

// T-3.2a: RTM FR-13.1
fn neighbor_community_weights(
    node_index: usize,
    communities: &[usize],
    adjacency: &[Vec<(usize, f64)>],
) -> BTreeMap<usize, f64> {
    let mut weights = BTreeMap::new();
    for &(neighbor_index, edge_weight) in &adjacency[node_index] {
        let community_id = communities[neighbor_index];
        *weights.entry(community_id).or_insert(0.0) += edge_weight;
    }
    weights
}

// T-3.2a: RTM FR-13.1 / FR-13.11
fn insertion_gain(
    sum_in: f64,
    sum_tot: f64,
    node_internal_weight: f64,
    node_degree: f64,
    total_edge_weight: f64,
    gamma: f64,
) -> f64 {
    let two_m = 2.0 * total_edge_weight;
    if two_m <= EPSILON {
        return 0.0;
    }

    ((sum_in + node_internal_weight) / two_m)
        - ((sum_tot + node_degree) / two_m).powi(2) * gamma
        - ((sum_in / two_m)
            - (sum_tot / two_m).powi(2) * gamma
            - (node_degree / two_m).powi(2) * gamma)
}

fn adjust_aggregate(aggregates: &mut BTreeMap<usize, f64>, community_id: usize, delta: f64) {
    let entry = aggregates.entry(community_id).or_insert(0.0);
    *entry += delta;
    if entry.abs() <= EPSILON {
        *entry = 0.0;
    }
}

// T-3.2a: RTM FR-13.1
fn normalize_communities(communities: &mut [usize]) {
    let mut normalized = BTreeMap::new();
    let mut next_id = 0usize;

    for community_id in communities.iter_mut() {
        let canonical_id = *normalized.entry(*community_id).or_insert_with(|| {
            let assigned = next_id;
            next_id += 1;
            assigned
        });
        *community_id = canonical_id;
    }
}

// T-3.2a: RTM FR-13.1 / FR-13.11
fn compute_modularity(
    adjacency: &[Vec<(usize, f64)>],
    degree: &[f64],
    communities: &[usize],
    total_edge_weight: f64,
    gamma: f64,
) -> f64 {
    if total_edge_weight <= EPSILON {
        return 0.0;
    }

    let mut sum_tot = BTreeMap::<usize, f64>::new();
    let mut sum_in = BTreeMap::<usize, f64>::new();

    for (node_index, &community_id) in communities.iter().enumerate() {
        *sum_tot.entry(community_id).or_insert(0.0) += degree[node_index];
        sum_in.entry(community_id).or_insert(0.0);
    }

    for (node_index, neighbors) in adjacency.iter().enumerate() {
        for &(neighbor_index, edge_weight) in neighbors {
            if neighbor_index > node_index && communities[node_index] == communities[neighbor_index]
            {
                *sum_in.entry(communities[node_index]).or_insert(0.0) += edge_weight;
            }
        }
    }

    let two_m = 2.0 * total_edge_weight;
    sum_tot
        .into_iter()
        .map(|(community_id, community_degree)| {
            let internal_weight = *sum_in.get(&community_id).unwrap_or(&0.0);
            (internal_weight / two_m) - gamma * (community_degree / two_m).powi(2)
        })
        .sum()
}

// T-3.2a: RTM FR-13.3
fn persist_assignments(
    connection: &mut Connection,
    concepts: &[String],
    communities: &[usize],
    modularity: f64,
    gamma: f64,
) -> Result<()> {
    let transaction = connection.transaction()?;
    transaction.execute("DELETE FROM concept_communities", [])?;

    for (concept_name, community_id) in concepts.iter().zip(communities.iter().copied()) {
        transaction.execute(
            "INSERT INTO concept_communities (concept_name, community_id, modularity, resolution, detected_at) \
             VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            params![concept_name, community_id as i64, modularity, gamma],
        )?;
    }

    transaction.commit()?;
    Ok(())
}
