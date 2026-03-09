use maenifold_core::json::OutputFormat;
use maenifold_core::{AssetCatalog, AssetManager, Config, MaenifoldError, ToolRegistry};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::{Value, json};
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

struct ResetOverrides;

impl Drop for ResetOverrides {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

fn isolated_initialized_config() -> (Config, TempDir, ResetOverrides) {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let config = Config::current();
    AssetManager::initialize_assets(&config).expect("assets");
    (config, tmp, ResetOverrides)
}

fn invoke_adopt(payload: Value) -> String {
    ToolRegistry::global()
        .try_invoke("Adopt", &payload, OutputFormat::Markdown)
        .expect("invoke Adopt")
}

fn invoke_adopt_err(payload: Value) -> MaenifoldError {
    ToolRegistry::global()
        .try_invoke("Adopt", &payload, OutputFormat::Markdown)
        .expect_err("Adopt should fail")
}

fn catalog(config: &Config) -> AssetCatalog {
    AssetManager::new(config).catalog().expect("asset catalog")
}

fn asset_id(entry: &Value) -> String {
    entry
        .get("id")
        .and_then(Value::as_str)
        .expect("catalog entry id")
        .to_string()
}

// T-1.4: RTM FR-1 - Ports missing C# coverage for case-insensitive Adopt type parsing.
#[test]
fn adopt_type_is_case_insensitive() {
    let _guard = TEST_MUTEX.lock();
    let (config, _tmp, _reset) = isolated_initialized_config();
    let role_id = asset_id(&catalog(&config).roles[0]);

    let lower = invoke_adopt(json!({"type":"role","identifier": &role_id}));
    let upper = invoke_adopt(json!({"type":"ROLE","identifier": &role_id}));
    let mixed = invoke_adopt(json!({"type":"Role","identifier": &role_id}));

    assert_eq!(
        lower, upper,
        "uppercase type should normalize to same output"
    );
    assert_eq!(
        lower, mixed,
        "mixed-case type should normalize to same output"
    );
    assert!(
        lower.contains("adopting the role"),
        "expected role adoption instructions: {lower}"
    );
}

// T-1.4: RTM FR-1 - Ports missing C# coverage for unknown Adopt identifiers.
#[test]
fn adopt_with_non_existent_identifier_returns_asset_not_found_error() {
    let _guard = TEST_MUTEX.lock();
    let (_config, _tmp, _reset) = isolated_initialized_config();

    let err = invoke_adopt_err(json!({
        "type":"role",
        "identifier":"non-existent-asset-xyz"
    }));

    let message = err.to_string();
    match &err {
        MaenifoldError::AssetNotFound(kind, identifier) => {
            assert_eq!(kind, "role");
            assert_eq!(identifier, "non-existent-asset-xyz");
        }
        other => panic!("expected AssetNotFound error, got {other}"),
    }
    assert!(
        message.contains("Asset not found"),
        "expected asset lookup failure message: {message}"
    );
    assert!(
        message.contains("role/non-existent-asset-xyz"),
        "expected kind/identifier in error message: {message}"
    );
}

// T-1.4: RTM FR-1 - Ports missing C# coverage for empty Adopt identifiers.
#[test]
fn adopt_with_empty_identifier_returns_invalid_argument_error() {
    let _guard = TEST_MUTEX.lock();
    let (_config, _tmp, _reset) = isolated_initialized_config();

    let err = invoke_adopt_err(json!({"type":"role","identifier":""}));
    let message = err.to_string();

    match err {
        MaenifoldError::InvalidArgument(detail) => {
            assert!(
                detail.contains("must not be empty"),
                "expected empty identifier detail: {detail}"
            );
        }
        other => panic!("expected InvalidArgument error, got {other}"),
    }

    assert!(
        message.contains("Identifier must not be empty"),
        "expected empty identifier message: {message}"
    );
}

// T-1.4: RTM FR-1 - Ports missing C# coverage for critical Adopt instruction text.
#[test]
fn adopt_includes_critical_instructions() {
    let _guard = TEST_MUTEX.lock();
    let (config, _tmp, _reset) = isolated_initialized_config();
    let color_id = asset_id(&catalog(&config).colors[0]);

    let output = invoke_adopt(json!({"type":"color","identifier": &color_id}));

    assert!(
        output.contains("CRITICAL INSTRUCTIONS"),
        "expected critical instructions header: {output}"
    );
    assert!(
        output.contains("parse the JSON"),
        "expected JSON parsing instruction: {output}"
    );
    assert!(
        output.contains("Incorporate ALL attributes"),
        "expected attribute incorporation instruction: {output}"
    );
    assert!(
        output.contains("Acknowledge adoption"),
        "expected acknowledgment instruction: {output}"
    );
}

// T-1.4: RTM FR-1 - Ports missing C# coverage for distinguishing bundled assets of the same type.
#[test]
fn adopt_distinguishes_between_different_assets_of_same_type() {
    let _guard = TEST_MUTEX.lock();
    let (config, _tmp, _reset) = isolated_initialized_config();
    let catalog = catalog(&config);
    assert!(
        catalog.roles.len() >= 2,
        "expected at least two bundled roles for distinction test"
    );

    let first_role_id = asset_id(&catalog.roles[0]);
    let second_role_id = asset_id(&catalog.roles[1]);

    let first_output = invoke_adopt(json!({"type":"role","identifier": &first_role_id}));
    let second_output = invoke_adopt(json!({"type":"role","identifier": &second_role_id}));

    assert!(
        first_output.contains(&first_role_id),
        "expected first output to reference {first_role_id}: {first_output}"
    );
    assert!(
        second_output.contains(&second_role_id),
        "expected second output to reference {second_role_id}: {second_output}"
    );
    assert_ne!(
        first_output, second_output,
        "different role assets should produce different adoption instructions"
    );
}
