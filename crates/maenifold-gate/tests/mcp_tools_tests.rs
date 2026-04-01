#![cfg(feature = "app")]

pub use maenifold_gate::{db, pass, registry, reject};

mod tools_tests {
    include!("../src/mcp/tools.rs");

    use std::{fs, path::PathBuf};

    use maenifold_gate::db::{GateDb, TaskUpsert};
    use rmcp::transport::streamable_http_server::session::{
        SessionId, SessionManager, never::NeverSessionManager,
    };
    use tempfile::TempDir;

    struct TaskEnv {
        _dir: TempDir,
        root: PathBuf,
        db: PathBuf,
    }

    fn setup_task_env(status: &str, pipeline: &str) -> TaskEnv {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        fs::create_dir_all(root.join("todo")).unwrap();
        fs::create_dir_all(root.join("done")).unwrap();
        fs::write(
            root.join("todo").join("T-55.5-mcp.md"),
            format!("---\nstatus: {status}\npipeline: {pipeline}\n---\n\n# Task\n"),
        )
        .unwrap();

        let db = root.join("gate.db");
        GateDb::open(&db)
            .unwrap()
            .upsert_task(&TaskUpsert {
                id: "T-55.5",
                file: "T-55.5-mcp.md",
                title: Some("Task"),
                requirement: "FR-42",
                status,
                location: "todo",
                attempts: 0,
            })
            .unwrap();

        TaskEnv {
            _dir: dir,
            root,
            db,
        }
    }

    fn arg_map<const N: usize>(entries: [(&str, Value); N]) -> Map<String, Value> {
        entries
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect()
    }

    #[test]
    fn gate_list_gates_returns_registry_json_with_expected_fields() {
        let mut args = Map::new();
        let result = dispatch("gate_list_gates", &mut args).unwrap();
        let value: Value = serde_json::from_str(&result.output).unwrap();
        let gates = value
            .get("gates")
            .and_then(Value::as_array)
            .expect("registry JSON should expose a gates array");

        for gate in gates {
            let gate = gate.as_object().expect("gate entries must be objects");
            for field in ["root", "name", "registered_at"] {
                assert!(
                    gate.get(field).and_then(Value::as_str).is_some(),
                    "missing string field {field} in {gate:?}"
                );
            }
        }

        assert!(args.is_empty());
    }

    #[test]
    fn tool_definitions_include_required_schema_fields() {
        let tools = build_tool_list();
        assert!(!tools.is_empty());

        for tool in tools {
            let value = serde_json::to_value(&tool).unwrap();
            assert!(value.get("name").and_then(Value::as_str).is_some());
            assert!(value.get("description").and_then(Value::as_str).is_some());

            let schema = value
                .get("inputSchema")
                .and_then(Value::as_object)
                .expect("tool should serialize inputSchema");
            assert_eq!(schema.get("type").and_then(Value::as_str), Some("object"));
            let properties = schema
                .get("properties")
                .and_then(Value::as_object)
                .expect("tool schema should expose properties");
            if matches!(
                value.get("name").and_then(Value::as_str),
                Some("gate_pass" | "gate_reject" | "gate_status" | "gate_sync")
            ) {
                assert!(
                    !properties.contains_key("pipeline"),
                    "pipeline should not be exposed in {:?}",
                    value.get("name")
                );
            }
        }
    }

    #[test]
    fn required_string_accepts_present_string_values() {
        let mut args = arg_map([("id", json!("T-42"))]);
        assert_eq!(required_string(&mut args, "id").unwrap(), "T-42");
        assert!(args.is_empty());
    }

    #[test]
    fn required_string_rejects_missing_values() {
        let mut args = Map::new();
        let error = required_string(&mut args, "id").unwrap_err().to_string();
        assert!(
            error.contains("missing required argument 'id'"),
            "got: {error}"
        );
    }

    #[test]
    fn required_string_rejects_wrong_types() {
        let mut args = arg_map([("id", json!(7))]);
        let error = required_string(&mut args, "id").unwrap_err().to_string();
        assert!(
            error.contains("argument 'id' must be a string"),
            "got: {error}"
        );
    }

    #[test]
    fn take_string_handles_optional_presence_and_absence() {
        let mut present = arg_map([("notes", json!("hello"))]);
        assert_eq!(
            take_string(&mut present, "notes").unwrap(),
            Some("hello".into())
        );
        assert!(present.is_empty());

        let mut absent = Map::new();
        assert_eq!(take_string(&mut absent, "notes").unwrap(), None);
    }

    #[test]
    fn take_bool_handles_true_false_and_absent_values() {
        let mut truthy = arg_map([("rebuild", json!(true))]);
        assert_eq!(take_bool(&mut truthy, "rebuild").unwrap(), Some(true));
        assert!(truthy.is_empty());

        let mut falsy = arg_map([("rebuild", json!(false))]);
        assert_eq!(take_bool(&mut falsy, "rebuild").unwrap(), Some(false));
        assert!(falsy.is_empty());

        let mut absent = Map::new();
        assert_eq!(take_bool(&mut absent, "rebuild").unwrap(), None);
    }

    #[test]
    fn unknown_tool_name_returns_error() {
        let mut args = Map::new();
        let error = dispatch("gate_unknown", &mut args).unwrap_err();
        assert!(error.contains("unknown tool: gate_unknown"), "got: {error}");
    }

    #[tokio::test]
    async fn never_session_manager_rejects_session_lifecycle_calls() {
        let manager = NeverSessionManager::default();
        let session_id: SessionId = "session-1".into();

        assert!(manager.create_session().await.is_err());
        assert!(manager.has_session(&session_id).await.is_err());
        assert!(manager.close_session(&session_id).await.is_err());
    }

    #[test]
    fn tool_argument_validation_rejects_unknown_args() {
        let mut args = arg_map([("extra", json!(true))]);
        let error = dispatch("gate_list_gates", &mut args).unwrap_err();
        assert!(error.contains("unknown arguments: extra"), "got: {error}");
    }

    #[test]
    fn gate_reject_dispatch_uses_pipeline_validated_reject_logic() {
        let env = setup_task_env("swe", "sdd");
        let mut args = arg_map([
            ("id", json!("T-55.5")),
            ("db", json!(env.db)),
            ("root", json!(env.root)),
        ]);

        let error = dispatch("gate_reject", &mut args).unwrap_err();

        assert!(
            error.contains("pipeline 'sdd' stage 'swe' has no reject transition"),
            "got: {error}"
        );
    }

    #[test]
    fn gate_reject_dispatch_updates_task_via_shared_reject_path() {
        let env = setup_task_env("red", "sdd");
        let mut args = arg_map([
            ("id", json!("T-55.5")),
            ("db", json!(env.db.clone())),
            ("root", json!(env.root.clone())),
            ("notes", json!("retry")),
        ]);

        let result = dispatch("gate_reject", &mut args).unwrap();
        let task = GateDb::open(&env.db)
            .unwrap()
            .get_task_by_id("T-55.5")
            .unwrap()
            .unwrap();

        assert_eq!(result.output, "T-55.5: red → pending [REJECT]");
        assert_eq!(task.status, "pending");
        assert_eq!(task.attempts, 1);
        assert!(
            fs::read_to_string(env.root.join("todo").join("T-55.5-mcp.md"))
                .unwrap()
                .contains("## Orchestrator — REJECT\n\nretry")
        );
    }

    #[test]
    fn mcp_gate_pass_dispatch_uses_pipeline_validated_pass_logic() {
        let env = setup_task_env("invalid-stage", "sdd");
        let mut args = arg_map([
            ("id", json!("T-55.5")),
            ("db", json!(env.db)),
            ("root", json!(env.root)),
        ]);

        let error = dispatch("gate_pass", &mut args).unwrap_err();

        assert!(
            error.contains("task status 'invalid-stage' is not a valid pipeline stage"),
            "got: {error}"
        );
    }

    #[test]
    fn mcp_gate_pass_dispatch_updates_task_via_shared_pass_path() {
        let env = setup_task_env("pending", "sdd");
        let mut args = arg_map([
            ("id", json!("T-55.5")),
            ("db", json!(env.db.clone())),
            ("root", json!(env.root.clone())),
            ("notes", json!("to swe")),
        ]);

        let result = dispatch("gate_pass", &mut args).unwrap();
        let task = GateDb::open(&env.db)
            .unwrap()
            .get_task_by_id("T-55.5")
            .unwrap()
            .unwrap();

        assert_eq!(result.output, "T-55.5: pending → swe [PASS]");
        assert_eq!(task.status, "swe");
        assert!(
            fs::read_to_string(env.root.join("todo").join("T-55.5-mcp.md"))
                .unwrap()
                .contains("## Orchestrator — PASS\n\nto swe")
        );
    }
}
