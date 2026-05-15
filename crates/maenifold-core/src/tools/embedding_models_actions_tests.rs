use super::*;
use crate::config::Config;
use crate::db::{GraphDatabase, open_connection};
use crate::mcp::schema::{EmbeddingModelsAction, EmbeddingModelsParams};
use crate::util::json::OutputFormat;

struct ResetConfigOverrides;

impl Drop for ResetConfigOverrides {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

#[test]
fn activate_clears_vec0_tables_after_loading_extension() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let _reset = ResetConfigOverrides;
    Config::override_root(temp.path());
    Config::set_database_path(temp.path().join("memory.db"));

    let config = Config::current();
    config.ensure_directories().expect("ensure directories");
    std::fs::write(config.maenifold_root.join("maenifold.json"), "{}").expect("config fixture");
    install_fixture_vec0(&config);
    let model_path = config
        .assets_path
        .join("models")
        .join("all-MiniLM-L6-v2")
        .join("model.onnx");
    std::fs::create_dir_all(model_path.parent().expect("model parent")).expect("model dir");
    std::fs::write(&model_path, b"fixture model").expect("model fixture");
    GraphDatabase::initialize_database(&config).expect("initialize database");

    let context = ToolContext::new(config.clone(), OutputFormat::Json);
    let params = EmbeddingModelsParams {
        action: EmbeddingModelsAction::Activate,
        model_id: Some("all-MiniLM-L6-v2".to_string()),
        trust_external_catalog: false,
        allow_restricted_license: false,
        learn: false,
    };

    let response = handle_activate(&context, &params).expect("activate model");

    assert_eq!(response["vectorsCleared"], true);
    assert_eq!(response["modelChanged"], true);
    assert_eq!(response["reembedRequired"], true);

    let conn = open_connection(&config.database_path).expect("open database");
    let remaining_tables: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master \
             WHERE type = 'table' AND name IN ('vec_concepts', 'vec_memory_files')",
            [],
            |row| row.get(0),
        )
        .expect("query vector table count");
    assert_eq!(remaining_tables, 0);
}

fn install_fixture_vec0(config: &Config) {
    let (platform_dir, filename) = vec0_platform_target();
    let source = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
        .join("native")
        .join(platform_dir)
        .join(filename);
    let target_dir = config.assets_path.join("native").join(platform_dir);
    std::fs::create_dir_all(&target_dir).expect("native dir");
    std::fs::copy(&source, target_dir.join(filename)).expect("vec0 fixture");
}

fn vec0_platform_target() -> (&'static str, &'static str) {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => ("osx-arm64", "vec0.dylib"),
        ("macos", "x86_64") => ("osx-x64", "vec0.dylib"),
        ("linux", "x86_64") => ("linux-x64", "vec0.so"),
        ("linux", "aarch64") => ("linux-arm64", "vec0.so"),
        ("windows", "x86_64") => ("win-x64", "vec0.dll"),
        other => panic!("unsupported vec0 test platform: {other:?}"),
    }
}
