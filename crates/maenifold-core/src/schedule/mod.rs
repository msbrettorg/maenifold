mod config;
mod cron;
pub mod env_allowlist;
mod launchd;
mod lock;
mod store;
mod supervisor;
mod systemd;

pub use config::{RunRecord, RunStatus, ScheduledTask};
pub use cron::{parse_schedule, ParsedSchedule};
pub use launchd::LaunchdManager;
pub use lock::LockGuard;
pub use store::ScheduleStore;
pub use supervisor::Supervisor;
pub use systemd::SystemdManager;

pub(crate) fn validate_schedule_id(id: &str) -> anyhow::Result<()> {
    if id.is_empty() || id.len() > 64 {
        anyhow::bail!("schedule id must be between 1 and 64 characters");
    }
    if !id.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-') {
        anyhow::bail!("schedule id must contain only alphanumeric characters or hyphens");
    }
    Ok(())
}
