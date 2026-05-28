use super::{validate_schedule_id, ScheduledTask};
#[path = "systemd_cron.rs"]
mod systemd_cron;
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use systemd_cron::cron_to_oncalendar;

const UNIT_PREFIX: &str = "maenifold-schedule-";
const DEFAULT_PATH: &str = "/usr/local/bin:/usr/bin:/bin";

pub trait SystemctlBackend: Send + Sync {
    fn daemon_reload(&self) -> Result<()>;
    fn enable(&self, unit: &str) -> Result<()>;
    fn disable(&self, unit: &str) -> Result<()>;
}

pub struct RealSystemctlBackend;

impl SystemctlBackend for RealSystemctlBackend {
    fn daemon_reload(&self) -> Result<()> {
        run_systemctl(["--user", "daemon-reload"], "daemon-reload")
    }

    fn enable(&self, unit: &str) -> Result<()> {
        run_systemctl(["--user", "enable", "--now", unit], "enable")
    }

    fn disable(&self, unit: &str) -> Result<()> {
        run_systemctl(["--user", "disable", "--now", unit], "disable")
    }
}

#[cfg(all(test, target_os = "linux"))]
pub(crate) struct NoopSystemctlBackend;

#[cfg(all(test, target_os = "linux"))]
impl SystemctlBackend for NoopSystemctlBackend {
    fn daemon_reload(&self) -> Result<()> {
        Ok(())
    }

    fn enable(&self, _unit: &str) -> Result<()> {
        Ok(())
    }

    fn disable(&self, _unit: &str) -> Result<()> {
        Ok(())
    }
}

#[cfg(all(test, target_os = "linux"))]
pub(crate) struct FailingSystemctlBackend;

#[cfg(all(test, target_os = "linux"))]
impl SystemctlBackend for FailingSystemctlBackend {
    fn daemon_reload(&self) -> Result<()> {
        bail!("simulated systemctl failure for test")
    }

    fn enable(&self, _unit: &str) -> Result<()> {
        bail!("simulated systemctl failure for test")
    }

    fn disable(&self, _unit: &str) -> Result<()> {
        bail!("simulated systemctl failure for test")
    }
}

pub struct SystemdManager {
    units_dir: PathBuf,
    maenifold_bin: PathBuf,
    backend: Arc<dyn SystemctlBackend>,
}

impl SystemdManager {
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
            PathBuf::from(home).join(".config/systemd/user"),
            maenifold_bin,
            Arc::new(RealSystemctlBackend),
        )
    }

    pub(crate) fn new_with(
        units_dir: PathBuf,
        maenifold_bin: PathBuf,
        backend: Arc<dyn SystemctlBackend>,
    ) -> Result<Self> {
        let maenifold_bin = if maenifold_bin.is_absolute() {
            maenifold_bin
        } else {
            std::env::current_dir()
                .context("failed to resolve current directory")?
                .join(maenifold_bin)
        };
        Ok(Self {
            units_dir,
            maenifold_bin,
            backend,
        })
    }

    pub fn install(&self, task: &ScheduledTask) -> Result<()> {
        validate_schedule_id(&task.id)?;
        fs::create_dir_all(&self.units_dir).with_context(|| {
            format!(
                "failed to create systemd user units directory '{}'",
                self.units_dir.display()
            )
        })?;
        match self.backend.disable(&timer_unit_name(&task.id)) {
            Ok(()) => {}
            Err(error) if is_unit_not_found(&error) => {}
            Err(error) => return Err(error),
        }
        fs::write(
            self.service_path(&task.id),
            render_service(task, &self.maenifold_bin)?,
        )
        .with_context(|| format!("failed to write service unit for '{}'", task.id))?;
        fs::write(self.timer_path(&task.id), render_timer(task)?)
            .with_context(|| format!("failed to write timer unit for '{}'", task.id))?;
        self.backend.daemon_reload()?;
        self.backend.enable(&timer_unit_name(&task.id))
    }

    pub fn uninstall(&self, task_id: &str) -> Result<()> {
        validate_schedule_id(task_id)?;
        let timer_path = self.timer_path(task_id);
        let service_path = self.service_path(task_id);
        if !timer_path.exists() && !service_path.exists() {
            return Ok(());
        }
        self.backend.disable(&timer_unit_name(task_id))?;
        if timer_path.exists() {
            fs::remove_file(&timer_path).with_context(|| {
                format!("failed to remove timer unit '{}'", timer_path.display())
            })?;
        }
        if service_path.exists() {
            fs::remove_file(&service_path).with_context(|| {
                format!("failed to remove service unit '{}'", service_path.display())
            })?;
        }
        self.backend.daemon_reload()
    }

    pub fn is_installed(&self, task_id: &str) -> bool {
        validate_schedule_id(task_id).is_ok()
            && self.timer_path(task_id).exists()
            && self.service_path(task_id).exists()
    }

    pub fn list_installed(&self) -> Result<Vec<String>> {
        if !self.units_dir.exists() {
            return Ok(Vec::new());
        }
        let mut task_ids = fs::read_dir(&self.units_dir)
            .with_context(|| format!("failed to read '{}'", self.units_dir.display()))?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter_map(|name| {
                name.strip_prefix(UNIT_PREFIX)
                    .and_then(|value| value.strip_suffix(".timer"))
                    .map(str::to_owned)
            })
            .collect::<Vec<_>>();
        task_ids.sort();
        task_ids.dedup();
        Ok(task_ids)
    }

    fn service_path(&self, task_id: &str) -> PathBuf {
        self.units_dir.join(service_unit_name(task_id))
    }

    fn timer_path(&self, task_id: &str) -> PathBuf {
        self.units_dir.join(timer_unit_name(task_id))
    }
}
fn render_timer(task: &ScheduledTask) -> Result<String> {
    let safe_name = sanitize_unit_value(&task.name);
    Ok(format!(
        "[Unit]\nDescription=Maenifold scheduled task: {}\n\n[Timer]\nOnCalendar={}\nPersistent=true\n\n[Install]\nWantedBy=timers.target\n",
        safe_name,
        cron_to_oncalendar(&task.schedule)?
    ))
}

fn render_service(task: &ScheduledTask, maenifold_bin: &Path) -> Result<String> {
    let safe_name = sanitize_unit_value(&task.name);
    let log_path = default_log_path(&task.id)?;
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create log directory '{}'", parent.display()))?;
    }
    Ok(format!(
        "[Unit]\nDescription=Maenifold scheduled task: {}\n\n[Service]\nType=oneshot\nExecStart={} schedule run {}\nStandardOutput=append:{}\nStandardError=append:{}\nEnvironment=PATH={}\n",
        safe_name,
        maenifold_bin.display(),
        task.id,
        log_path.display(),
        log_path.display(),
        DEFAULT_PATH
    ))
}
fn sanitize_unit_value(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).collect()
}
fn default_log_path(task_id: &str) -> Result<PathBuf> {
    let home = std::env::var_os("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home).join(format!(".local/state/maenifold/schedule-{task_id}.log")))
}

fn run_systemctl<const N: usize>(args: [&str; N], action: &str) -> Result<()> {
    let output = Command::new("systemctl")
        .args(args)
        .output()
        .with_context(|| format!("failed to run systemctl {action}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        bail!("systemctl {action} failed: {detail}");
    }
    Ok(())
}
fn is_unit_not_found(error: &anyhow::Error) -> bool {
    let message = error.to_string().to_ascii_lowercase();
    [
        "systemctl disable failed: unit file maenifold-schedule-",
        "systemctl disable failed: unit maenifold-schedule-",
        "systemctl disable failed: not loaded",
        "systemctl disable failed: not found",
    ]
    .iter()
    .any(|needle| message.contains(needle))
}
fn timer_unit_name(task_id: &str) -> String {
    format!("{UNIT_PREFIX}{task_id}.timer")
}
fn service_unit_name(task_id: &str) -> String {
    format!("{UNIT_PREFIX}{task_id}.service")
}

#[cfg(test)]
mod tests {
    use super::cron_to_oncalendar;

    #[test]
    fn converts_weekday_schedule() {
        let calendar = cron_to_oncalendar("0 9 * * 1-5").expect("cron should convert");
        assert_eq!(calendar, "Mon..Fri *-*-* 09:00:00");
    }

    #[test]
    fn converts_daily_midnight_schedule() {
        let calendar = cron_to_oncalendar("0 0 * * *").expect("cron should convert");
        assert_eq!(calendar, "*-*-* 00:00:00");
    }
    #[test]
    fn converts_monthly_schedule() {
        let calendar = cron_to_oncalendar("0 12 1 * *").expect("cron should convert");
        assert_eq!(calendar, "*-*-01 12:00:00");
    }
}

#[cfg(all(test, target_os = "linux"))]
mod isolation_tests {
    use super::*;
    use chrono::Utc;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tempfile::TempDir;

    struct CountingSystemctlBackend {
        daemon_reload_count: Arc<AtomicUsize>,
        enable_count: Arc<AtomicUsize>,
        disable_count: Arc<AtomicUsize>,
    }

    impl SystemctlBackend for CountingSystemctlBackend {
        fn daemon_reload(&self) -> Result<()> {
            self.daemon_reload_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn enable(&self, _unit: &str) -> Result<()> {
            self.enable_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn disable(&self, _unit: &str) -> Result<()> {
            self.disable_count.fetch_add(1, Ordering::SeqCst);
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
            mcp_action: None,
            mcp_parameters: None,
            timeout_seconds: 300,
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_run: None,
        }
    }

    fn user_units_dir() -> PathBuf {
        PathBuf::from(std::env::var_os("HOME").expect("HOME should be set"))
            .join(".config/systemd/user")
    }

    fn host_schedule_unit_count() -> usize {
        let units_dir = user_units_dir();
        fs::read_dir(&units_dir)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| {
                name.starts_with(UNIT_PREFIX)
                    && (name.ends_with(".timer") || name.ends_with(".service"))
            })
            .count()
    }

    #[test]
    fn install_and_uninstall_stay_within_temp_units_dir() {
        let before_count = host_schedule_unit_count();
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
        let temp_units_dir = temp.path().join(".config/systemd/user");
        let temp_timer_path = temp_units_dir.join(timer_unit_name(&task_id));
        let temp_service_path = temp_units_dir.join(service_unit_name(&task_id));
        let host_timer_path = user_units_dir().join(timer_unit_name(&task_id));
        let host_service_path = user_units_dir().join(service_unit_name(&task_id));
        let daemon_reload_count = Arc::new(AtomicUsize::new(0));
        let enable_count = Arc::new(AtomicUsize::new(0));
        let disable_count = Arc::new(AtomicUsize::new(0));
        let backend = CountingSystemctlBackend {
            daemon_reload_count: Arc::clone(&daemon_reload_count),
            enable_count: Arc::clone(&enable_count),
            disable_count: Arc::clone(&disable_count),
        };
        let manager = SystemdManager::new_with(temp_units_dir.clone(), fake_bin, Arc::new(backend))
            .expect("create systemd manager");

        manager.install(&task).expect("install schedule");

        assert!(
            temp_timer_path.exists(),
            "temp timer should exist after install"
        );
        assert!(
            temp_service_path.exists(),
            "temp service should exist after install"
        );
        assert!(
            !host_timer_path.exists(),
            "host user units should not contain test timer"
        );
        assert!(
            !host_service_path.exists(),
            "host user units should not contain test service"
        );
        assert_eq!(daemon_reload_count.load(Ordering::SeqCst), 1);
        assert_eq!(enable_count.load(Ordering::SeqCst), 1);
        assert_eq!(disable_count.load(Ordering::SeqCst), 1);

        manager.uninstall(&task_id).expect("uninstall schedule");

        assert!(
            !temp_timer_path.exists(),
            "temp timer should be removed after uninstall"
        );
        assert!(
            !temp_service_path.exists(),
            "temp service should be removed after uninstall"
        );
        let temp_entries = fs::read_dir(&temp_units_dir)
            .expect("temp units dir should exist")
            .filter_map(|entry| entry.ok())
            .count();
        assert_eq!(temp_entries, 0, "temp units dir should be clean");
        assert_eq!(daemon_reload_count.load(Ordering::SeqCst), 2);
        assert_eq!(enable_count.load(Ordering::SeqCst), 1);
        assert_eq!(disable_count.load(Ordering::SeqCst), 2);

        let after_count = host_schedule_unit_count();
        assert_eq!(
            after_count, before_count,
            "host user unit count should remain unchanged"
        );
    }
}

#[cfg(test)]
mod cross_platform_trait_tests {
    use super::*;
    use chrono::Utc;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tempfile::TempDir;

    struct MockBackend {
        daemon_reload_calls: Arc<AtomicUsize>,
        enable_calls: Arc<AtomicUsize>,
        disable_calls: Arc<AtomicUsize>,
    }

    impl SystemctlBackend for MockBackend {
        fn daemon_reload(&self) -> Result<()> {
            self.daemon_reload_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn enable(&self, _unit: &str) -> Result<()> {
            self.enable_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn disable(&self, _unit: &str) -> Result<()> {
            self.disable_calls.fetch_add(1, Ordering::SeqCst);
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
            mcp_action: None,
            mcp_parameters: None,
            timeout_seconds: 300,
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_run: None,
        }
    }

    fn user_units_dir() -> PathBuf {
        PathBuf::from(std::env::var_os("HOME").expect("HOME should be set"))
            .join(".config/systemd/user")
    }

    fn host_schedule_unit_count() -> usize {
        let units_dir = user_units_dir();
        fs::read_dir(&units_dir)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| {
                name.starts_with(UNIT_PREFIX)
                    && (name.ends_with(".timer") || name.ends_with(".service"))
            })
            .count()
    }

    #[test]
    fn install_uninstall_routes_through_backend_and_writes_to_temp_units_dir() {
        let before_count = host_schedule_unit_count();
        let temp = TempDir::new().expect("temp dir");
        let fake_bin = fake_bin_in(&temp);
        let temp_units_dir = temp.path().join(".config/systemd/user");
        let task = task("alpha-test-1");
        let temp_timer_path = temp_units_dir.join(timer_unit_name(&task.id));
        let temp_service_path = temp_units_dir.join(service_unit_name(&task.id));
        let host_timer_path = user_units_dir().join(timer_unit_name(&task.id));
        let host_service_path = user_units_dir().join(service_unit_name(&task.id));
        let daemon_reload_calls = Arc::new(AtomicUsize::new(0));
        let enable_calls = Arc::new(AtomicUsize::new(0));
        let disable_calls = Arc::new(AtomicUsize::new(0));
        let backend = MockBackend {
            daemon_reload_calls: Arc::clone(&daemon_reload_calls),
            enable_calls: Arc::clone(&enable_calls),
            disable_calls: Arc::clone(&disable_calls),
        };
        let manager = SystemdManager::new_with(temp_units_dir.clone(), fake_bin, Arc::new(backend))
            .expect("create systemd manager");

        manager.install(&task).expect("install schedule");

        assert!(
            temp_timer_path.exists(),
            "temp timer should exist after install"
        );
        assert!(
            temp_service_path.exists(),
            "temp service should exist after install"
        );
        assert!(
            !host_timer_path.exists(),
            "host user units should not contain test timer"
        );
        assert!(
            !host_service_path.exists(),
            "host user units should not contain test service"
        );
        assert_eq!(daemon_reload_calls.load(Ordering::SeqCst), 1);
        assert_eq!(enable_calls.load(Ordering::SeqCst), 1);
        assert_eq!(disable_calls.load(Ordering::SeqCst), 1);

        manager.uninstall(&task.id).expect("uninstall schedule");

        assert!(
            !temp_timer_path.exists(),
            "temp timer should be removed after uninstall"
        );
        assert!(
            !temp_service_path.exists(),
            "temp service should be removed after uninstall"
        );
        let temp_entries = fs::read_dir(&temp_units_dir)
            .expect("temp units dir should exist")
            .filter_map(|entry| entry.ok())
            .count();
        assert_eq!(temp_entries, 0, "temp units dir should be clean");
        assert_eq!(disable_calls.load(Ordering::SeqCst), 2);
        assert_eq!(daemon_reload_calls.load(Ordering::SeqCst), 2);
        assert_eq!(enable_calls.load(Ordering::SeqCst), 1);

        let after_count = host_schedule_unit_count();
        assert_eq!(
            before_count, after_count,
            "host user unit count should remain unchanged"
        );
    }
}
