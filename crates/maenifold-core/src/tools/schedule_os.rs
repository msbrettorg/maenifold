use crate::error::{MaenifoldError, Result};
#[cfg(target_os = "macos")]
use crate::schedule::LaunchdManager;
use crate::schedule::ScheduledTask;
#[cfg(target_os = "linux")]
use crate::schedule::SystemdManager;
use std::path::Path;

#[cfg(target_os = "macos")]
pub(super) fn sync_os_scheduler(task: &ScheduledTask, bin: &Path) -> Result<()> {
    let manager = LaunchdManager::new(bin.to_path_buf())
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    if task.enabled {
        manager
            .install(task)
            .map_err(|error| MaenifoldError::Message(error.to_string()))
    } else {
        manager
            .uninstall(&task.id)
            .map_err(|error| MaenifoldError::Message(error.to_string()))
    }
}

#[cfg(target_os = "linux")]
pub(super) fn sync_os_scheduler(task: &ScheduledTask, bin: &Path) -> Result<()> {
    let manager = SystemdManager::new(bin.to_path_buf())
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    if task.enabled {
        manager
            .install(task)
            .map_err(|error| MaenifoldError::Message(error.to_string()))
    } else {
        manager
            .uninstall(&task.id)
            .map_err(|error| MaenifoldError::Message(error.to_string()))
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub(super) fn sync_os_scheduler(task: &ScheduledTask, _bin: &Path) -> Result<()> {
    if task.enabled {
        Err(MaenifoldError::InvalidArgument(
            "enabled schedules are only supported on macOS launchd and Linux systemd user timers"
                .to_string(),
        ))
    } else {
        Ok(())
    }
}

#[cfg(target_os = "macos")]
pub(super) fn uninstall_os_scheduler(task_id: &str, bin: &Path) -> Result<()> {
    LaunchdManager::new(bin.to_path_buf())
        .map_err(|error| MaenifoldError::Message(error.to_string()))?
        .uninstall(task_id)
        .map_err(|error| MaenifoldError::Message(error.to_string()))
}

#[cfg(target_os = "linux")]
pub(super) fn uninstall_os_scheduler(task_id: &str, bin: &Path) -> Result<()> {
    SystemdManager::new(bin.to_path_buf())
        .map_err(|error| MaenifoldError::Message(error.to_string()))?
        .uninstall(task_id)
        .map_err(|error| MaenifoldError::Message(error.to_string()))
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub(super) fn uninstall_os_scheduler(_task_id: &str, _bin: &Path) -> Result<()> {
    Ok(())
}

pub(super) fn is_not_installed_scheduler_error(error: &MaenifoldError) -> bool {
    let message = error.to_string().to_ascii_lowercase();
    [
        "launchctl unload failed: unload failed: 5: input/output error",
        "launchctl unload failed: could not find specified service",
        "launchctl unload failed: no such process",
        "systemctl disable failed: unit file maenifold-schedule-",
        "systemctl disable failed: unit maenifold-schedule-",
    ]
    .iter()
    .any(|needle| message.contains(needle))
}
