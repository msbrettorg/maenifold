use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

fn isolated_root() -> TempDir {
    TempDir::new().expect("tmp")
}

fn maenifold_cmd() -> Command {
    // T-0.3g: RTM-047 - avoid deprecated assert_cmd constructor without changing test behavior.
    Command::new(assert_cmd::cargo::cargo_bin!("maenifold"))
}

#[test]
fn search_memories_json_no_longer_returns_placeholder_error() {
    let tmp = isolated_root();
    let mut cmd = maenifold_cmd();
    let output = cmd
        .env("MAENIFOLD_ROOT", tmp.path())
        .env("MAENIFOLD_DATABASE_PATH", tmp.path().join("memory.db"))
        .args([
            "--tool",
            "SearchMemories",
            "--payload",
            r#"{"query":"machine learning","mode":"Hybrid","pageSize":5,"page":1}"#,
            "--json",
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        !stdout.contains("NOT_IMPLEMENTED"),
        "SearchMemories should return a real JSON result contract, not the scaffold placeholder. Output: {stdout}"
    );

    let parsed: Value = serde_json::from_slice(&output).expect("json");
    assert!(
        parsed.is_object() || parsed.is_array(),
        "search JSON should be structured"
    );
}

#[test]
fn sync_output_no_longer_returns_placeholder_error() {
    let tmp = isolated_root();
    let mut cmd = maenifold_cmd();
    let output = cmd
        .env("MAENIFOLD_ROOT", tmp.path())
        .env("MAENIFOLD_DATABASE_PATH", tmp.path().join("memory.db"))
        .args(["--tool", "Sync", "--payload", "{}", "--json"])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        !stdout.contains("NOT_IMPLEMENTED"),
        "Sync should return a real completion payload, not the scaffold placeholder. Output: {stdout}"
    );

    let text = String::from_utf8(output).expect("utf8");
    assert!(
        text.contains("Sync complete:"),
        "sync output should follow the C# completion contract. Output: {text}"
    );
}

#[test]
fn search_memories_json_returns_seeded_result_after_sync() {
    let tmp = isolated_root();

    maenifold_cmd()
        .env("MAENIFOLD_ROOT", tmp.path())
        .env("MAENIFOLD_DATABASE_PATH", tmp.path().join("memory.db"))
        .args([
            "--tool",
            "WriteMemory",
            "--payload",
            r#"{"title":"Search Seed","content":"Testing [[search-index]] and [[machine-learning]]."}"#,
            "--json",
        ])
        .assert()
        .code(0);

    maenifold_cmd()
        .env("MAENIFOLD_ROOT", tmp.path())
        .env("MAENIFOLD_DATABASE_PATH", tmp.path().join("memory.db"))
        .args(["--tool", "Sync", "--payload", "{}", "--json"])
        .assert()
        .code(0);

    let output = maenifold_cmd()
        .env("MAENIFOLD_ROOT", tmp.path())
        .env("MAENIFOLD_DATABASE_PATH", tmp.path().join("memory.db"))
        .args([
            "--tool",
            "SearchMemories",
            "--payload",
            r#"{"query":"machine learning","mode":"Hybrid","pageSize":5,"page":1}"#,
            "--json",
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        stdout.contains("search-seed") || stdout.contains("Search Seed"),
        "expected seeded result missing: {stdout}"
    );
}

#[test]
fn build_context_cli_returns_neighboring_concepts() {
    let tmp = isolated_root();

    maenifold_cmd()
        .env("MAENIFOLD_ROOT", tmp.path())
        .env("MAENIFOLD_DATABASE_PATH", tmp.path().join("memory.db"))
        .args([
            "--tool",
            "WriteMemory",
            "--payload",
            r#"{"title":"Context Seed A","content":"This links [[authentication]] and [[oauth2]]."}"#,
            "--json",
        ])
        .assert()
        .code(0);

    maenifold_cmd()
        .env("MAENIFOLD_ROOT", tmp.path())
        .env("MAENIFOLD_DATABASE_PATH", tmp.path().join("memory.db"))
        .args([
            "--tool",
            "WriteMemory",
            "--payload",
            r#"{"title":"Context Seed B","content":"This links [[authentication]] and [[jwt]]."}"#,
            "--json",
        ])
        .assert()
        .code(0);

    maenifold_cmd()
        .env("MAENIFOLD_ROOT", tmp.path())
        .env("MAENIFOLD_DATABASE_PATH", tmp.path().join("memory.db"))
        .args(["--tool", "Sync", "--payload", "{}", "--json"])
        .assert()
        .code(0);

    let output = maenifold_cmd()
        .env("MAENIFOLD_ROOT", tmp.path())
        .env("MAENIFOLD_DATABASE_PATH", tmp.path().join("memory.db"))
        .args([
            "--tool",
            "BuildContext",
            "--payload",
            r#"{"conceptName":"authentication","depth":1}"#,
            "--json",
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        stdout.contains("oauth2"),
        "expected oauth2 neighbor missing: {stdout}"
    );
    assert!(
        stdout.contains("jwt"),
        "expected jwt neighbor missing: {stdout}"
    );
}
