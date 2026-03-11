// T-12.7: RTM NFR-1
#![allow(dead_code)]

use chrono::{DateTime, Timelike, Utc};
use filetime::{FileTime, set_file_mtime};
use maenifold_core::Config;
use maenifold_core::markdown::{Frontmatter, path_to_uri, write_markdown};
use serde_yaml::Value as YamlValue;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration as StdDuration, SystemTime, UNIX_EPOCH};

use crate::support::{path_for_uri, session_uri};

pub(crate) fn create_normal_memory_file(
    cfg: &Config,
    relative_path: &str,
    content: &str,
) -> (PathBuf, String) {
    let path = cfg.memory_path.join(relative_path);
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(&path, content).expect("write memory file");
    let uri = path_to_uri(&path, &cfg.memory_path).expect("path to uri");
    (path, uri)
}

pub(crate) fn set_file_mtime_utc(path: &Path, instant: DateTime<Utc>) {
    set_file_mtime(
        path,
        FileTime::from_unix_time(instant.timestamp(), instant.timestamp_subsec_nanos()),
    )
    .expect("set mtime");
}

pub(crate) fn file_mtime_rfc3339(path: &Path) -> String {
    let modified: DateTime<Utc> = fs::metadata(path)
        .expect("metadata")
        .modified()
        .expect("modified")
        .into();
    modified.to_rfc3339()
}

pub(crate) fn align_to_second_boundary() {
    let nanos = Utc::now().nanosecond();
    if nanos > 900_000_000 {
        thread::sleep(StdDuration::from_millis(150));
        return;
    }
    let wait = 1_000_000_000u32 - nanos + 20_000_000;
    thread::sleep(StdDuration::from_nanos(u64::from(wait)));
}

pub(crate) fn create_workflow_session_fixture(
    cfg: &Config,
    session_type: &str,
    status: &str,
) -> String {
    let millis = Utc::now().timestamp_millis();
    let session_id = match session_type {
        "workflow" => format!("workflow-{millis}"),
        "sequential" => {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos() as u64;
            format!("session-{millis}-{}", 10_000 + (nanos % 90_000))
        }
        other => panic!("unsupported session type: {other}"),
    };
    let uri = session_uri(session_type, &session_id);
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String(format!("Test {session_type} session {session_id}")),
    );
    frontmatter.insert(
        "permalink".to_string(),
        YamlValue::String(session_id.clone()),
    );
    frontmatter.insert(
        "type".to_string(),
        YamlValue::String(session_type.to_string()),
    );
    frontmatter.insert("status".to_string(), YamlValue::String(status.to_string()));
    let body = if session_type == "workflow" {
        "# Workflow Session\n\n## Step 1\n\nOld [[Session Cleanup]] workflow\n"
    } else {
        "# Session\n\n## Thought 1\nOld [[Session Cleanup]] session\n"
    };
    write_markdown(&uri, Some(&frontmatter), body, cfg).expect("write fixture session");
    let _ = path_for_uri(cfg, &uri);
    session_id
}
