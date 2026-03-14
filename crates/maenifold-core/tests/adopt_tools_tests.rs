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

// T-1.4: RTM FR-1, NFR-2 - Tests empty and whitespace-only identifiers against
// the `trim().is_empty()` code path in `validate_adopt_identifier`.
#[test]
fn adopt_with_empty_or_whitespace_identifier_returns_invalid_argument_error() {
    let _guard = TEST_MUTEX.lock();
    let (_config, _tmp, _reset) = isolated_initialized_config();

    // Each case exercises the same trim().is_empty() branch documented in the error
    // message: "Identifier must not be empty or whitespace-only."
    let cases: &[(&str, &str)] = &[
        ("", "empty string"),
        ("   ", "whitespace-only spaces"),
        ("\t", "whitespace-only tab"),
    ];

    for (identifier, label) in cases {
        let err = invoke_adopt_err(json!({"type":"role","identifier": identifier}));
        let message = err.to_string();

        match &err {
            MaenifoldError::InvalidArgument(detail) => {
                assert!(
                    detail.contains("must not be empty"),
                    "[{label}] expected empty/whitespace identifier detail: {detail}"
                );
            }
            other => panic!("[{label}] expected InvalidArgument error, got {other}"),
        }

        assert!(
            message.contains("Identifier must not be empty"),
            "[{label}] expected empty/whitespace identifier message: {message}"
        );
    }
}

// T-1.4: RTM FR-1 - Ports missing C# coverage for critical Adopt instruction text.
// Uses a color asset to demonstrate the CRITICAL INSTRUCTIONS template is asset-type-agnostic.
#[test]
fn adopt_includes_critical_instructions() {
    let _guard = TEST_MUTEX.lock();
    let (config, _tmp, _reset) = isolated_initialized_config();
    let cat = catalog(&config);
    assert!(
        !cat.colors.is_empty(),
        "expected at least one bundled color asset"
    );
    let color_id = asset_id(&cat.colors[0]);

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
    // Logically follows from the contains assertions above given distinct ids,
    // but serves as a belt-and-suspenders guard against coincidental embedding.
    assert_ne!(
        first_output, second_output,
        "different role assets should produce different adoption instructions"
    );
}

// T-1.4: RTM FR-1, NFR-2 - Security: path-traversal identifiers must be rejected by
// `validate_adopt_identifier` before any filesystem access occurs.
#[test]
fn adopt_with_path_traversal_identifier_is_rejected() {
    let _guard = TEST_MUTEX.lock();
    let (_config, _tmp, _reset) = isolated_initialized_config();

    let cases: &[(&str, &str)] = &[
        ("../secret", "upward traversal"),
        ("../../etc/passwd", "deep traversal"),
        ("a/b", "embedded slash"),
        ("role\\..\\secret", "backslash traversal"),
    ];

    for (identifier, label) in cases {
        let err = invoke_adopt_err(json!({"type":"role","identifier": identifier}));

        match &err {
            MaenifoldError::InvalidArgument(detail) => {
                assert!(
                    detail.contains("may only contain ASCII letters"),
                    "[{label}] expected slug-validation detail for '{identifier}': {detail}"
                );
            }
            other => panic!("[{label}] expected InvalidArgument for '{identifier}', got {other}"),
        }
    }
}

// T-1.4: RTM FR-1 (RTM-004) - The `parse_adopt_kind` allow-list intentionally excludes
// "workflow" even though `AssetKind::parse()` accepts it. This test pins that contract.
#[test]
fn adopt_workflow_type_is_rejected_with_allowed_types_in_message() {
    let _guard = TEST_MUTEX.lock();
    let (_config, _tmp, _reset) = isolated_initialized_config();

    let err = invoke_adopt_err(json!({"type":"workflow","identifier":"any"}));
    let message = err.to_string();

    match &err {
        MaenifoldError::InvalidArgument(detail) => {
            assert!(
                detail.contains("Must be one of: role, color, perspective"),
                "expected allowed-types list in error: {detail}"
            );
            assert!(
                detail.contains("Invalid type 'workflow'"),
                "expected rejected type name in error: {detail}"
            );
        }
        other => panic!("expected InvalidArgument for workflow type, got {other}"),
    }

    // The allowed-types portion should not list "workflow" as adoptable.
    let allowed_portion = message.split("Must be one of:").nth(1).unwrap_or("");
    assert!(
        !allowed_portion.contains("workflow"),
        "workflow must not appear in the allowed types list: {message}"
    );
}

// T-1.4: RTM FR-1, NFR-2 - Missing required properties yield MissingProperty errors
// with the correct field name.
#[test]
fn adopt_missing_required_properties_returns_missing_property_error() {
    let _guard = TEST_MUTEX.lock();
    let (_config, _tmp, _reset) = isolated_initialized_config();

    // Missing "type" field entirely.
    let err = invoke_adopt_err(json!({"identifier":"some-role"}));
    match &err {
        MaenifoldError::MissingProperty(field) => {
            assert_eq!(*field, "type", "expected MissingProperty('type')");
        }
        other => panic!("expected MissingProperty for missing type, got {other}"),
    }

    // Missing "identifier" field entirely.
    let err = invoke_adopt_err(json!({"type":"role"}));
    match &err {
        MaenifoldError::MissingProperty(field) => {
            assert_eq!(
                *field, "identifier",
                "expected MissingProperty('identifier')"
            );
        }
        other => panic!("expected MissingProperty for missing identifier, got {other}"),
    }

    // Both missing — "type" is parsed first, so it should be the reported field.
    let err = invoke_adopt_err(json!({}));
    match &err {
        MaenifoldError::MissingProperty(field) => {
            assert_eq!(
                *field, "type",
                "expected MissingProperty('type') when both fields missing"
            );
        }
        other => panic!("expected MissingProperty for empty payload, got {other}"),
    }
}
