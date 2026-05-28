use super::{
    ConsoleIpcAction, ConsoleIpcError, ConsoleProtocolError, WINDOW_MAENIFOLD_INIT_SCRIPT,
    dispatch_console_invoke, parse_console_ipc, resolve_console_asset,
};
use crate::acp_config::IntentResolverSettings;
use crate::mcp::gateway_runtime::GatewayState;
use crate::mcp::stdio_pool::StdioPool;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;

fn write_fixture(root: &Path, relative_path: &str, contents: &[u8]) {
    let path = root.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create fixture parent");
    }
    fs::write(path, contents).expect("write fixture");
}

#[test]
fn resolve_console_asset_returns_index_html() {
    let root = TempDir::new().expect("create temp dir");
    let expected = b"<html>console</html>";
    write_fixture(root.path(), "index.html", expected);

    let (bytes, mime_type) =
        resolve_console_asset(root.path(), "index.html").expect("resolve index.html");

    assert_eq!(bytes, expected);
    assert_eq!(mime_type, "text/html");
}

#[test]
fn resolve_console_asset_returns_js_asset() {
    let root = TempDir::new().expect("create temp dir");
    let expected = b"console.log('ready');";
    write_fixture(root.path(), "assets/index-abc.js", expected);

    let (bytes, mime_type) = resolve_console_asset(root.path(), "assets/index-abc.js")
        .expect("resolve javascript asset");

    assert_eq!(bytes, expected);
    assert_eq!(mime_type, "application/javascript");
}

#[test]
fn resolve_console_asset_rejects_parent_dir_traversal() {
    let root = TempDir::new().expect("create temp dir");

    let error =
        resolve_console_asset(root.path(), "../Cargo.toml").expect_err("reject parent traversal");

    assert!(matches!(error, ConsoleProtocolError::PathTraversal));
}

#[test]
fn resolve_console_asset_rejects_embedded_parent_traversal() {
    let root = TempDir::new().expect("create temp dir");

    let error = resolve_console_asset(root.path(), "assets/../../etc/passwd")
        .expect_err("reject embedded traversal");

    assert!(matches!(error, ConsoleProtocolError::PathTraversal));
}

#[test]
fn resolve_console_asset_returns_not_found_for_missing_file() {
    let root = TempDir::new().expect("create temp dir");

    let error =
        resolve_console_asset(root.path(), "missing.html").expect_err("missing asset should fail");

    assert!(matches!(error, ConsoleProtocolError::NotFound));
}

#[test]
fn mime_type_dispatch_covers_required_extensions() {
    let root = TempDir::new().expect("create temp dir");
    let cases = [
        ("index.html", "text/html"),
        ("assets/app.js", "application/javascript"),
        ("assets/app.css", "text/css"),
        ("assets/font.woff2", "font/woff2"),
        ("assets/image.png", "image/png"),
        ("assets/logo.svg", "image/svg+xml"),
        ("assets/data.json", "application/json"),
        ("assets/blob.xyz", "application/octet-stream"),
    ];

    for (relative_path, expected_mime) in cases {
        write_fixture(root.path(), relative_path, b"fixture");
        let (_, mime_type) = resolve_console_asset(root.path(), relative_path)
            .expect("fixture asset should resolve");
        assert_eq!(mime_type, expected_mime, "wrong mime for {relative_path}");
    }
}

#[test]
fn window_maenifold_init_script_defines_required_surface() {
    assert!(WINDOW_MAENIFOLD_INIT_SCRIPT.contains("window.maenifold = {"));
    assert!(WINDOW_MAENIFOLD_INIT_SCRIPT.contains("window.ipc.postMessage"));
    assert!(WINDOW_MAENIFOLD_INIT_SCRIPT.contains("invoke(tool, args)"));
    assert!(WINDOW_MAENIFOLD_INIT_SCRIPT.contains("subscribe(channel, cb)"));
    assert!(WINDOW_MAENIFOLD_INIT_SCRIPT.contains("window.__maenifoldDispatch"));
}

#[cfg(test)]
mod console_ipc_envelope_tests {
    use super::*;

    async fn console_gateway_state() -> Arc<GatewayState> {
        let pool = Arc::new(
            StdioPool::start(
                Vec::<crate::mcp::stdio_pool::StdioServerConfig>::new(),
                Arc::new(crate::mcp::PendingRequestRegistry::new()),
            )
            .await
            .expect("empty pool"),
        );
        Arc::new(
            GatewayState::degraded(
                pool,
                IntentResolverSettings::default(),
                PathBuf::from("."),
                "test startup error".to_string(),
            )
            .expect("gateway"),
        )
    }

    #[test]
    fn parse_console_ipc_returns_invoke_action_for_valid_envelope() {
        let message = r#"{"type":"invoke","requestId":"r1","tool":"GatewayExecute","args":{"domain":"maenifold.gate","command":"gate_Status","parameters":{}}}"#;

        let action = parse_console_ipc(message).expect("invoke envelope should parse");

        assert_eq!(
            action,
            ConsoleIpcAction::Invoke {
                request_id: "r1".to_string(),
                tool: "GatewayExecute".to_string(),
                args: json!({
                    "domain": "maenifold.gate",
                    "command": "gate_Status",
                    "parameters": {}
                }),
            }
        );
    }

    #[test]
    fn parse_console_ipc_returns_subscription_stub_for_subscribe_envelope() {
        let message = r#"{"type":"subscribe","requestId":"r2","payload":{"channel":"X"}}"#;

        let action = parse_console_ipc(message).expect("subscribe envelope should parse");

        assert_eq!(action, ConsoleIpcAction::SubscriptionStub);
    }

    #[test]
    fn parse_console_ipc_returns_subscription_stub_for_unsubscribe_envelope() {
        let message = r#"{"type":"unsubscribe","requestId":"r3","payload":{"channel":"X"}}"#;

        let action = parse_console_ipc(message).expect("unsubscribe envelope should parse");

        assert_eq!(action, ConsoleIpcAction::SubscriptionStub);
    }

    #[test]
    fn parse_console_ipc_rejects_invalid_json() {
        let error = parse_console_ipc("not json").expect_err("invalid json should fail");

        assert!(matches!(error, ConsoleIpcError::ParseError(_)));
    }

    #[test]
    fn parse_console_ipc_rejects_unknown_type() {
        let error = parse_console_ipc(r#"{"type":"ping","requestId":"r4"}"#)
            .expect_err("unknown type should fail");

        match error {
            ConsoleIpcError::UnknownType(message_type) => assert_eq!(message_type, "ping"),
            other => panic!("expected UnknownType error, got {other:?}"),
        }
    }

    #[test]
    fn parse_console_ipc_accepts_invoke_with_empty_args() {
        let message = r#"{"type":"invoke","requestId":"r5","tool":"GatewayExecute","args":{}}"#;

        let action =
            parse_console_ipc(message).expect("invoke envelope with empty args should parse");

        assert_eq!(
            action,
            ConsoleIpcAction::Invoke {
                request_id: "r5".to_string(),
                tool: "GatewayExecute".to_string(),
                args: json!({}),
            }
        );
    }

    #[tokio::test]
    async fn dispatch_console_invoke_rejects_unknown_tool() {
        let gateway = console_gateway_state().await;

        let script = dispatch_console_invoke(
            "r6".to_string(),
            "NotARealTool".to_string(),
            json!({}),
            gateway,
        )
        .await;

        assert!(script.contains("unsupported console IPC tool"), "{script}");
        assert!(script.contains("NotARealTool"), "{script}");
        assert!(script.contains(r#""requestId":"r6""#), "{script}");
    }

    #[tokio::test]
    async fn dispatch_console_invoke_returns_error_script_for_unknown_domain() {
        let gateway = console_gateway_state().await;

        let script = dispatch_console_invoke(
            "r7".to_string(),
            "GatewayExecute".to_string(),
            json!({
                "domain": "bogus.domain",
                "command": "x",
                "parameters": {}
            }),
            gateway,
        )
        .await;

        assert!(script.contains(r#""requestId":"r7""#), "{script}");
        assert!(
            script.contains("\"error\"") || script.contains("\"isError\""),
            "{script}"
        );
    }

    #[test]
    fn window_init_script_uses_two_arg_invoke() {
        assert!(WINDOW_MAENIFOLD_INIT_SCRIPT.contains("invoke(tool, args)"));
        assert!(
            WINDOW_MAENIFOLD_INIT_SCRIPT.contains(r#"{ type: "invoke", requestId, tool, args }"#)
        );
    }
}
