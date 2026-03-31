#![allow(dead_code, unused_imports)]

use maenifold_core::json::OutputFormat;
use maenifold_core::tools::ToolContext;
use maenifold_core::{AssetManager, Config, GraphDatabase, MaenifoldError, ToolRegistry};

pub use maenifold_core::memory;
use serde_json::Value;
use serde_yaml::Value as YamlValue;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use walkdir::WalkDir;

pub use maenifold_core::markdown;

pub(crate) use crate::GLOBAL_CONFIG_TEST_MUTEX as TEST_MUTEX;

pub struct TestEnv {
    _tmp: TempDir,
    pub cfg: Config,
}

impl TestEnv {
    pub fn ctx(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Markdown)
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

// T-12.7: RTM NFR-1 - share markdown boundary helpers while splitting oversized coverage.
pub fn isolated_env() -> TestEnv {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    AssetManager::initialize_assets(&cfg).expect("assets");
    GraphDatabase::initialize_database(&cfg).expect("db init");
    TestEnv { _tmp: tmp, cfg }
}

pub fn seed_file(
    cfg: &Config,
    uri: &str,
    frontmatter: Option<&markdown::Frontmatter>,
    content: &str,
) {
    markdown::write_markdown(uri, frontmatter, content, cfg).expect("seed markdown");
}

pub fn write_markdown_boundary(
    uri: Option<&str>,
    frontmatter: Option<&markdown::Frontmatter>,
    content: &str,
    cfg: &Config,
) -> maenifold_core::Result<()> {
    match uri {
        Some(uri) => markdown::write_markdown(uri, frontmatter, content, cfg),
        None => Err(MaenifoldError::InvalidArgument(
            "uri: URI cannot be null or empty.".to_string(),
        )),
    }
}

pub fn get_memory_directories(
    cfg: &Config,
    folder_uri: Option<&str>,
) -> maenifold_core::Result<Vec<String>> {
    let mut directories = BTreeSet::new();
    for file_uri in markdown::get_memory_files(cfg, folder_uri)? {
        let path = markdown::uri_to_path(&file_uri, &cfg.memory_path)?;
        let Some(parent) = path.parent() else {
            continue;
        };
        if parent == cfg.memory_path {
            continue;
        }
        directories.insert(markdown::path_to_uri(parent, &cfg.memory_path)?);
    }
    Ok(directories.into_iter().collect())
}

pub fn yaml_string(frontmatter: &markdown::Frontmatter, key: &str) -> Option<String> {
    frontmatter.get(key).and_then(|value| match value {
        YamlValue::String(text) => Some(text.clone()),
        other => serde_yaml::to_string(other)
            .ok()
            .map(|text| text.trim().to_string()),
    })
}

pub fn assert_invalid_argument(err: MaenifoldError, expected_fragment: &str) {
    match err {
        MaenifoldError::InvalidArgument(message) => assert!(
            message
                .to_ascii_lowercase()
                .contains(&expected_fragment.to_ascii_lowercase()),
            "expected invalid argument containing {expected_fragment:?}, got {message:?}"
        ),
        other => panic!("expected invalid argument, got {other:?}"),
    }
}

pub fn assert_message_error(err: MaenifoldError, expected_fragment: &str) {
    match err {
        MaenifoldError::Message(message) => assert!(
            message.contains(expected_fragment),
            "expected message containing {expected_fragment:?}, got {message:?}"
        ),
        other => panic!("expected message error, got {other:?}"),
    }
}

pub fn read_crate_sources() -> String {
    let src_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut combined = String::new();
    for entry in WalkDir::new(src_root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("rs"))
    {
        combined.push_str(&fs::read_to_string(entry.path()).expect("read source file"));
        // T-12.7: RTM NFR-1 - preserve fixture source boundaries with an escaped newline separator.
        combined.push('\n');
    }
    combined
}

pub fn invoke_repair_concepts(payload: Value) -> String {
    ToolRegistry::global()
        .try_invoke("RepairConcepts", &payload, OutputFormat::Markdown)
        .expect("invoke repair concepts")
}
