#![allow(dead_code)]
use std::io;
use std::process::{Command, Stdio};

pub fn assert_semantically_equal(left: &str, right: &str) {
    let left = canonical_json(left).expect("left JSON canonicalization failed");
    let right = canonical_json(right).expect("right JSON canonicalization failed");
    assert_eq!(left, right, "JSON payloads differ semantically");
}

pub fn assert_semantically_equal_ignoring_fields(left: &str, right: &str, ignored_fields: &[&str]) {
    let left = canonical_json_with_ignored_fields(left, ignored_fields)
        .expect("left JSON canonicalization failed");
    let right = canonical_json_with_ignored_fields(right, ignored_fields)
        .expect("right JSON canonicalization failed");
    assert_eq!(left, right, "JSON payloads differ semantically");
}

pub fn canonical_json(input: &str) -> io::Result<String> {
    canonicalize(input, &[])
}

pub fn canonical_json_with_ignored_fields(
    input: &str,
    ignored_fields: &[&str],
) -> io::Result<String> {
    canonicalize(input, ignored_fields)
}

fn canonicalize(input: &str, ignored_fields: &[&str]) -> io::Result<String> {
    let python = python_command();
    let script = r#"
import json, sys

ignored = set(sys.argv[1:])

def normalize(value):
    if isinstance(value, dict):
        items = {}
        for k, v in sorted(value.items()):
            if k in ignored:
                continue
            items[k] = normalize(v)
        return items
    if isinstance(value, float) and value.is_integer():
        return int(value)
    if isinstance(value, list):
        normalized = [normalize(v) for v in value]
        if all(isinstance(v, dict) for v in normalized):
            def key_fn(item):
                return (item.get("id"), item.get("name"), json.dumps(item, sort_keys=True, separators=(",", ":")))
            return sorted(normalized, key=key_fn)
        return sorted(normalized, key=lambda item: json.dumps(item, sort_keys=True, separators=(",", ":")))
    return value

value = json.loads(sys.stdin.read())
print(json.dumps(normalize(value), sort_keys=True, separators=(",", ":")))
"#;
    let mut child = Command::new(python)
        .arg("-c")
        .arg(script)
        .args(ignored_fields)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    {
        use std::io::Write;
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "missing stdin"))?;
        stdin.write_all(input.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn python_command() -> &'static str {
    if Command::new("python3").arg("--version").output().is_ok() {
        "python3"
    } else {
        "python"
    }
}
