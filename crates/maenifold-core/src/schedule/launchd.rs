use super::ScheduledTask;
use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
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

fn cron_to_start_calendar_interval_xml(cron: &str) -> Result<String> {
    let mut parts = cron.split_whitespace();
    let fields = [
        parts
            .next()
            .context("cron expression is missing minute field")?,
        parts
            .next()
            .context("cron expression is missing hour field")?,
        parts
            .next()
            .context("cron expression is missing day-of-month field")?,
        parts
            .next()
            .context("cron expression is missing month field")?,
        parts
            .next()
            .context("cron expression is missing day-of-week field")?,
    ];
    if parts.next().is_some() {
        bail!("launchd requires 5-field cron expressions");
    }
    let expanded = [
        ("Minute", expand_field(fields[0], 0, 59, false)?),
        ("Hour", expand_field(fields[1], 0, 23, false)?),
        ("Day", expand_field(fields[2], 1, 31, false)?),
        ("Month", expand_field(fields[3], 1, 12, false)?),
        ("Weekday", expand_field(fields[4], 0, 7, true)?),
    ];
    let schedules = expanded
        .iter()
        .fold(vec![BTreeMap::new()], |acc, (key, values)| {
            if values.is_empty() {
                return acc;
            }
            acc.into_iter()
                .flat_map(|base| {
                    values.iter().map(move |value| {
                        let mut next = base.clone();
                        next.insert(*key, *value);
                        next
                    })
                })
                .collect::<Vec<_>>()
        });

    if schedules.is_empty() {
        bail!("cron expression produced no launchd calendar intervals");
    }
    Ok(if schedules.len() == 1 {
        render_calendar_dict(&schedules[0])
    } else {
        let body = schedules
            .iter()
            .map(render_calendar_dict)
            .collect::<String>();
        format!("<array>{body}</array>")
    })
}

fn expand_field(expr: &str, min: u32, max: u32, normalize_sunday: bool) -> Result<Vec<u32>> {
    if expr == "*" {
        return Ok(Vec::new());
    }
    let mut values = Vec::new();
    for segment in expr.split(',') {
        let (base, step) = match segment.split_once('/') {
            Some((base, step)) => {
                let step = parse_number(step, min, max)?;
                if step == 0 {
                    bail!("cron step value cannot be zero in '{segment}'");
                }
                (base, step)
            }
            None => (segment, 1),
        };
        let (start, end) = if base == "*" {
            (min, max)
        } else if let Some((start, end)) = base.split_once('-') {
            let start = parse_number(start, min, max)?;
            let end = parse_number(end, min, max)?;
            if start > end {
                bail!("invalid cron range '{segment}'");
            }
            (start, end)
        } else {
            let value = parse_number(base, min, max)?;
            (value, value)
        };
        let mut value = start;
        while value <= end {
            let normalized = if normalize_sunday && value == 7 {
                0
            } else {
                value
            };
            if !values.contains(&normalized) {
                values.push(normalized);
            }
            if end - value < step {
                break;
            }
            value += step;
        }
    }
    values.sort_unstable();
    Ok(values)
}

fn parse_number(input: &str, min: u32, max: u32) -> Result<u32> {
    let value = input
        .parse::<u32>()
        .with_context(|| format!("invalid cron value '{input}'"))?;
    if !(min..=max).contains(&value) {
        bail!("cron value '{input}' is out of range {min}-{max}");
    }
    Ok(value)
}

fn render_calendar_dict(fields: &BTreeMap<&str, u32>) -> String {
    let body = fields
        .iter()
        .map(|(key, value)| format!("<key>{key}</key><integer>{value}</integer>"))
        .collect::<String>();
    format!("<dict>{body}</dict>")
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn validate_schedule_id(id: &str) -> Result<()> {
    if id.is_empty() || id.len() > 64 {
        bail!("schedule id must be between 1 and 64 characters");
    }
    if !id.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-') {
        bail!("schedule id must contain only alphanumeric characters or hyphens");
    }
    Ok(())
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
mod tests {
    use super::{cron_to_start_calendar_interval_xml, render_plist};
    use crate::schedule::ScheduledTask;
    use chrono::Utc;
    fn task(schedule: &str) -> ScheduledTask {
        ScheduledTask {
            id: "weekday-job".into(),
            name: "Weekday job".into(),
            schedule: schedule.into(),
            schedule_human: None,
            agent: "agent".into(),
            prompt: "prompt".into(),
            workdir: None,
            timeout_seconds: 300,
            skip_if_running: true,
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_run: None,
        }
    }
    #[test]
    fn omits_wildcards() {
        let xml = cron_to_start_calendar_interval_xml("15 9 * * *").expect("cron should render");
        assert_eq!(xml, "<dict><key>Hour</key><integer>9</integer><key>Minute</key><integer>15</integer></dict>");
    }
    #[test]
    fn expands_multi_value_fields() {
        let xml =
            cron_to_start_calendar_interval_xml("0 9,17 * * 1,5").expect("cron should render");
        assert!(xml.starts_with("<array>"));
        assert!(xml.contains("<key>Weekday</key><integer>1</integer>"));
        assert!(xml.contains("<key>Weekday</key><integer>5</integer>"));
    }
    #[test]
    fn renders_valid_weekday_plist_xml() {
        let plist = render_plist(&task("0 9 * * 1-5"), std::path::Path::new("/bin/maenifold"))
            .expect("weekday plist should render");
        let file = tempfile::NamedTempFile::new().expect("temp file should exist");
        std::fs::write(file.path(), &plist).expect("plist should write");
        assert!(std::process::Command::new("plutil")
            .args(["-lint", file.path().to_str().expect("utf-8 path")])
            .output()
            .expect("plutil should run")
            .status
            .success());
        assert_eq!(plist.matches("<key>Weekday</key>").count(), 5);
        for day in 1..=5 {
            assert!(plist.contains(&format!("<key>Weekday</key><integer>{day}</integer>")));
        }
    }
    #[test]
    fn expands_wildcard_steps_into_minutes() {
        let xml = cron_to_start_calendar_interval_xml("*/15 * * * *")
            .expect("stepped cron should render");
        assert!(xml.starts_with("<array>"));
        assert_eq!(xml.matches("<key>Minute</key>").count(), 4);
        for minute in [0, 15, 30, 45] {
            assert!(xml.contains(&format!("<key>Minute</key><integer>{minute}</integer>")));
        }
        assert!(!xml.contains("<key>Hour</key>"));
    }
    #[test]
    fn rejects_invalid_cron_for_plist_generation() {
        let error = render_plist(&task("* * *"), std::path::Path::new("/bin/maenifold"))
            .expect_err("invalid cron should fail");
        assert!(error.to_string().contains("missing"));
    }
}
