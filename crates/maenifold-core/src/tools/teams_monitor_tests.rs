use super::*;
use crate::config::Config;
use crate::test_util::ConfigResetGuard;
use crate::teams::set_teams_disabled;
use crate::util::json::OutputFormat;
use serde_json::{Value, json};
use std::sync::{LazyLock, Mutex};
use tempfile::TempDir;

static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

struct TeamsRuntimeResetGuard;

impl Drop for TeamsRuntimeResetGuard {
    fn drop(&mut self) {
        set_teams_disabled();
    }
}

fn with_context(test: impl FnOnce(&ToolContext)) {
    let _lock = TEST_LOCK.lock().expect("test lock");
    let _runtime_guard = TeamsRuntimeResetGuard;
    set_teams_disabled();
    let tmp = TempDir::new().expect("temp dir");
    std::fs::write(
        tmp.path().join("maenifold.json"),
        r#"{"agents":{},"settings":{}}"#,
    )
    .expect("write minimal maenifold config");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let config = Config::current();
    config.ensure_directories().expect("ensure directories");
    let _guard = ConfigResetGuard;
    test(&ToolContext::new(config, OutputFormat::Json));
}

fn run(context: &ToolContext, payload: Value) -> Value {
    serde_json::from_str(&teams_monitor(context, &payload).expect("teams monitor response"))
        .expect("json response")
}

#[test]
fn enable_without_credentials_returns_auth_error() {
    with_context(|context| {
        let result = teams_monitor(context, &json!({"action": "enable"}));
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("authentication required"));
    });
}

#[test]
fn disable_action_returns_success_with_enabled_false() {
    with_context(|context| assert_eq!(run(context, json!({"action":"disable"}))["enabled"], false));
}

#[test]
fn status_action_returns_current_config_state() {
    with_context(|context| {
        let response = run(context, json!({"action":"status"}));
        assert_eq!(response["action"], "status");
        assert_eq!(
            response["config"],
            json!({
                "clientId": null,
                "tenantId": null,
                "keywords": [],
                "senderAllowList": null,
                "senderDenyList": null,
                "cooldownSeconds": 60,
                "dispatchAgent": null,
            })
        );
    });
}

#[test]
fn configure_with_keywords_updates_status() {
    with_context(|context| {
        run(
            context,
            json!({"action":"configure","keywords":["alpha","beta"]}),
        );
        assert_eq!(
            run(context, json!({"action":"status"}))["config"]["keywords"],
            json!(["alpha", "beta"])
        );
    });
}

#[test]
fn unknown_action_returns_valid_action_list() {
    with_context(|context| {
        let error = teams_monitor(context, &json!({"action":"bogus"})).expect_err("unknown action");
        assert_eq!(
            error.to_string(),
            "unknown action 'bogus'. Available: enable, disable, status, configure, authenticate, authenticate_start, authenticate_complete, refresh, sign_out"
        );
    });
}

#[test]
fn authenticate_requires_client_id() {
    with_context(|context| {
        let error = teams_monitor(
            context,
            &json!({"action":"authenticate","tenantId":"tenant"}),
        )
        .expect_err("missing clientId");
        assert_eq!(
            error.to_string(),
            "Required property 'clientId' is missing from payload."
        );
    });
}

#[test]
fn invalid_dispatch_agent_type_returns_type_error() {
    with_context(|context| {
        let error = teams_monitor(context, &json!({"action":"configure","dispatchAgent":true}))
            .expect_err("dispatchAgent type error");
        assert_eq!(error.to_string(), "'dispatchAgent' must be a string");
    });
}

#[test]
fn invalid_cooldown_seconds_type_returns_type_error() {
    with_context(|context| {
        let error = teams_monitor(
            context,
            &json!({"action":"configure","cooldownSeconds":"soon"}),
        )
        .expect_err("cooldownSeconds type error");
        assert_eq!(error.to_string(), "'cooldownSeconds' must be an integer");
    });
}
