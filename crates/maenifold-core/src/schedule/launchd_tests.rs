use super::*;
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
#[test]
fn omits_wildcards() {
    let xml = cron_to_start_calendar_interval_xml("15 9 * * *").expect("cron should render");
    assert_eq!(
        xml,
        "<dict><key>Hour</key><integer>9</integer><key>Minute</key><integer>15</integer></dict>"
    );
}
#[test]
fn expands_multi_value_fields() {
    let xml = cron_to_start_calendar_interval_xml("0 9,17 * * 1,5").expect("cron should render");
    assert!(xml.starts_with("<array>"));
    assert!(xml.contains("<key>Weekday</key><integer>1</integer>"));
    assert!(xml.contains("<key>Weekday</key><integer>5</integer>"));
}
/// plutil only ships with macOS; gate the syntactic validity check to macOS.
#[cfg(target_os = "macos")]
#[test]
fn renders_valid_weekday_plist_xml() {
    let plist = render_plist(&task("0 9 * * 1-5"), std::path::Path::new("/bin/maenifold"))
        .expect("weekday plist should render");
    let file = tempfile::NamedTempFile::new().expect("temp file should exist");
    std::fs::write(file.path(), &plist).expect("plist should write");
    assert!(
        std::process::Command::new("plutil")
            .args(["-lint", file.path().to_str().expect("utf-8 path")])
            .output()
            .expect("plutil should run")
            .status
            .success()
    );
    assert_eq!(plist.matches("<key>Weekday</key>").count(), 5);
    for day in 1..=5 {
        assert!(plist.contains(&format!("<key>Weekday</key><integer>{day}</integer>")));
    }
}
#[test]
fn expands_wildcard_steps_into_minutes() {
    let xml =
        cron_to_start_calendar_interval_xml("*/15 * * * *").expect("stepped cron should render");
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
