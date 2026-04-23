use super::data::GraphData;
use super::louvain_scoring::{
    adjust_aggregate, compute_modularity, insertion_gain, neighbor_community_weights,
    normalize_communities,
};
use super::metrics::EPSILON;
use std::collections::{BTreeMap, BTreeSet};

// T-15.10: RTM FR-23.7 — Seedable PRNG for Fisher-Yates shuffle (SplitMix64).
// Avoids adding `rand` dependency for this single use case.
struct SplitMix64(u64);

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn from_entropy() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        Self(nanos)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    fn next_usize(&mut self, upper_exclusive: usize) -> usize {
        (self.next_u64() % upper_exclusive as u64) as usize
    }
}

// T-15.10: RTM FR-23.7 — Fisher-Yates shuffle matching C# CommunityDetection.Shuffle.
fn fisher_yates_shuffle(order: &mut [usize], rng: &mut SplitMix64) {
    for i in (1..order.len()).rev() {
        let j = rng.next_usize(i + 1);
        order.swap(i, j);
    }
}

// T-12.0: RTM FR-23.7
// T-15.9: RTM FR-23.7 — gain formula factor-of-2 fix
// T-15.10: RTM FR-23.7 — node visit order shuffle
#[allow(clippy::too_many_lines)]
pub(crate) fn run_louvain(graph: &GraphData, gamma: f64, seed: Option<u64>) -> (Vec<usize>, f64) {
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

        // T-15.10: RTM FR-23.7 — Shuffle node visit order each iteration.
        // With seed: deterministic (same shuffle each iteration, matching C# `new Random(seed)`).
        // Without seed: entropy-based (different shuffle each iteration, matching C# `new Random()`).
        let mut order: Vec<usize> = (0..node_count).collect();
        let mut rng = match seed {
            Some(s) => SplitMix64::new(s),
            None => SplitMix64::from_entropy(),
        };
        fisher_yates_shuffle(&mut order, &mut rng);

        for &node_index in &order {
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

// T-12.0: RTM FR-23.7
pub(crate) fn fallback_to_connected_components(
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

pub(crate) fn connected_components(graph: &GraphData) -> Vec<usize> {
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

