#![allow(dead_code, unused_imports)]

use maenifold_core::json::OutputFormat;
use maenifold_core::tools::ToolContext;
use maenifold_core::{AssetManager, Config, repair};
use std::fs;
use tempfile::TempDir;

// T-1.4: RTM FR-2
pub(crate) use crate::GLOBAL_CONFIG_TEST_MUTEX as TEST_MUTEX;

pub(crate) const CONCEPT_ANALYSIS_FIXTURE: &str = r#"---
title: Test Note
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Test
This note discusses [[tool]], [[tool]], [[tools]], [[tools]], [[coding-tool]], [[tool system]], and [[GraphTools.cs]].
"#;

pub(crate) struct ConfigOverrideReset;

impl Drop for ConfigOverrideReset {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

pub(crate) fn isolated_config() -> (TempDir, Config) {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    AssetManager::initialize_assets(&cfg).expect("assets");
    (tmp, cfg)
}

pub(crate) fn write_memory_fixture(cfg: &Config, relative_path: &str, content: &str) {
    let path = cfg.memory_path.join(relative_path);
    fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
    fs::write(path, content).expect("write memory fixture");
}

pub(crate) fn analyze_concept_corruption(cfg: &Config, payload: serde_json::Value) -> String {
    repair::analyze_concept_corruption(
        &ToolContext::new(cfg.clone(), OutputFormat::Markdown),
        &payload,
    )
    .expect("analyze concept corruption")
}

/// Fallible variant for testing error paths (missing/empty params, invalid maxResults).
pub(crate) fn try_analyze_concept_corruption(
    cfg: &Config,
    payload: serde_json::Value,
) -> Result<String, String> {
    repair::analyze_concept_corruption(
        &ToolContext::new(cfg.clone(), OutputFormat::Markdown),
        &payload,
    )
    .map_err(|e| e.to_string())
}

pub(crate) fn report_body_before_repairs(output: &str) -> &str {
    // Match the "=== SUGGESTED" prefix pattern to be resilient to header renames
    // (e.g., "=== SUGGESTED REPAIRS ===" → "=== SUGGESTED FIXES ===").
    output
        .find("=== SUGGESTED")
        .map_or(output, |idx| &output[..idx])
}

pub(crate) fn bullet_count(output: &str) -> usize {
    output.lines().filter(|line| line.contains("  • ")).count()
}
