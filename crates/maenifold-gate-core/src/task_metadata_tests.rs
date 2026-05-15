use super::{TaskMetadataParser, is_linked_session_id};

fn parse_yaml(yaml: &str) -> crate::task_metadata::TaskMetadata {
    let lines: Vec<_> = yaml.lines().collect();
    TaskMetadataParser::parse(&lines).expect("parse metadata")
}

// --- Tests migrated from the inline block ---

#[test]
fn parses_optional_session_id_from_frontmatter() {
    let metadata = parse_yaml(
        "---\nid: T-108.2\ntitle: Linked task\npipeline: sdd\nsessionId: session-1775916753112-57359\n---",
    );
    assert_eq!(
        metadata.session_id.as_deref(),
        Some("session-1775916753112-57359")
    );
    assert!(is_linked_session_id(metadata.session_id.as_deref()));
}

#[test]
fn ignores_non_string_session_id_values() {
    let metadata =
        parse_yaml("---\ntitle: Lenient task\npipeline: sdd\nsessionId:\n  - malformed\n---");
    assert_eq!(metadata.session_id, None);
    assert!(!is_linked_session_id(metadata.session_id.as_deref()));
}

#[test]
fn parse_task_id_accepts_valid_ids_and_rejects_path_traversal() {
    let valid = "---\nid: T-138.4\ntitle: Valid task\npipeline: sdd\n---\n# Valid task\n";
    assert_eq!(
        TaskMetadataParser::parse_task_id(valid).expect("valid id"),
        Some("T-138.4".to_string())
    );

    for invalid in ["../../escape", "foo/bar", "foo\\bar"] {
        let yaml = format!("---\nid: {invalid}\ntitle: bad\npipeline: sdd\n---\n# bad\n");
        assert!(TaskMetadataParser::parse_task_id(&yaml).is_err());
    }
}

#[test]
fn parses_depends_on_string_list_alias_and_dedups() {
    let cases = [
        ("depends_on: T-237.11", vec!["T-237.11"]),
        (
            "depends_on: [T-237.11, T-238.24]",
            vec!["T-237.11", "T-238.24"],
        ),
        ("depends: T-237.11", vec!["T-237.11"]),
        (
            "depends_on: [T-1, T-2]\ndepends: [T-2, T-3]",
            vec!["T-1", "T-2", "T-3"],
        ),
        ("depends_on: ['  T-1  ', '', '   ', T-1]", vec!["T-1"]),
    ];

    for (depends_yaml, expected) in cases {
        let yaml = format!("---\nid: T-9.1\ntitle: deps\npipeline: sdd\n{depends_yaml}\n---");
        let metadata = parse_yaml(&yaml);
        assert_eq!(metadata.depends_on, expected);
    }
}

#[test]
fn parses_blocked_frontmatter_fields() {
    let metadata = parse_yaml(
        "---\nid: T-309.7\ntitle: blocked task\npipeline: gsd\nblocked: true\nblocked_reason: waiting on user authorization\nsuperseded_by: [T-309.8, T-309.9]\n---",
    );
    assert!(metadata.blocked);
    assert_eq!(
        metadata.blocked_reason.as_deref(),
        Some("waiting on user authorization")
    );
    assert_eq!(
        metadata.superseded_by,
        Some(vec!["T-309.8".to_string(), "T-309.9".to_string()])
    );
}

#[test]
fn missing_blocked_frontmatter_defaults_to_open_task() {
    let metadata = parse_yaml("---\nid: T-309.7\ntitle: open task\npipeline: gsd\n---");
    assert!(!metadata.blocked);
    assert_eq!(metadata.blocked_reason, None);
    assert_eq!(metadata.superseded_by, None);
}

#[test]
fn rejects_self_dependency() {
    let yaml = "---\nid: T-1.1\ntitle: self dep\npipeline: sdd\ndepends_on: [T-0.1, T-1.1]\n---";
    let lines: Vec<_> = yaml.lines().collect();
    let error = TaskMetadataParser::parse(&lines).expect_err("self dependency should fail");
    assert!(error.to_string().contains("task depends on itself"));
}

// --- New priority / sprint tests (T-250.0.1) ---

#[test]
fn missing_priority_yields_none() {
    let metadata = parse_yaml("---\nid: T-250.1\ntitle: task\npipeline: sdd\n---");
    assert_eq!(metadata.priority, None);
}

#[test]
fn valid_priority_p0_is_parsed() {
    let metadata = parse_yaml("---\nid: T-250.1\ntitle: task\npipeline: sdd\npriority: P0\n---");
    assert_eq!(metadata.priority.as_deref(), Some("P0"));
}

#[test]
fn invalid_priority_p5_returns_error() {
    let yaml = "---\nid: T-250.1\ntitle: task\npipeline: sdd\npriority: P5\n---";
    let lines: Vec<_> = yaml.lines().collect();
    let error = TaskMetadataParser::parse(&lines).expect_err("P5 should be rejected");
    assert!(
        error.to_string().to_ascii_lowercase().contains("priority"),
        "error should mention 'priority', got: {error}"
    );
}

#[test]
fn missing_sprint_yields_none() {
    let metadata = parse_yaml("---\nid: T-250.1\ntitle: task\npipeline: sdd\n---");
    assert_eq!(metadata.sprint, None);
}

#[test]
fn valid_sprint_243_is_parsed() {
    let metadata = parse_yaml("---\nid: T-250.1\ntitle: task\npipeline: sdd\nsprint: 243\n---");
    assert_eq!(metadata.sprint, Some(243));
}

#[test]
fn malformed_sprint_abc_returns_error() {
    let yaml = "---\nid: T-250.1\ntitle: task\npipeline: sdd\nsprint: abc\n---";
    let lines: Vec<_> = yaml.lines().collect();
    assert!(
        TaskMetadataParser::parse(&lines).is_err(),
        "sprint: abc should fail to deserialize"
    );
}
