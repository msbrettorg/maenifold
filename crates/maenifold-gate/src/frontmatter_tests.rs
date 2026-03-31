use super::*;
use serde_yaml::Value as YamlValue;

#[test]
fn split_frontmatter_basic() {
    let input = "---\nid: T-1\nstatus: pending\n---\n\n# Title\nBody";
    let (yaml, body) = split_frontmatter(input).unwrap();
    assert_eq!(yaml, "id: T-1\nstatus: pending\n");
    assert_eq!(body, "# Title\nBody");
}

#[test]
fn split_frontmatter_none_when_missing() {
    assert!(split_frontmatter("# No frontmatter").is_none());
}

#[test]
fn parse_frontmatter_extracts_fields() {
    let input = "---\nid: T-1\nstatus: swe\npipeline: tdd\nattempts: 2\n---\n\n# Title";
    let (fm, body) = parse_frontmatter(input);
    let fm = fm.unwrap();
    assert_eq!(fm["id"], YamlValue::String("T-1".into()));
    assert_eq!(fm["status"], YamlValue::String("swe".into()));
    assert_eq!(fm["attempts"], YamlValue::Number(2.into()));
    assert_eq!(body, "# Title");
}

#[test]
fn parse_frontmatter_malformed_yaml_degrades() {
    let input = "---\n: invalid: yaml: [[\n---\n\nBody";
    let (fm, body) = parse_frontmatter(input);
    assert!(fm.is_none());
    assert_eq!(body, "Body");
}

#[test]
fn render_frontmatter_key_ordering() {
    let mut fm = Frontmatter::new();
    fm.insert("modified".into(), YamlValue::String("2026-01-01".into()));
    fm.insert("id".into(), YamlValue::String("T-1".into()));
    fm.insert("status".into(), YamlValue::String("swe".into()));
    let rendered = render_frontmatter(&fm).unwrap();
    let id_pos = rendered.find("id:").unwrap();
    let status_pos = rendered.find("status:").unwrap();
    let modified_pos = rendered.find("modified:").unwrap();
    assert!(id_pos < status_pos);
    assert!(status_pos < modified_pos);
}

#[test]
fn round_trip_frontmatter() {
    let mut fm = Frontmatter::new();
    fm.insert("id".into(), YamlValue::String("T-42".into()));
    fm.insert("status".into(), YamlValue::String("red".into()));
    fm.insert("pipeline".into(), YamlValue::String("tdd".into()));
    fm.insert("attempts".into(), YamlValue::Number(1.into()));
    let rendered = render_frontmatter(&fm).unwrap();
    let full = format!("---\n{rendered}\n---\n\n# Body");
    let (parsed, body) = parse_frontmatter(&full);
    let parsed = parsed.unwrap();
    assert_eq!(parsed["id"], YamlValue::String("T-42".into()));
    assert_eq!(parsed["status"], YamlValue::String("red".into()));
    assert_eq!(body, "# Body");
}

#[test]
fn write_and_read_task_frontmatter() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("T-1.md");
    let mut fm = Frontmatter::new();
    fm.insert("id".into(), YamlValue::String("T-1".into()));
    fm.insert("status".into(), YamlValue::String("pending".into()));
    write_task_with_frontmatter(&path, Some(&fm), "# Task\n\nBody").unwrap();
    let (read_fm, body) = read_task_frontmatter(&path).unwrap();
    let read_fm = read_fm.unwrap();
    assert_eq!(read_fm["id"], YamlValue::String("T-1".into()));
    assert_eq!(read_fm["status"], YamlValue::String("pending".into()));
    assert!(read_fm.contains_key("created"));
    assert!(read_fm.contains_key("modified"));
    assert!(body.contains("# Task"));
}

#[test]
fn write_task_uses_atomic_replace() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("T-1.md");
    let mut fm = Frontmatter::new();
    fm.insert("id".into(), YamlValue::String("T-1".into()));
    write_task_with_frontmatter(&path, Some(&fm), "# Body").unwrap();
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("# Body"));
    assert!(content.contains("id: T-1"));
    // Sibling temp file must not linger after rename
    assert!(!dir.path().join(".T-1.md.tmp").exists());
}

#[test]
fn update_task_frontmatter_modifies_in_place() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("T-2.md");
    let mut fm = Frontmatter::new();
    fm.insert("id".into(), YamlValue::String("T-2".into()));
    fm.insert("status".into(), YamlValue::String("pending".into()));
    write_task_with_frontmatter(&path, Some(&fm), "# Task\n\nBody").unwrap();
    update_task_frontmatter(&path, |fm| {
        fm.insert("status".into(), YamlValue::String("swe".into()));
    })
    .unwrap();
    let (read_fm, body) = read_task_frontmatter(&path).unwrap();
    let read_fm = read_fm.unwrap();
    assert_eq!(read_fm["status"], YamlValue::String("swe".into()));
    assert!(body.contains("# Task"));
}

#[test]
fn update_refuses_malformed_frontmatter() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("T-bad.md");
    std::fs::write(&path, "---\n: bad: [\n---\n\n# Broken\n").unwrap();
    let err = update_task_frontmatter(&path, |_| {}).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("frontmatter delimiters") && msg.contains("malformed"),
        "unexpected error: {msg}"
    );
    // Original file must be untouched
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.starts_with("---\n: bad:"));
}

#[test]
fn update_preserves_body_content() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("T-3.md");
    let body = "# Title\n\n## SWE\n\nDid stuff\n\n**Verdict**: PASS";
    let mut fm = Frontmatter::new();
    fm.insert("status".into(), YamlValue::String("swe".into()));
    write_task_with_frontmatter(&path, Some(&fm), body).unwrap();
    update_task_frontmatter(&path, |fm| {
        fm.insert("status".into(), YamlValue::String("red".into()));
    })
    .unwrap();
    let (_, read_body) = read_task_frontmatter(&path).unwrap();
    assert!(read_body.contains("## SWE"));
    assert!(read_body.contains("**Verdict**: PASS"));
}

#[test]
fn read_attempts_accepts_number_and_string() {
    let mut number = Frontmatter::new();
    number.insert("attempts".into(), YamlValue::Number(2.into()));
    assert_eq!(read_attempts(&number), 2);

    let mut string = Frontmatter::new();
    string.insert("attempts".into(), YamlValue::String("3".into()));
    assert_eq!(read_attempts(&string), 3);

    // Whitespace-padded string must also parse
    let mut padded = Frontmatter::new();
    padded.insert("attempts".into(), YamlValue::String(" 7 ".into()));
    assert_eq!(read_attempts(&padded), 7);

    // Non-numeric string falls back to 0
    let mut bad = Frontmatter::new();
    bad.insert("attempts".into(), YamlValue::String("nope".into()));
    assert_eq!(read_attempts(&bad), 0);

    // Missing key falls back to 0
    assert_eq!(read_attempts(&Frontmatter::new()), 0);
}
