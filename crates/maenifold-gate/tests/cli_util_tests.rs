use std::path::PathBuf;

use maenifold_gate::{CliError, Command, CommandStubError, HelpTopic, parse_from};

fn parse_error(args: impl IntoIterator<Item = &'static str>) -> CliError {
    match parse_from(args) {
        Ok(_) => panic!("expected parse error"),
        Err(error) => error,
    }
}

#[test]
fn contains_help_detects_long_flag() {
    let cli = parse_from(["gate", "pass", "--help"]).unwrap();
    match cli.command {
        Some(Command::Help(HelpTopic::Pass)) => {}
        other => panic!(
            "expected pass help, got unexpected command: {}",
            other.is_some()
        ),
    }
}

#[test]
fn contains_help_detects_short_flag() {
    let cli = parse_from(["gate", "reject", "-h"]).unwrap();
    match cli.command {
        Some(Command::Help(HelpTopic::Reject)) => {}
        other => panic!(
            "expected reject help, got unexpected command: {}",
            other.is_some()
        ),
    }
}

#[test]
fn contains_help_handles_no_args() {
    let cli = parse_from(["gate"]).unwrap();
    assert!(cli.command.is_none());
}

#[test]
fn contains_help_works_with_mixed_args() {
    let cli = parse_from([
        "gate", "append", "--notes", "hi", "--agent", "bot", "--help",
    ])
    .unwrap();
    match cli.command {
        Some(Command::Help(HelpTopic::Append)) => {}
        _ => panic!("expected append help"),
    }
}

#[test]
fn contains_help_false_when_help_absent() {
    let cli = parse_from(["gate", "sync", "."]).unwrap();
    match cli.command {
        Some(Command::Sync(args)) => assert_eq!(args.root, PathBuf::from(".")),
        _ => panic!("expected sync command"),
    }
}

#[test]
fn next_value_returns_present_value() {
    let cli = parse_from(["gate", "pass", "--db", "custom.db", "T-1"]).unwrap();
    match cli.command {
        Some(Command::Pass(args)) => assert_eq!(args.db_path, PathBuf::from("custom.db")),
        _ => panic!("expected pass command"),
    }
}

#[test]
fn next_value_errors_when_value_missing_at_end() {
    let error = parse_error(["gate", "pass", "T-1", "--db"]);
    let message = error.to_string();
    assert!(message.contains("missing value"), "got: {message}");
    assert!(message.contains("--db"), "got: {message}");
}

#[test]
fn next_value_errors_for_empty_iterator() {
    let error = parse_error(["gate", "status", "--root"]);
    let message = error.to_string();
    assert!(message.contains("missing value"), "got: {message}");
    assert!(message.contains("--root"), "got: {message}");
}

#[test]
fn only_positional_accepts_single_value() {
    let cli = parse_from(["gate", "pass", "--root", ".", "T-42"]).unwrap();
    match cli.command {
        Some(Command::Pass(args)) => assert_eq!(args.id, "T-42"),
        _ => panic!("expected pass command"),
    }
}

#[test]
fn only_positional_rejects_extra_values() {
    let error = parse_error(["gate", "pass", "--root", ".", "T-1", "T-2"]);
    let message = error.to_string();
    assert!(
        message.contains("unexpected extra positional arguments"),
        "got: {message}"
    );
    assert!(
        message.contains("<ID>") || message.contains("ID"),
        "got: {message}"
    );
}

#[test]
fn only_positional_rejects_empty_input() {
    let error = parse_error(["gate", "pass", "--root", "."]);
    let message = error.to_string();
    assert!(
        message.contains("missing required argument"),
        "got: {message}"
    );
    assert!(message.contains("<ID>"), "got: {message}");
}

#[test]
fn only_positional_rejects_flags_instead_of_value() {
    let error = parse_error(["gate", "pass", "--unknown", "T-1"]);
    let message = error.to_string();
    assert!(message.contains("unknown option"), "got: {message}");
    assert!(message.contains("--unknown"), "got: {message}");
}

#[test]
fn set_once_string_sets_value_first_time() {
    let cli = parse_from([
        "gate", "append", "--root", ".", "--notes", "hello", "--agent", "bot", "T-1",
    ])
    .unwrap();
    match cli.command {
        Some(Command::Append(args)) => {
            assert_eq!(args.notes, "hello");
            assert_eq!(args.agent, "bot");
        }
        _ => panic!("expected append command"),
    }
}

#[test]
fn set_once_string_rejects_duplicate_notes() {
    let error = parse_error([
        "gate", "append", "--root", ".", "--notes", "a", "--notes", "b", "--agent", "bot", "T-1",
    ]);
    let message = error.to_string();
    assert!(message.contains("duplicate option"), "got: {message}");
    assert!(message.contains("--notes"), "got: {message}");
}

#[test]
fn set_once_path_sets_value_first_time() {
    let cli = parse_from(["gate", "pass", "--root", "/tmp/project", "T-1"]).unwrap();
    match cli.command {
        Some(Command::Pass(args)) => assert_eq!(args.root, PathBuf::from("/tmp/project")),
        _ => panic!("expected pass command"),
    }
}

#[test]
fn set_once_path_rejects_duplicate_root() {
    let error = parse_error(["gate", "pass", "--root", "/a", "--root", "/b", "T-1"]);
    let message = error.to_string();
    assert!(message.contains("duplicate option"), "got: {message}");
    assert!(message.contains("--root"), "got: {message}");
}

#[test]
fn cli_error_display_returns_original_message() {
    let error = CliError::new("test error");
    assert_eq!(error.to_string(), "test error");
}

#[test]
fn cli_error_display_from_parser_keeps_context() {
    let error = parse_error(["gate", "unknown-command"]);
    let message = error.to_string();
    assert!(message.contains("unknown command"), "got: {message}");
    assert!(message.contains("unknown-command"), "got: {message}");
}

#[test]
fn command_stub_error_display_returns_original_message() {
    let error = CommandStubError::new("stubbed command failed");
    assert_eq!(error.to_string(), "stubbed command failed");
}
