use crate::error::{MaenifoldError, Result};
#[cfg(all(test, target_os = "macos"))]
use crate::schedule::LaunchctlBackend;
#[cfg(target_os = "macos")]
use crate::schedule::LaunchdManager;
use crate::schedule::ScheduledTask;
#[cfg(all(test, target_os = "linux"))]
use crate::schedule::SystemctlBackend;
#[cfg(target_os = "linux")]
use crate::schedule::SystemdManager;
use serde::Serialize;
use std::path::Path;
#[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
use std::path::PathBuf;
#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::process::Command;
#[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
use std::sync::Arc;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VerifyInstalledStatus {
    pub id: String,
    pub installed: bool,
    pub enabled_in_config: bool,
    pub discrepancy: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_fire_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_fire_result: Option<LastFireResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LastFireResult {
    pub exit_code: i32,
    pub finished_at: String,
}

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

#[cfg(all(test, target_os = "macos"))]
pub(in crate::tools) fn sync_os_scheduler_for_test(
    task: &ScheduledTask,
    bin: &Path,
    agents_dir: PathBuf,
    backend: Arc<dyn LaunchctlBackend>,
) -> Result<()> {
    let manager = LaunchdManager::new_with(agents_dir, bin.to_path_buf(), backend)
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

#[cfg(all(test, target_os = "linux"))]
pub(in crate::tools) fn sync_os_scheduler_for_test(
    task: &ScheduledTask,
    bin: &Path,
    units_dir: PathBuf,
    backend: Arc<dyn SystemctlBackend>,
) -> Result<()> {
    let manager = SystemdManager::new_with(units_dir, bin.to_path_buf(), backend)
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

#[cfg(all(test, target_os = "macos"))]
pub(in crate::tools) fn uninstall_os_scheduler_for_test(
    task_id: &str,
    bin: &Path,
    agents_dir: PathBuf,
    backend: Arc<dyn LaunchctlBackend>,
) -> Result<()> {
    LaunchdManager::new_with(agents_dir, bin.to_path_buf(), backend)
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

#[cfg(all(test, target_os = "linux"))]
pub(in crate::tools) fn uninstall_os_scheduler_for_test(
    task_id: &str,
    bin: &Path,
    units_dir: PathBuf,
    backend: Arc<dyn SystemctlBackend>,
) -> Result<()> {
    SystemdManager::new_with(units_dir, bin.to_path_buf(), backend)
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

pub(crate) fn verify_installed(task: &ScheduledTask, bin: &Path) -> Result<VerifyInstalledStatus> {
    let installed = verify_os_installed(task, bin)?;
    Ok(VerifyInstalledStatus {
        id: task.id.clone(),
        installed,
        enabled_in_config: task.enabled,
        discrepancy: installed != task.enabled,
        next_fire_time: None,
        last_fire_result: None,
        note: unsupported_note(),
    })
}

#[cfg(target_os = "macos")]
fn verify_os_installed(task: &ScheduledTask, bin: &Path) -> Result<bool> {
    // Side-effect: validates bin path; manager is otherwise unused.
    LaunchdManager::new(bin.to_path_buf())
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    let label = format!("com.maenifold.schedule.{}", task.id);
    let target = format!("user/{}/{}", unsafe { libc::getuid() }, label);
    let output = Command::new("launchctl")
        .arg("print")
        .arg(target)
        .output()
        .map_err(|error| {
            MaenifoldError::Message(format!("failed to run launchctl print: {error}"))
        })?;
    Ok(output.status.success())
}

#[cfg(target_os = "linux")]
fn verify_os_installed(task: &ScheduledTask, bin: &Path) -> Result<bool> {
    // Side-effect: validates bin path; manager is otherwise unused.
    SystemdManager::new(bin.to_path_buf())
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    let unit = format!("maenifold-schedule-{}.timer", task.id);
    let output = Command::new("systemctl")
        .args(["--user", "is-enabled", &unit])
        .output()
        .map_err(|error| {
            MaenifoldError::Message(format!("failed to run systemctl is-enabled: {error}"))
        })?;
    Ok(output.status.success())
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn verify_os_installed(_task: &ScheduledTask, _bin: &Path) -> Result<bool> {
    Ok(false)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn unsupported_note() -> Option<String> {
    None
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn unsupported_note() -> Option<String> {
    Some("platform not supported".to_string())
}

// OS integration verified by manual UAT, see done/T-243.108.md history.
