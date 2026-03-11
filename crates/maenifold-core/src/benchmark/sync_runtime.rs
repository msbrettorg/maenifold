use super::stats::TimingStats;
use crate::benchmark::render::{claim_status, format_ms, format_seconds};
use crate::error::Result;
use crate::sync;
use crate::tools::ToolContext;
use serde_json::json;

const SYNC_CLAIM_PROJECTED_MS: f64 = 27_000.0;

// T-12.6: RTM FR-5
pub(crate) fn render_sync_benchmark(context: &ToolContext, iterations: usize) -> Result<String> {
    let file_count = walkdir::WalkDir::new(&context.config.memory_path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .count();
    let sync_iterations = iterations.min(3);
    let mut timings = Vec::with_capacity(sync_iterations);

    for _ in 0..sync_iterations {
        let started = std::time::Instant::now();
        let _ = sync::sync(context, &json!({}))?;
        timings.push(started.elapsed().as_secs_f64() * 1_000.0);
    }

    let stats = TimingStats::from_samples(&timings);
    let avg_per_file_ms = if file_count > 0 {
        stats.avg_ms / file_count as f64
    } else {
        0.0
    };
    let projected_2500_ms = avg_per_file_ms * 2_500.0;

    Ok(format!(
        "Sync Performance Benchmark\n\
--------------------------\n\
Claim: 27s for 2,500 files\n\
Sync runs executed: {}\n\
Files available for sync: {}\n\
Average sync time: {}\n\
Sync Min/Max/Median: {} / {} / {}\n\
Average time per file: {}\n\
Projected time for 2,500 files: {} (claim: 27.0s) {}\n\n",
        sync_iterations,
        file_count,
        format_ms(stats.avg_ms),
        format_ms(stats.min_ms),
        format_ms(stats.max_ms),
        format_ms(stats.median_ms),
        format_ms(avg_per_file_ms),
        format_seconds(projected_2500_ms),
        claim_status(projected_2500_ms, SYNC_CLAIM_PROJECTED_MS),
    ))
}
