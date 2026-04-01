use super::data::GraphData;
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

// T-15.9: RTM FR-23.7 — Modularity gain formula matching Blondel 2008 and C#.
// The first term uses kiIn/m (= 2*kiIn/(2m)), not kiIn/(2m).
#[allow(clippy::too_many_arguments)]
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

    ((sum_in + 2.0 * node_internal_weight) / two_m)
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

#[cfg(test)]
mod tests {
    use super::*;

    // T-15.9: RTM FR-23.7 — Verify gain formula uses kiIn/m (not kiIn/(2m)).
    // The (ki/(2m))^2 terms cancel, so the simplified gain is:
    //   gain = kiIn/m - gamma * sumTot * ki / (2m^2)
    // With sum_in=0, sum_tot=0, kiIn=3, ki=6, m=15, gamma=1:
    //   Correct (kiIn/m):    3/15 - 0 = 0.2
    //   Old bug (kiIn/(2m)): 3/30 - 0 = 0.1
    #[test]
    fn insertion_gain_uses_ki_in_over_m() {
        let gain = insertion_gain(
            0.0,  // sum_in: community has no internal edges
            0.0,  // sum_tot: community aggregate degree is 0 (node already removed)
            3.0,  // node_internal_weight (kiIn)
            6.0,  // node_degree (ki)
            15.0, // total_edge_weight (m)
            1.0,  // gamma
        );
        // gain = kiIn/m - gamma * sumTot * ki / (2m^2) = 3/15 - 0 = 0.2
        // Old buggy formula would give: kiIn/(2m) - 0 = 3/30 = 0.1
        let expected = 3.0 / 15.0; // 0.2
        assert!(
            (gain - expected).abs() < 1e-10,
            "insertion_gain must use kiIn/m: expected {expected}, got {gain}"
        );
    }

    // T-15.9: RTM FR-23.7 — The simplified gain equals kiIn/m - gamma*sumTot*ki/(2m^2).
    // This matches the C# formula exactly.
    #[test]
    fn insertion_gain_matches_csharp_formula() {
        let sum_in = 4.0;
        let sum_tot = 10.0;
        let ki_in = 3.0;
        let ki = 8.0;
        let m = 20.0;
        let gamma = 1.0;

        let gain = insertion_gain(sum_in, sum_tot, ki_in, ki, m, gamma);

        // C# formula: kiIn/m - gamma * sumTot * ki / (2*m*m)
        let csharp_gain = ki_in / m - gamma * sum_tot * ki / (2.0 * m * m);
        // But insertion_gain returns the FULL modularity contribution (not just the net).
        // Expand: (sum_in + 2*kiIn)/(2m) - gamma*((sum_tot+ki)/(2m))^2
        //         - sum_in/(2m) + gamma*(sum_tot/(2m))^2 + gamma*(ki/(2m))^2
        // Simplify: 2*kiIn/(2m) - gamma*(2*sum_tot*ki)/(4*m^2)
        //         = kiIn/m - gamma*sum_tot*ki/(2*m^2)
        assert!(
            (gain - csharp_gain).abs() < 1e-10,
            "gain must match C# formula: expected {csharp_gain}, got {gain}"
        );
    }

    // T-15.10: RTM FR-23.7 — Fisher-Yates shuffle produces deterministic order with seed.
    #[test]
    fn fisher_yates_deterministic_with_seed() {
        let mut order1: Vec<usize> = (0..10).collect();
        let mut order2: Vec<usize> = (0..10).collect();
        let mut rng1 = SplitMix64::new(42);
        let mut rng2 = SplitMix64::new(42);
        fisher_yates_shuffle(&mut order1, &mut rng1);
        fisher_yates_shuffle(&mut order2, &mut rng2);
        assert_eq!(order1, order2, "same seed must produce same shuffle");
        // Verify it actually shuffled (not identity)
        let identity: Vec<usize> = (0..10).collect();
        assert_ne!(
            order1, identity,
            "shuffle with seed 42 should differ from identity"
        );
    }

    // T-15.10: RTM FR-23.7 — Different seeds produce different orders.
    #[test]
    fn fisher_yates_different_seeds_different_order() {
        let mut order1: Vec<usize> = (0..20).collect();
        let mut order2: Vec<usize> = (0..20).collect();
        let mut rng1 = SplitMix64::new(42);
        let mut rng2 = SplitMix64::new(99);
        fisher_yates_shuffle(&mut order1, &mut rng1);
        fisher_yates_shuffle(&mut order2, &mut rng2);
        assert_ne!(
            order1, order2,
            "different seeds should produce different shuffles"
        );
    }

    // T-15.10: RTM FR-23.7 — run_louvain with same seed is deterministic.
    #[test]
    fn run_louvain_same_seed_deterministic() {
        // Build a small graph: triangle (0-1-2) + pair (3-4)
        let graph = GraphData {
            concepts: vec!["a".into(), "b".into(), "c".into(), "d".into(), "e".into()],
            adjacency: vec![
                vec![(1, 5.0), (2, 5.0)], // a -> b, c
                vec![(0, 5.0), (2, 5.0)], // b -> a, c
                vec![(0, 5.0), (1, 5.0)], // c -> a, b
                vec![(4, 5.0)],           // d -> e
                vec![(3, 5.0)],           // e -> d
            ],
            total_edge_weight: 20.0, // 4 edges * 5.0 weight
        };

        let (comm1, mod1) = run_louvain(&graph, 1.0, Some(42));
        let (comm2, mod2) = run_louvain(&graph, 1.0, Some(42));

        assert_eq!(comm1, comm2, "same seed must produce same communities");
        assert!(
            (mod1 - mod2).abs() < 1e-10,
            "same seed must produce same modularity"
        );
    }
}
