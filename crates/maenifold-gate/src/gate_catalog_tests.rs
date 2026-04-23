    use std::ffi::OsString;

    use tempfile::TempDir;

    use super::{SDD_JSON, TDD_JSON, GSD_JSON, LIGHTWEIGHT_JSON, MDD_JSON, SLC_JSON, SDD_C_JSON, TDD_C_JSON, GateDefinitionCatalog, builtin_asset_text};
    use crate::gate_definition::GateDefinition;
    use crate::task_metadata::TaskMetadata;

    fn set_maenifold_root(temp: &std::path::Path) -> (Option<OsString>, std::path::PathBuf) {
        let previous = std::env::var_os("MAENIFOLD_ROOT");
        unsafe {
            std::env::set_var("MAENIFOLD_ROOT", temp);
        }
        let gates_dir = temp.join("assets").join("gates");
        std::fs::create_dir_all(&gates_dir).expect("create gates dir");
        (previous, gates_dir)
    }

    fn restore_maenifold_root(previous: Option<OsString>) {
        match previous {
            Some(value) => unsafe {
                std::env::set_var("MAENIFOLD_ROOT", value);
            },
            None => unsafe {
                std::env::remove_var("MAENIFOLD_ROOT");
            },
        }
    }

    #[test]
    fn builtin_assets_refresh_reserved_runtime_copies_when_bundled_content_changes() {
        let _guard = super::maenifold_root_lock().lock().expect("lock env guard");
        let temp = TempDir::new().expect("create temp dir");
        let (previous, gates_dir) = set_maenifold_root(temp.path());

        let sdd_path = gates_dir.join("sdd.json");
        let stale_sdd = r#"{
  "schemaVersion": 1,
  "id": "sdd",
  "name": "Stale Runtime SDD",
  "initial": "pending",
  "states": {
    "pending": { "on": { "pass": "done" } },
    "done": { "terminal": true }
  }
}"#;
        std::fs::write(&sdd_path, stale_sdd).expect("write stale runtime asset");

        let definition = super::load_builtin_inner("sdd").expect("load builtin");
        let preserved = std::fs::read_to_string(&sdd_path).expect("read runtime asset");

        restore_maenifold_root(previous);

        assert_eq!(
            definition.ordered_stage_ids().expect("stage ids"),
            vec!["pending", "swe", "red-team", "blue-team", "done"]
        );
        assert_eq!(preserved, SDD_JSON);
        assert!(gates_dir.join("tdd.json").exists());
        assert!(gates_dir.join("gsd.json").exists());
        assert!(gates_dir.join("lightweight.json").exists());
    }

    #[test]
    fn builtin_bootstrap_does_not_overwrite_non_reserved_runtime_definitions() {
        let _guard = super::maenifold_root_lock().lock().expect("lock env guard");
        let temp = TempDir::new().expect("create temp dir");
        let (previous, gates_dir) = set_maenifold_root(temp.path());

        let custom_path = gates_dir.join("custom.json");
        let custom_definition = r#"{
  "schemaVersion": 1,
  "id": "custom",
  "name": "Custom Runtime Gate",
  "initial": "pending",
  "states": {
    "pending": { "on": { "pass": "done" } },
    "done": { "terminal": true }
  }
}"#;
        std::fs::write(&custom_path, custom_definition).expect("write custom runtime definition");

        super::load_builtin_inner("tdd").expect("load builtin");
        let preserved =
            std::fs::read_to_string(&custom_path).expect("read custom runtime definition");

        restore_maenifold_root(previous);

        assert_eq!(preserved, custom_definition);
    }

    #[test]
    fn loads_custom_definition_by_name_from_gates_dir() {
        let _guard = super::maenifold_root_lock().lock().expect("lock env guard");
        let temp = TempDir::new().expect("create temp dir");
        let (previous, gates_dir) = set_maenifold_root(temp.path());

        let dkd = r#"{
  "schemaVersion": 1,
  "id": "dkd",
  "name": "Dunning Kruger Development",
  "initial": "pending",
  "states": {
    "pending": { "on": { "pass": "swe-plan" } },
    "swe-plan": { "on": { "pass": "done" } },
    "done": { "terminal": true }
  }
}"#;
        std::fs::write(gates_dir.join("dkd.json"), dkd).expect("write dkd");

        let definition = super::load_builtin_inner("dkd").expect("load dkd");

        restore_maenifold_root(previous);

        assert_eq!(
            definition.ordered_stage_ids().expect("stage ids"),
            vec!["pending", "swe-plan", "done"]
        );
    }

    #[test]
    fn gates_dir_uses_maenifold_root() {
        let _guard = super::maenifold_root_lock().lock().expect("lock env guard");
        let temp = TempDir::new().expect("create temp dir");
        let (previous, _gates_dir) = set_maenifold_root(temp.path());

        let dir = GateDefinitionCatalog::gates_dir();

        restore_maenifold_root(previous);

        assert_eq!(dir, temp.path().join("assets").join("gates"));
    }

    #[test]
    fn all_builtin_assets_parse_and_validate() {
        let assets: &[(&str, &str)] = &[
            ("sdd", SDD_JSON),
            ("tdd", TDD_JSON),
            ("gsd", GSD_JSON),
            ("lightweight", LIGHTWEIGHT_JSON),
            ("mdd", MDD_JSON),
            ("slc", SLC_JSON),
            ("sdd-c", SDD_C_JSON),
            ("tdd-c", TDD_C_JSON),
        ];
        for (name, json) in assets {
            let definition = GateDefinition::from_json_text(json)
                .unwrap_or_else(|err| panic!("built-in '{name}' failed to parse: {err}"));
            assert!(
                !definition.initial.is_empty(),
                "built-in '{name}' has empty initial state"
            );
            let stages = definition.ordered_stage_ids()
                .unwrap_or_else(|err| panic!("built-in '{name}' failed ordered_stage_ids: {err}"));
            assert!(
                stages.len() >= 2,
                "built-in '{name}' has fewer than 2 stages: {stages:?}"
            );
        }
    }

    #[test]
    fn builtin_asset_text_rejects_unknown_name() {
        let result = builtin_asset_text("nonexistent");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("nonexistent"), "error should mention the name: {err}");
    }

    #[test]
    fn resolve_for_task_rejects_pipeline_mismatch() {
        let root = tempfile::tempdir().expect("create temp root");
        let metadata = TaskMetadata {
            id: Some("T-1.1".to_string()),
            title: "Test task".to_string(),
            pipeline: "sdd".to_string(),
            state: None,
            session_id: None,
        };
        let result = GateDefinitionCatalog::resolve_for_task(
            root.path(),
            &metadata,
            Some("tdd"),
        );
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("sdd"), "error should mention bound pipeline: {err}");
        assert!(err.contains("tdd"), "error should mention requested pipeline: {err}");
    }

