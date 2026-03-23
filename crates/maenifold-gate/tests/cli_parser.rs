use std::{env, path::PathBuf};

use maenifold_gate::{Command, HelpTopic, SyncArgs, parse_from};

#[test]
fn pass_uses_current_dir_defaults() {
    let cli = parse_from(["gate", "pass", "T-1.1"]).unwrap();
    match cli.command {
        Some(Command::Pass(args)) => {
            let cwd = env::current_dir().unwrap();
            assert_eq!(args.id, "T-1.1");
            assert_eq!(args.notes, None);
            assert_eq!(args.pipeline_path, None);
            assert_eq!(args.root, cwd);
            assert_eq!(args.db_path, args.root.join("pipeline.db"));
        }
        _ => panic!("expected pass command"),
    }
}

#[test]
fn reject_parses_pipeline_and_notes() {
    let cli = parse_from([
        "gate",
        "reject",
        "--root",
        ".",
        "--pipeline",
        "custom.json",
        "--notes",
        "needs work",
        "T-1.1",
    ])
    .unwrap();
    match cli.command {
        Some(Command::Reject(args)) => {
            assert_eq!(args.id, "T-1.1");
            assert_eq!(args.notes.as_deref(), Some("needs work"));
            assert_eq!(args.pipeline_path, Some(PathBuf::from("custom.json")));
            assert_eq!(args.root, PathBuf::from("."));
            assert_eq!(args.db_path, PathBuf::from(".").join("pipeline.db"));
        }
        _ => panic!("expected reject command"),
    }
}

#[test]
fn pass_parses_embedded_pipeline_name() {
    let cli = parse_from([
        "gate",
        "pass",
        "--root",
        ".",
        "--pipeline",
        "tdd",
        "T-1.1",
    ])
    .unwrap();
    match cli.command {
        Some(Command::Pass(args)) => {
            assert_eq!(args.id, "T-1.1");
            assert_eq!(args.pipeline_path, Some(PathBuf::from("tdd")));
            assert_eq!(args.root, PathBuf::from("."));
            assert_eq!(args.db_path, PathBuf::from(".").join("pipeline.db"));
        }
        _ => panic!("expected pass command"),
    }
}

#[test]
fn append_requires_agent() {
    let error = match parse_from(["gate", "append", "--notes", "hello", "T-1.1"]) {
        Ok(_) => panic!("expected parse error"),
        Err(error) => error,
    };
    assert_eq!(error.to_string(), "missing required option --agent");
}

#[test]
fn status_accepts_optional_id() {
    let cli = parse_from(["gate", "status", "--root", ".", "T-1.1"]).unwrap();
    match cli.command {
        Some(Command::Status(args)) => {
            assert_eq!(args.root, PathBuf::from("."));
            assert_eq!(args.id.as_deref(), Some("T-1.1"));
            assert!(!args.json);
            assert_eq!(args.db, None);
        }
        _ => panic!("expected status command"),
    }
}

#[test]
fn sync_help_is_available() {
    let cli = parse_from(["gate", "sync", "--help"]).unwrap();
    match cli.command {
        Some(Command::Help(topic)) => assert_eq!(topic, HelpTopic::Sync),
        _ => panic!("expected sync help"),
    }
}

#[test]
fn sync_requires_root_positional() {
    let error = match parse_from(["gate", "sync", "--rebuild"]) {
        Ok(_) => panic!("expected parse error"),
        Err(error) => error,
    };
    assert_eq!(error.to_string(), "missing required argument <ROOT>");
}

#[test]
fn sync_parses_root_and_rebuild_flag() {
    let cli = parse_from(["gate", "sync", "--rebuild", "."]).unwrap();
    match cli.command {
        Some(Command::Sync(args)) => {
            assert_eq!(
                args,
                SyncArgs {
                    db: None,
                    root: PathBuf::from("."),
                    rebuild: true,
                }
            );
        }
        _ => panic!("expected sync command"),
    }
}
