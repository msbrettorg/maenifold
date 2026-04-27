mod config;
mod cron;
mod launchd;
mod lock;
mod store;
mod supervisor;
mod systemd;

pub use config::{RunRecord, RunStatus, ScheduledTask};
pub use cron::{ParsedSchedule, parse_schedule};
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
