use super::{ScheduledTask, validate_schedule_id};
#[path = "launchd_cron.rs"]
mod launchd_cron;
use anyhow::{Context, Result, bail};
use launchd_cron::cron_to_start_calendar_interval_xml;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const LABEL_PREFIX: &str = "com.maenifold.schedule.";
const DEFAULT_PATH: &str = "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin";

pub struct LaunchdManager {
    agents_dir: PathBuf,
    maenifold_bin: PathBuf,
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
        Ok(Self {
            agents_dir: PathBuf::from(home).join("Library/LaunchAgents"),
            maenifold_bin,
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
            match launchctl_unload(&plist_path) {
                Ok(()) => {}
                Err(error) if is_not_loaded_or_not_found(&error) => {}
                Err(error) => return Err(error),
            }
        }
        fs::write(&plist_path, render_plist(task, &self.maenifold_bin)?)
            .with_context(|| format!("failed to write plist '{}'", plist_path.display()))?;
        launchctl_load(&plist_path)
    }

    pub fn uninstall(&self, task_id: &str) -> Result<()> {
        validate_schedule_id(task_id)?;
        let plist_path = self.plist_path(task_id);
        if plist_path.exists() {
            launchctl_unload(&plist_path)?;
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

fn launchctl_load(plist_path: &Path) -> Result<()> {
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

fn launchctl_unload(plist_path: &Path) -> Result<()> {
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
