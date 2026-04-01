#![allow(dead_code, unused_imports)]

use maenifold_core::json::OutputFormat;
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase, memory, repair};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

// T-0.1: RTM NFR-3 — Shared harness preserves memory path coverage while splitting the oversized test file.
pub(crate) use crate::GLOBAL_CONFIG_TEST_MUTEX as TEST_MUTEX;
pub(crate) const TEST_FOLDER: &str = "memory-path-tests";

pub(crate) struct TestEnv {
    _tmp: TempDir,
    pub(crate) cfg: Config,
}

impl TestEnv {
    pub(crate) fn ctx(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Markdown)
    }

    pub(crate) fn test_folder_path(&self) -> std::path::PathBuf {
        self.cfg.memory_path.join(TEST_FOLDER)
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

pub(crate) fn isolated_env() -> TestEnv {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    GraphDatabase::initialize_database(&cfg).expect("db init");
    fs::create_dir_all(cfg.memory_path.join(TEST_FOLDER)).expect("create test folder");
    TestEnv { _tmp: tmp, cfg }
}

pub(crate) fn write_memory_output(
    env: &TestEnv,
    title: &str,
    content: &str,
    folder: Option<&str>,
) -> String {
    memory::write_memory(
        &env.ctx(),
        &json!({
            "title": title,
            "content": content,
            "folder": folder,
        }),
    )
    .expect("write memory")
}

pub(crate) fn read_memory_output(
    env: &TestEnv,
    identifier: &str,
) -> Result<String, maenifold_core::MaenifoldError> {
    memory::read_memory(&env.ctx(), &json!({ "identifier": identifier }))
}

pub(crate) fn move_memory_output(
    env: &TestEnv,
    source: &str,
    destination: &str,
) -> Result<String, maenifold_core::MaenifoldError> {
    memory::move_memory(
        &env.ctx(),
        &json!({
            "source": source,
            "destination": destination,
        }),
    )
}

pub(crate) fn extract_concepts_output(
    env: &TestEnv,
    identifier: &str,
) -> Result<String, maenifold_core::MaenifoldError> {
    repair::extract_concepts_from_file(&env.ctx(), &json!({ "identifier": identifier }))
}
