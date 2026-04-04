use super::data::parse_source_file_weights;
use std::collections::BTreeMap;
#[cfg(test)]
use std::sync::{Mutex, OnceLock};

pub(crate) const EPSILON: f64 = 1e-12;
// T-14.6: RTM FR-15
const COMMUNITY_NMI_THRESHOLD_ENV: &str = "MAENIFOLD_COMMUNITY_NMI_THRESHOLD";

// T-4.3: RTM FR-23.4
pub fn compute_edge_decay_weight(source_files: &str) -> crate::error::Result<f64> {
    let weights = parse_source_file_weights(source_files)?;
    if weights.is_empty() {
        return Ok(0.0);
    }

    Ok(weights.values().sum::<f64>() / weights.len() as f64)
}

// T-4.3: RTM FR-23.7
pub fn compute_nmi(left: &BTreeMap<String, usize>, right: &BTreeMap<String, usize>) -> f64 {
    let shared = left
        .keys()
        .filter(|concept| right.contains_key(*concept))
        .cloned()
        .collect::<Vec<_>>();
    if shared.is_empty() {
        return 1.0;
    }

    let total = shared.len() as f64;
    let mut left_counts = BTreeMap::<usize, usize>::new();
    let mut right_counts = BTreeMap::<usize, usize>::new();
    let mut joint_counts = BTreeMap::<(usize, usize), usize>::new();

    for concept in shared {
        let left_cluster = left[&concept];
        let right_cluster = right[&concept];
        *left_counts.entry(left_cluster).or_insert(0) += 1;
        *right_counts.entry(right_cluster).or_insert(0) += 1;
        *joint_counts
            .entry((left_cluster, right_cluster))
            .or_insert(0) += 1;
    }

    let entropy = |counts: &BTreeMap<usize, usize>| {
        counts
            .values()
            .map(|count| {
                let probability = *count as f64 / total;
                if probability <= EPSILON {
                    0.0
                } else {
                    -probability * probability.ln()
                }
            })
            .sum::<f64>()
    };

    let left_entropy = entropy(&left_counts);
    let right_entropy = entropy(&right_counts);
    if left_entropy <= EPSILON && right_entropy <= EPSILON {
        return 1.0;
    }

    let mut mutual_information = 0.0;
    for ((left_cluster, right_cluster), joint_count) in joint_counts {
        let joint_probability = joint_count as f64 / total;
        if joint_probability <= EPSILON {
            continue;
        }
        let left_probability = *left_counts.get(&left_cluster).unwrap_or(&0) as f64 / total;
        let right_probability = *right_counts.get(&right_cluster).unwrap_or(&0) as f64 / total;
        if left_probability <= EPSILON || right_probability <= EPSILON {
            continue;
        }
        mutual_information +=
            joint_probability * (joint_probability / (left_probability * right_probability)).ln();
    }

    let normalized = (2.0 * mutual_information) / (left_entropy + right_entropy);
    normalized.clamp(0.0, 1.0)
}

// T-4.3: RTM FR-23.7
pub(crate) fn maybe_warn_on_community_drift(
    previous_assignments: &BTreeMap<String, usize>,
    concepts: &[String],
    communities: &[usize],
) {
    if previous_assignments.is_empty() || concepts.len() != communities.len() {
        return;
    }

    let current_assignments = concepts
        .iter()
        .cloned()
        .zip(communities.iter().copied())
        .collect::<BTreeMap<_, _>>();
    let nmi = compute_nmi(previous_assignments, &current_assignments);
    let threshold = community_nmi_threshold();

    if nmi < threshold {
        let warning = format!(
            "Community drift warning: NMI {:.4} fell below threshold {:.4}\n",
            nmi, threshold
        );
        #[cfg(unix)]
        unsafe {
            unsafe extern "C" {
                fn write(fd: i32, buf: *const u8, count: usize) -> isize;
            }

            let _ = write(2, warning.as_ptr(), warning.len());
        }
        #[cfg(not(unix))]
        eprintln!("{warning}");
    }
}

// T-14.6: RTM FR-15
fn community_nmi_threshold() -> f64 {
    std::env::var(COMMUNITY_NMI_THRESHOLD_ENV)
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(0.8)
}

#[cfg(test)]
fn env_lock() -> &'static Mutex<()> {
    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    ENV_LOCK.get_or_init(|| Mutex::new(()))
}

#[cfg(test)]
struct EnvVarGuard {
    key: &'static str,
    previous: Option<String>,
}

#[cfg(test)]
impl EnvVarGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let previous = std::env::var(key).ok();
        unsafe {
            std::env::set_var(key, value);
        }
        Self { key, previous }
    }

    fn remove(key: &'static str) -> Self {
        let previous = std::env::var(key).ok();
        unsafe {
            std::env::remove_var(key);
        }
        Self { key, previous }
    }
}

#[cfg(test)]
impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => unsafe {
                std::env::set_var(self.key, value);
            },
            None => unsafe {
                std::env::remove_var(self.key);
            },
        }
    }
}
