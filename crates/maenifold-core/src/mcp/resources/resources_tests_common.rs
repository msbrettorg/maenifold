#![allow(dead_code, unused_imports)]

pub use super::*;
pub use crate::config::Config;
pub use crate::registry::ToolRegistry;
pub(crate) use crate::test_util::ConfigResetGuard;
pub use crate::util::json::OutputFormat;
pub use crate::workflow::pipeline::pipeline_lock_path;
pub use chrono::{Datelike, TimeZone};
pub use serde_json::json;
pub use serial_test::serial;
pub use std::fs;
pub use std::io::Write;
pub use std::sync::atomic::{AtomicUsize, Ordering};
pub use std::sync::Arc;
pub use std::thread;
pub use tempfile::TempDir;

pub fn setup_test_env() -> (TempDir, ConfigResetGuard) {
        let tmp = TempDir::new().expect("temp dir");
        Config::override_root(tmp.path());
        Config::set_database_path(tmp.path().join("memory.db"));
        let config = Config::current();
        config.ensure_directories().expect("ensure directories");
        
        // Create pipeline directory structure
        let pipelines_dir = config.memory_path.join("pipelines/2026/04/22");
        fs::create_dir_all(&pipelines_dir).expect("create pipelines dir");
        
        // Create asset directories
        for kind in ["workflows", "roles", "colors", "perspectives"] {
            fs::create_dir_all(config.assets_path.join(kind)).expect("create asset dir");
        }
        
        // Create usage directory for Pipeline tool
        fs::create_dir_all(config.assets_path.join("usage/tools")).expect("create usage dir");
        fs::write(
            config.assets_path.join("usage/tools/pipeline.md"),
            "# Pipeline\n\nPipeline tool documentation.\n",
        )
        .expect("write pipeline help");
        
        (tmp, ConfigResetGuard)
    }

pub fn create_pipeline_session(config: &Config, session_id: &str) {
        // Extract timestamp from session ID to compute the correct directory path
        // The session ID format is "pipeline-{timestamp_millis}-{suffix}"
        // e.g., "pipeline-1745366400000-test1" -> 1745366400000ms -> 2025-04-22
        let timestamp_millis = session_id
            .split('-')
            .find_map(|segment| {
                if segment.len() == 13 && segment.bytes().all(|b| b.is_ascii_digit()) {
                    segment.parse::<i64>().ok()
                } else {
                    None
                }
            })
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
        
        let dt = chrono::Utc
            .timestamp_millis_opt(timestamp_millis)
            .single()
            .unwrap_or_else(chrono::Utc::now);
        let year = dt.year();
        let month = dt.month();
        let day = dt.day();
        
        // Create workflow session for the node (referenced by nodeStates.sessionId)
        let workflow_session_id = format!("workflow-{timestamp_millis}-node1");
        let workflow_dir = config.memory_path.join(format!("thinking/workflow/{year:04}/{month:02}/{day:02}"));
        fs::create_dir_all(&workflow_dir).expect("create workflow dir");
        let workflow_path = workflow_dir.join(format!("{workflow_session_id}.md"));
        let workflow_content = r#"---
type: workflow
status: running
---

# Workflow Session

Test workflow session for pipeline node.
"#;
        fs::write(&workflow_path, workflow_content).expect("write workflow session");
        
        // Create pipeline session
        let pipelines_dir = config.memory_path.join(format!("pipelines/{year:04}/{month:02}/{day:02}"));
        fs::create_dir_all(&pipelines_dir).expect("create pipelines dir");
        
        let session_path = pipelines_dir.join(format!("{session_id}.md"));
        let frontmatter = format!(
            r#"---
type: pipeline
permalink: {session_id}
status: running
definition:
  nodes:
    - id: node-1
      type: prompt
      agent: test
      prompt: test
  edges: []
nodeStates:
  node-1:
    sessionId: {workflow_session_id}
    status: active
    attempts: 1
---

# Pipeline Session

Test pipeline session for concurrency testing.
"#
        );
        fs::write(&session_path, frontmatter).expect("write pipeline session");
    }
