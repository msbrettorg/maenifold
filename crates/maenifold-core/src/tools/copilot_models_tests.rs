use super::*;
use crate::config::Config;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use serde_json::json;
use std::path::PathBuf;

fn repo_assets_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

#[test]
fn get_copilot_models_learn_returns_help_doc() {
    let mut config = Config::current();
    config.assets_path = repo_assets_path();
    let context = ToolContext::new(config, OutputFormat::Markdown);

    let output =
        get_copilot_models(&context, &json!({ "learn": true })).expect("learn should return help");

    assert!(output.starts_with("# GetCopilotModels"), "{output}");
    assert!(output.contains("- refresh (boolean, optional)"), "{output}");
}
