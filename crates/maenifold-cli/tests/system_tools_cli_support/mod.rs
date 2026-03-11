#![allow(dead_code)]

// T-12.7: RTM NFR-1 - share system-tool CLI fixtures across the themed split test files.
use assert_cmd::Command;
use rusqlite::{Connection, params};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::TempDir;

pub(crate) fn isolated_root() -> TempDir {
    TempDir::new().expect("tmp")
}

pub(crate) fn maenifold_cmd(tmp: &TempDir) -> Command {
    // T-0.3g: RTM-047 - avoid deprecated assert_cmd constructor without changing test behavior.
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("maenifold"));
    cmd.env("MAENIFOLD_ROOT", tmp.path())
        .env("MAENIFOLD_DATABASE_PATH", tmp.path().join("memory.db"));
    cmd
}

pub(crate) fn write_markdown_file(tmp: &TempDir, relative: &str, content: &str) {
    let path = tmp.path().join("memory").join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
    fs::write(path, content).expect("write markdown");
}

pub(crate) fn seed_last_accessed(tmp: &TempDir, uri: &str, last_accessed: &str) {
    let db_path = tmp.path().join("memory.db");
    let connection = Connection::open(db_path).expect("open db");
    connection
        .execute(
            "INSERT OR REPLACE INTO file_content (file_path, last_indexed, last_accessed) VALUES (?1, ?2, ?3)",
            params![uri, "2026-03-08T00:00:00Z", last_accessed],
        )
        .expect("seed last_accessed");
}

pub(crate) fn write_json_asset(tmp: &TempDir, relative: &str, content: &str) {
    let path = tmp.path().join("assets").join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
    fs::write(path, content).expect("write asset");
}

pub(crate) fn write_concept_analysis_fixture(tmp: &TempDir) {
    write_markdown_file(
        tmp,
        "concept-analysis/test-note.md",
        r#"---
title: Test Note
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Test
This note discusses [[tool]], [[tool]], [[tools]], [[tools]], [[coding-tool]], [[tool system]], and [[GraphTools.cs]].
"#,
    );
}

pub(crate) fn render_json_or_text(output: Vec<u8>) -> String {
    let text = String::from_utf8(output).expect("utf8");
    match serde_json::from_str::<Value>(&text) {
        Ok(Value::String(rendered)) => rendered,
        Ok(value) => value.to_string(),
        Err(_) => text,
    }
}

pub(crate) fn render_json_data_or_text(output: Vec<u8>) -> String {
    let text = String::from_utf8(output).expect("utf8");
    match serde_json::from_str::<Value>(&text) {
        Ok(Value::String(rendered)) => rendered,
        Ok(value) => value
            .get("data")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| value.to_string()),
        Err(_) => text,
    }
}

pub(crate) fn test_mutex() -> &'static Mutex<()> {
    static TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_MUTEX.get_or_init(|| Mutex::new(()))
}

pub(crate) fn workspace_assets_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .join("assets")
}

pub(crate) fn unique_asset_relative(dir: &str, prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    format!("{dir}/{prefix}-{nanos}.json")
}

pub(crate) fn write_file(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
    fs::write(path, content).expect("write file");
}

fn remove_empty_ancestors(mut current: Option<&Path>, stop_at: &Path) {
    while let Some(path) = current {
        if path == stop_at {
            break;
        }
        match fs::remove_dir(path) {
            Ok(()) => current = path.parent(),
            Err(_) => break,
        }
    }
}

pub(crate) struct PackagedAssetGuard {
    path: PathBuf,
    assets_root: PathBuf,
}

impl PackagedAssetGuard {
    pub(crate) fn create(relative: &str, content: &str) -> Self {
        let assets_root = workspace_assets_root();
        let path = assets_root.join(relative);
        write_file(&path, content);
        Self { path, assets_root }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for PackagedAssetGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        remove_empty_ancestors(self.path.parent(), &self.assets_root);
    }
}
