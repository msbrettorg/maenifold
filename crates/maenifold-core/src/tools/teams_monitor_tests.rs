use super::*;
use crate::config::Config;
use crate::teams::set_teams_disabled;
use crate::test_util::ConfigResetGuard;
use crate::util::json::OutputFormat;
use serde_json::{Value, json};
use serial_test::serial;
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
#[serial]
fn authenticate_action_is_removed_from_public_surface() {
    with_context(|context| {
        let error = teams_monitor(context, &json!({"action":"authenticate"}))
            .expect_err("authenticate removed");
        let message = error.to_string();
        assert!(message.contains("unknown action 'authenticate'"));
        assert!(!message.contains("clientId"));
        assert!(!message.contains("tenantId"));
    });
}

#[test]
#[serial]
fn disable_action_returns_success_with_enabled_false() {
    with_context(|context| assert_eq!(run(context, json!({"action":"disable"}))["enabled"], false));
}

#[test]
#[serial]
fn status_action_returns_current_config_state() {
    with_context(|context| {
        let response = run(context, json!({"action":"status"}));
        assert_eq!(response["action"], "status");
        assert_eq!(
            response["config"],
            json!({
                "identity": null,
                "keyword": "@maenifold",
                "listenScope": "wildcard",
                "monitoredConversations": [],
                "replyBehavior": "sourceConversation",
                "replyPrefix": "maenifold: ",
                "dispatchTimeoutMinutes": 10,
                "dispatchAgent": null,
                "connection": {
                    "trouterGateway": "go-msit.trouter.teams.microsoft.com",
                    "registrarUrl": "https://teams.cloud.microsoft/registrar/prod/V2/registrations",
                    "appId": "TeamsCDLWebWorker",
                    "reconnectDelaySeconds": 5,
                    "maxReconnectDelaySeconds": 300,
                    "tokenRefreshMinutes": 50,
                    "heartbeatIntervalSeconds": 30,
                    "chatsvcRegion": "amer"
                },
            })
        );
    });
}

#[test]
#[serial]
fn configure_with_keyword_updates_status() {
    with_context(|context| {
        run(
            context,
            json!({"action":"configure","keyword":"alpha","listenScope":"self","replyBehavior":"selfChat","replyPrefix":"reply: ","dispatchTimeoutMinutes":3}),
        );
        let config = run(context, json!({"action":"status"}))["config"].clone();
        assert_eq!(config["keyword"], json!("alpha"));
        assert_eq!(config["listenScope"], json!("self"));
        assert_eq!(config["replyBehavior"], json!("selfChat"));
        assert_eq!(config["replyPrefix"], json!("reply: "));
        assert_eq!(config["dispatchTimeoutMinutes"], json!(3));
    });
}

#[test]
#[serial]
fn unknown_action_returns_valid_action_list() {
    with_context(|context| {
        let error = teams_monitor(context, &json!({"action":"bogus"})).expect_err("unknown action");
        assert_eq!(
            error.to_string(),
            "unknown action 'bogus'. Available: enable, disable, status, configure, detect_identity, refresh, sign_out"
        );
    });
}

#[test]
#[serial]
fn detect_identity_does_not_accept_app_registration_fields() {
    with_context(|context| {
        let error = teams_monitor(
            context,
            &json!({"action":"authenticate_start","tenantId":"tenant","clientId":"client"}),
        )
        .expect_err("authenticate_start removed");
        let message = error.to_string();
        assert!(message.contains("unknown action 'authenticate_start'"));
        assert!(!message.contains("authenticate_complete"));
    });
}

#[test]
#[serial]
fn invalid_dispatch_agent_type_returns_type_error() {
    with_context(|context| {
        let error = teams_monitor(context, &json!({"action":"configure","dispatchAgent":true}))
            .expect_err("dispatchAgent type error");
        assert_eq!(error.to_string(), "'dispatchAgent' must be a string");
    });
}

#[test]
#[serial]
fn invalid_dispatch_timeout_minutes_type_returns_type_error() {
    with_context(|context| {
        let error = teams_monitor(
            context,
            &json!({"action":"configure","dispatchTimeoutMinutes":"soon"}),
        )
        .expect_err("dispatchTimeoutMinutes type error");
        assert_eq!(
            error.to_string(),
            "'dispatchTimeoutMinutes' must be an integer"
        );
    });
}
