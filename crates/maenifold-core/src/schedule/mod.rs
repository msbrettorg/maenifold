mod config;
mod cron;
pub mod env_allowlist;
mod launchd;
mod lock;
pub(crate) mod mcp_action_allowlist;
pub mod pause_state;
#[cfg(test)]
#[path = "pause_state_tests.rs"]
mod pause_state_tests;
mod store;
mod supervisor;
mod systemd;

pub use config::{RunRecord, RunStatus, ScheduledTask};
pub use cron::{ParsedSchedule, parse_schedule};
#[cfg(all(test, target_os = "macos"))]
#[allow(unused_imports)]
pub(crate) use launchd::{FailingLaunchctlBackend, NoopLaunchctlBackend};
pub use launchd::{LaunchctlBackend, LaunchdManager, RealLaunchctlBackend};
pub use lock::LockGuard;
pub use store::ScheduleStore;
pub use supervisor::Supervisor;
#[cfg(all(test, target_os = "linux"))]
pub(crate) use systemd::{FailingSystemctlBackend, NoopSystemctlBackend};
pub use systemd::{RealSystemctlBackend, SystemctlBackend, SystemdManager};

pub(crate) fn validate_schedule_id(id: &str) -> anyhow::Result<()> {
    if id.is_empty() || id.len() > 64 {
        anyhow::bail!("schedule id must be between 1 and 64 characters");
    }
    if !id.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-') {
        anyhow::bail!("schedule id must contain only alphanumeric characters or hyphens");
    }
    Ok(())
}
