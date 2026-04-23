//! T-01.17: scoring helpers split from [`super::louvain`] to keep the parent
//! file under the per-file LOC budget. No behavioral changes.

use super::metrics::EPSILON;
use std::collections::BTreeMap;

pub(super) fn neighbor_community_weights(
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

// T-15.9: RTM FR-23.7 — Modularity gain formula matching Blondel 2008 and C#.
// The first term uses kiIn/m (= 2*kiIn/(2m)), not kiIn/(2m).
#[allow(clippy::too_many_arguments)]
pub(super) fn insertion_gain(
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

    ((sum_in + 2.0 * node_internal_weight) / two_m)
        - ((sum_tot + node_degree) / two_m).powi(2) * gamma
        - ((sum_in / two_m)
            - (sum_tot / two_m).powi(2) * gamma
            - (node_degree / two_m).powi(2) * gamma)
}

pub(super) fn adjust_aggregate(aggregates: &mut BTreeMap<usize, f64>, community_id: usize, delta: f64) {
    let entry = aggregates.entry(community_id).or_insert(0.0);
    *entry += delta;
    if entry.abs() <= EPSILON {
        *entry = 0.0;
    }
}

pub(super) fn normalize_communities(communities: &mut [usize]) {
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

pub(super) fn compute_modularity(
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
