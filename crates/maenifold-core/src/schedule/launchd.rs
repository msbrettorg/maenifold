use super::{ScheduledTask, validate_schedule_id};
#[path = "launchd_cron.rs"]
mod launchd_cron;
use anyhow::{Context, Result, bail};
use launchd_cron::cron_to_start_calendar_interval_xml;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

const LABEL_PREFIX: &str = "com.maenifold.schedule.";
const DEFAULT_PATH: &str = "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin";

pub trait LaunchctlBackend: Send + Sync {
    fn load(&self, plist_path: &Path) -> Result<()>;
    fn unload(&self, plist_path: &Path) -> Result<()>;
}

pub struct RealLaunchctlBackend;

impl LaunchctlBackend for RealLaunchctlBackend {
    fn load(&self, plist_path: &Path) -> Result<()> {
        let output = Command::new("launchctl")
            .arg("load")
            .arg(plist_path)
            .output()
            .context("failed to run launchctl load")?;
        if !output.status.success() {
            bail!(
                "launchctl load failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }

    fn unload(&self, plist_path: &Path) -> Result<()> {
        let output = Command::new("launchctl")
            .arg("unload")
            .arg(plist_path)
            .output()
            .context("failed to run launchctl unload")?;
        if !output.status.success() {
            bail!(
                "launchctl unload failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }
}

#[cfg(all(test, target_os = "macos"))]
pub(crate) struct NoopLaunchctlBackend;

#[cfg(all(test, target_os = "macos"))]
impl LaunchctlBackend for NoopLaunchctlBackend {
    fn load(&self, _plist_path: &Path) -> Result<()> {
        Ok(())
    }

    fn unload(&self, _plist_path: &Path) -> Result<()> {
        Ok(())
    }
}

#[cfg(all(test, target_os = "macos"))]
pub(crate) struct FailingLaunchctlBackend;

#[cfg(all(test, target_os = "macos"))]
impl LaunchctlBackend for FailingLaunchctlBackend {
    fn load(&self, _plist_path: &Path) -> Result<()> {
        bail!("simulated launchctl failure for test")
    }

    fn unload(&self, _plist_path: &Path) -> Result<()> {
        bail!("simulated launchctl failure for test")
    }
}

pub struct LaunchdManager {
    agents_dir: PathBuf,
    maenifold_bin: PathBuf,
    backend: Arc<dyn LaunchctlBackend>,
}

impl LaunchdManager {
    pub fn new(maenifold_bin: PathBuf) -> Result<Self> {
        let home = std::env::var_os("HOME").context("HOME is not set")?;
        let maenifold_bin = if maenifold_bin.is_absolute() {
            maenifold_bin
        } else {
            std::env::current_dir()
                .context("failed to resolve current directory")?
                .join(maenifold_bin)
        };
        if !maenifold_bin.exists() {
            bail!(
                "maenifold binary does not exist: {}",
                maenifold_bin.display()
            );
        }
        Self::new_with(
            PathBuf::from(home).join("Library/LaunchAgents"),
            maenifold_bin,
            Arc::new(RealLaunchctlBackend),
        )
    }

    pub(crate) fn new_with(
        agents_dir: PathBuf,
        maenifold_bin: PathBuf,
        backend: Arc<dyn LaunchctlBackend>,
    ) -> Result<Self> {
        let maenifold_bin = if maenifold_bin.is_absolute() {
            maenifold_bin
        } else {
            std::env::current_dir()
                .context("failed to resolve current directory")?
                .join(maenifold_bin)
        };
        Ok(Self {
            agents_dir,
            maenifold_bin,
            backend,
        })
    }

    pub fn install(&self, task: &ScheduledTask) -> Result<()> {
        validate_schedule_id(&task.id)?;
        fs::create_dir_all(&self.agents_dir).with_context(|| {
            format!(
                "failed to create launch agents directory '{}'",
                self.agents_dir.display()
            )
        })?;
        let plist_path = self.plist_path(&task.id);
        if plist_path.exists() {
            match self.backend.unload(&plist_path) {
                Ok(()) => {}
                Err(error) if is_not_loaded_or_not_found(&error) => {}
                Err(error) => return Err(error),
            }
        }
        fs::write(&plist_path, render_plist(task, &self.maenifold_bin)?)
            .with_context(|| format!("failed to write plist '{}'", plist_path.display()))?;
        self.backend.load(&plist_path)
    }

    pub fn uninstall(&self, task_id: &str) -> Result<()> {
        validate_schedule_id(task_id)?;
        let plist_path = self.plist_path(task_id);
        if plist_path.exists() {
            self.backend.unload(&plist_path)?;
            fs::remove_file(&plist_path)
                .with_context(|| format!("failed to remove plist '{}'", plist_path.display()))?;
        }
        Ok(())
    }

    pub fn is_installed(&self, task_id: &str) -> bool {
        validate_schedule_id(task_id).is_ok() && self.plist_path(task_id).exists()
    }

    pub fn list_installed(&self) -> Result<Vec<String>> {
        if !self.agents_dir.exists() {
            return Ok(Vec::new());
        }
        let mut task_ids = fs::read_dir(&self.agents_dir)
            .with_context(|| format!("failed to read '{}'", self.agents_dir.display()))?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter_map(|name| {
                name.strip_prefix(LABEL_PREFIX)
                    .and_then(|value| value.strip_suffix(".plist"))
                    .map(str::to_owned)
            })
            .collect::<Vec<_>>();
        task_ids.sort();
        Ok(task_ids)
    }

    fn plist_path(&self, task_id: &str) -> PathBuf {
        self.agents_dir
            .join(format!("{LABEL_PREFIX}{task_id}.plist"))
    }
}

fn render_plist(task: &ScheduledTask, maenifold_bin: &Path) -> Result<String> {
    let label = format!("{LABEL_PREFIX}{}", task.id);
    let log_path = default_log_path(&task.id)?;
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create log directory '{}'", parent.display()))?;
    }
    let calendar_xml = cron_to_start_calendar_interval_xml(&task.schedule)?;
    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{bin}</string>
        <string>schedule</string>
        <string>run</string>
        <string>{task_id}</string>
    </array>
    <key>StartCalendarInterval</key>
    {calendar_xml}
    <key>StandardOutPath</key>
    <string>{log}</string>
    <key>StandardErrorPath</key>
    <string>{log}</string>
    <key>RunAtLoad</key>
    <false/>
    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>{path}</string>
    </dict>
</dict>
</plist>
"#,
        label = escape_xml(&label),
        bin = escape_xml(&maenifold_bin.display().to_string()),
        task_id = escape_xml(&task.id),
        calendar_xml = calendar_xml,
        log = escape_xml(&log_path.display().to_string()),
        path = DEFAULT_PATH,
    ))
}

fn default_log_path(task_id: &str) -> Result<PathBuf> {
    let home = std::env::var_os("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home).join(format!("Library/Logs/maenifold/schedule-{task_id}.log")))
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn is_not_loaded_or_not_found(error: &anyhow::Error) -> bool {
    let message = error.to_string().to_ascii_lowercase();
    [
        "launchctl unload failed: unload failed: 5: input/output error",
        "launchctl unload failed: could not find specified service",
        "launchctl unload failed: no such process",
        "launchctl unload failed: not loaded",
    ]
    .iter()
    .any(|needle| message.contains(needle))
}

#[cfg(test)]
#[path = "launchd_tests.rs"]
mod tests;

#[cfg(all(test, target_os = "macos"))]
mod isolation_tests {
    use super::*;
    use chrono::Utc;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    use tempfile::TempDir;

    struct CountingBackend {
        load_count: Arc<AtomicUsize>,
        unload_count: Arc<AtomicUsize>,
    }

    impl LaunchctlBackend for CountingBackend {
        fn load(&self, _plist_path: &Path) -> Result<()> {
            self.load_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn unload(&self, _plist_path: &Path) -> Result<()> {
            self.unload_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    fn fake_bin_in(temp: &TempDir) -> PathBuf {
        let bin = temp.path().join("fake-maenifold");
        fs::write(&bin, "#!/bin/sh\nexit 0\n").expect("write fake bin");
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&bin).expect("fake bin metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&bin, permissions).expect("set fake bin permissions");
        bin
    }

    fn task(task_id: &str) -> ScheduledTask {
        ScheduledTask {
            id: task_id.to_string(),
            name: task_id.to_string(),
            schedule: "0 9 * * 1-5".to_string(),
            schedule_human: Some("every weekday at 9am".to_string()),
            agent: "swe".to_string(),
            prompt: "Run scheduled maintenance".to_string(),
            workdir: None,
            auto_grant_permissions: false,
            allowed_tools: Vec::new(),
            config_options: serde_json::Map::new(),
            mcp_action: None,
            mcp_parameters: None,
            timeout_seconds: 300,
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_run: None,
        }
    }

    fn launch_agents_dir() -> PathBuf {
        PathBuf::from(std::env::var_os("HOME").expect("HOME should be set"))
            .join("Library/LaunchAgents")
    }

    fn host_schedule_plist_count() -> usize {
        let agents_dir = launch_agents_dir();
        fs::read_dir(&agents_dir)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| name.starts_with(LABEL_PREFIX) && name.ends_with(".plist"))
            .count()
    }

    #[test]
    fn install_and_uninstall_stay_within_temp_agents_dir() {
        let before_count = host_schedule_plist_count();
        let temp = TempDir::new().expect("temp dir");
        let fake_bin = fake_bin_in(&temp);
        let task_id = format!(
            "bluteam-isolation-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after epoch")
                .as_nanos()
        );
        let task = task(&task_id);
        let temp_agents_dir = temp.path().join("Library/LaunchAgents");
        let temp_plist_path = temp_agents_dir.join(format!("{LABEL_PREFIX}{task_id}.plist"));
        let host_plist_path = launch_agents_dir().join(format!("{LABEL_PREFIX}{task_id}.plist"));
        let load_count = Arc::new(AtomicUsize::new(0));
        let unload_count = Arc::new(AtomicUsize::new(0));
        let backend = CountingBackend {
            load_count: Arc::clone(&load_count),
            unload_count: Arc::clone(&unload_count),
        };
        let manager =
            LaunchdManager::new_with(temp_agents_dir.clone(), fake_bin, Arc::new(backend))
                .expect("create launchd manager");

        manager.install(&task).expect("install schedule");

        assert!(
            temp_plist_path.exists(),
            "temp plist should exist after install"
        );
        assert!(
            !host_plist_path.exists(),
            "host LaunchAgents should not contain test plist"
        );
        assert_eq!(load_count.load(Ordering::SeqCst), 1);

        manager.uninstall(&task_id).expect("uninstall schedule");

        assert!(
            !temp_plist_path.exists(),
            "temp plist should be removed after uninstall"
        );
        let temp_entries = fs::read_dir(&temp_agents_dir)
            .expect("temp agents dir should exist")
            .filter_map(|entry| entry.ok())
            .count();
        assert_eq!(temp_entries, 0, "temp agents dir should be clean");
        assert_eq!(unload_count.load(Ordering::SeqCst), 1);

        let after_count = host_schedule_plist_count();
        assert_eq!(
            after_count, before_count,
            "host LaunchAgents plist count should remain unchanged"
        );
    }
}
