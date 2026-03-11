// T-12.6: RTM FR-5
#[derive(Clone)]
pub(crate) struct TimingStats {
    pub(crate) min_ms: f64,
    pub(crate) max_ms: f64,
    pub(crate) avg_ms: f64,
    pub(crate) median_ms: f64,
}

impl TimingStats {
    pub(crate) fn from_samples(samples: &[f64]) -> Self {
        if samples.is_empty() {
            return Self {
                min_ms: 0.0,
                max_ms: 0.0,
                avg_ms: 0.0,
                median_ms: 0.0,
            };
        }

        let mut sorted = samples.to_vec();
        sorted.sort_by(f64::total_cmp);
        let len = sorted.len();
        let median_ms = if len.is_multiple_of(2) {
            let upper = sorted[len / 2];
            let lower = sorted[(len / 2) - 1];
            (lower + upper) / 2.0
        } else {
            sorted[len / 2]
        };
        let sum = samples.iter().sum::<f64>();

        Self {
            min_ms: *sorted.first().unwrap_or(&0.0),
            max_ms: *sorted.last().unwrap_or(&0.0),
            avg_ms: sum / samples.len() as f64,
            median_ms,
        }
    }
}
