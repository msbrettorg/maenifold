use super::{ScheduledTask, validate_schedule_id};
#[path = "systemd_cron.rs"]
mod systemd_cron;
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use systemd_cron::cron_to_oncalendar;
const UNIT_PREFIX: &str = "maenifold-schedule-";
const DEFAULT_PATH: &str = "/usr/local/bin:/usr/bin:/bin";

pub struct SystemdManager {
    units_dir: PathBuf,
    maenifold_bin: PathBuf,
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
        Ok(Self {
            units_dir: PathBuf::from(home).join(".config/systemd/user"),
            maenifold_bin,
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
        match systemctl_disable(&timer_unit_name(&task.id)) {
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
        systemctl_daemon_reload()?;
        systemctl_enable(&timer_unit_name(&task.id))
    }

    pub fn uninstall(&self, task_id: &str) -> Result<()> {
        validate_schedule_id(task_id)?;
        let timer_path = self.timer_path(task_id);
        let service_path = self.service_path(task_id);
        if !timer_path.exists() && !service_path.exists() {
            return Ok(());
        }
        systemctl_disable(&timer_unit_name(task_id))?;
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
        systemctl_daemon_reload()
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
fn systemctl_enable(unit: &str) -> Result<()> {
    run_systemctl(["--user", "enable", "--now", unit], "enable")
}
fn systemctl_disable(unit: &str) -> Result<()> {
    run_systemctl(["--user", "disable", "--now", unit], "disable")
}
fn systemctl_daemon_reload() -> Result<()> {
    run_systemctl(["--user", "daemon-reload"], "daemon-reload")
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
