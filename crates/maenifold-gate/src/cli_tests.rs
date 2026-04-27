use super::{CliAction, CommandRequest, GateCli, GateCliApp, parse_cli, validate_cli};
use clap::Parser;

#[test]
fn classify_mcp_mode_as_tray_enabled() {
    let args = vec!["--mcp".to_string()];
    assert_eq!(GateCliApp::classify(&args), CliAction::Mcp { root: None });
}

#[test]
fn classify_http_mode_as_tray_enabled() {
    let args = vec![
        "--http".to_string(),
        "--port".to_string(),
        "8181".to_string(),
    ];
    assert_eq!(
        GateCliApp::classify(&args),
        CliAction::Http {
            port: 8181,
            root: None,
        }
    );
}

#[test]
fn classify_service_mode_with_root() {
    let args = vec![
        "--http".to_string(),
        "--root".to_string(),
        "/workspace/project".to_string(),
    ];
    assert_eq!(
        GateCliApp::classify(&args),
        CliAction::Http {
            port: 8081,
            root: Some("/workspace/project".into()),
        }
    );
}

#[test]
fn reject_port_without_http() {
    let args = vec!["--port".to_string(), "8181".to_string()];
    let parsed = parse_cli(&args).expect("CLI should parse");
    let error = validate_cli(&parsed).expect_err("port without --http must fail");
    assert!(
        error
            .to_string()
            .contains("--port may only be used with --http")
    );
}

#[test]
fn reject_top_level_root_without_service_mode() {
    let args = vec!["--root".to_string(), "/workspace/project".to_string()];
    let parsed = parse_cli(&args).expect("CLI should parse");
    let error = validate_cli(&parsed).expect_err("top-level root without service mode must fail");
    assert!(
        error
            .to_string()
            .contains("top-level --root may only be used with --mcp or --http")
    );
}

#[test]
fn subcommand_root_remains_valid_without_service_mode() {
    let args = vec![
        "status".to_string(),
        "--root".to_string(),
        "/workspace/project".to_string(),
    ];
    let parsed = parse_cli(&args).expect("CLI should parse");
    validate_cli(&parsed).expect("subcommand root should remain valid");
}

#[test]
fn parse_status_ready_flag() {
    let cli = GateCli::parse_from(["gate", "status", "--ready"]);
    let Some(command) = cli.command else {
        panic!("status subcommand should be present");
    };
    let request = command.into_request().expect("status request should build");
    let CommandRequest::Status(args) = request else {
        panic!("status subcommand should produce a status request");
    };
    assert!(args.filter.ready_only);
    assert!(!args.filter.blocked_only);
    assert_eq!(args.filter.state, None);
    assert!(!args.filter.include_done);
}

#[test]
fn parse_status_state_flag() {
    let cli = GateCli::parse_from(["gate", "status", "--state", "swe"]);
    let Some(command) = cli.command else {
        panic!("status subcommand should be present");
    };
    let request = command.into_request().expect("status request should build");
    let CommandRequest::Status(args) = request else {
        panic!("status subcommand should produce a status request");
    };
    assert_eq!(args.filter.state.as_deref(), Some("swe"));
    assert!(!args.filter.ready_only);
    assert!(!args.filter.blocked_only);
    assert!(!args.filter.include_done);
}

#[test]
fn parse_status_all_flag() {
    let cli = GateCli::parse_from(["gate", "status", "--all"]);
    let Some(command) = cli.command else {
        panic!("status subcommand should be present");
    };
    let request = command.into_request().expect("status request should build");
    let CommandRequest::Status(args) = request else {
        panic!("status subcommand should produce a status request");
    };
    assert!(args.filter.include_done);
    assert!(!args.filter.ready_only);
    assert!(!args.filter.blocked_only);
    assert_eq!(args.filter.state, None);
}

#[test]
fn parse_status_blocked_flag() {
    let cli = GateCli::parse_from(["gate", "status", "--blocked"]);
    let Some(command) = cli.command else {
        panic!("status subcommand should be present");
    };
    let request = command.into_request().expect("status request should build");
    let CommandRequest::Status(args) = request else {
        panic!("status subcommand should produce a status request");
    };
    assert!(args.filter.blocked_only);
    assert!(!args.filter.ready_only);
}
