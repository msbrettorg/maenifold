use std::collections::BTreeMap;

use serde_yml::Value as YamlValue;
use tempfile::TempDir;

use crate::config::Config;
use crate::markdown::{Frontmatter, write_markdown};
use crate::test_util::ConfigResetGuard;
use crate::thinking::sequential_session_uri;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;

pub(super) fn test_context() -> (TempDir, ToolContext, ConfigResetGuard) {
    let tmp = TempDir::new().expect("temp dir");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let config = Config::current();
    config.ensure_directories().expect("ensure directories");
    (
        tmp,
        ToolContext::new(config, OutputFormat::Markdown),
        ConfigResetGuard,
    )
}

pub(super) fn seed_session(context: &ToolContext, session_id: &str) {
    let mut frontmatter: Frontmatter = BTreeMap::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String(format!("Sequential Thinking Session {session_id}")),
    );
    frontmatter.insert(
        "permalink".to_string(),
        YamlValue::String(session_id.to_string()),
    );
    frontmatter.insert(
        "type".to_string(),
        YamlValue::String("sequential".to_string()),
    );
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("active".to_string()),
    );
    frontmatter.insert(
        "phase".to_string(),
        YamlValue::String("running".to_string()),
    );
    frontmatter.insert(
        "activeSubmachineType".to_string(),
        YamlValue::String(String::new()),
    );
    frontmatter.insert(
        "activeSubmachineSessionId".to_string(),
        YamlValue::String(String::new()),
    );
    frontmatter.insert(
        "activeSubmachineTarget".to_string(),
        YamlValue::String(String::new()),
    );
    write_markdown(
        &sequential_session_uri(session_id),
        Some(&frontmatter),
        "# Sequential Thinking Session\n\n",
        &context.config,
    )
    .expect("seed session");
}
