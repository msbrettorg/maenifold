// T-92.2: Structured JSONL event logging via tracing-appender.

use crate::config::Config;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize structured JSONL file logging.
///
/// Returns a [`WorkerGuard`] that must be held alive for the duration of the
/// process — dropping it flushes buffered events and stops the background
/// writer thread.
pub fn init_tracing(config: &Config) -> Option<WorkerGuard> {
    let rotation_str = config.log_rotation();
    let (rotation, warn_unrecognized) = match rotation_str.as_str() {
        "daily" => (Rotation::DAILY, false),
        "hourly" => (Rotation::HOURLY, false),
        "minutely" => (Rotation::MINUTELY, false),
        _ => (Rotation::DAILY, true),
    };

    let file_appender = match RollingFileAppender::builder()
        .rotation(rotation)
        .filename_prefix("maenifold")
        .filename_suffix("jsonl")
        .build(config.maenifold_root.join("logs"))
    {
        Ok(file_appender) => file_appender,
        Err(error) => {
            eprintln!("failed to initialize rolling file appender: {error}");
            try_init_stderr_fallback();
            return None;
        }
    };

    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let subscriber = tracing_subscriber::registry()
        .with(default_env_filter())
        .with(fmt::layer().json().with_writer(non_blocking));

    if let Err(error) = subscriber.try_init() {
        eprintln!("tracing already initialized; skipping file subscriber: {error}");
        try_init_stderr_fallback();
        return None;
    }

    if warn_unrecognized {
        tracing::warn!(
            rotation = %rotation_str,
            "unrecognized MAENIFOLD_LOG_ROTATION value, defaulting to daily"
        );
    }

    Some(guard)
}

fn default_env_filter() -> EnvFilter {
    EnvFilter::builder()
        .with_default_directive(tracing::Level::INFO.into())
        .from_env_lossy()
}

fn try_init_stderr_fallback() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(default_env_filter())
        .try_init();
}
