use assert_cmd::Command;
use tempfile::TempDir;

fn isolated_root() -> TempDir {
    TempDir::new().expect("tmp")
}

fn maenifold_cmd(tmp: &TempDir) -> Command {
    // T-0.3g: RTM-047 - avoid deprecated assert_cmd constructor without changing test behavior.
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("maenifold"));
    cmd.env("MAENIFOLD_ROOT", tmp.path())
        .env("MAENIFOLD_DATABASE_PATH", tmp.path().join("memory.db"));
    cmd
}

#[test]
fn find_similar_concepts_cli_returns_ranked_text_after_sync() {
    let tmp = isolated_root();
    for payload in [
        r#"{"title":"Similarity CLI Seed A","content":"Links [[authentication]] and [[oauth2]]."}"#,
        r#"{"title":"Similarity CLI Seed B","content":"Links [[authentication]] and [[jwt]]."}"#,
    ] {
        maenifold_cmd(&tmp)
            .args(["--tool", "WriteMemory", "--payload", payload, "--json"])
            .assert()
            .code(0);
    }
    maenifold_cmd(&tmp)
        .args(["--tool", "Sync", "--payload", "{}", "--json"])
        .assert()
        .code(0);

     let output = maenifold_cmd(&tmp)
         .args([
             "--tool",
             "FindSimilarConcepts",
             "--payload",
             r#"{"conceptName":"authentication","maxResults":5}"#,
         ])
         .assert()
         .code(0)
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(output).expect("utf8");
    assert!(
        text.contains("Similar concepts to 'authentication'"),
        "unexpected output: {text}"
    );
    assert!(text.contains("oauth2"), "missing oauth2 concept: {text}");
    assert!(text.contains("jwt"), "missing jwt concept: {text}");
}

#[test]
fn watcher_cli_start_and_stop_should_return_lifecycle_messages() {
    let tmp = isolated_root();

    let start = maenifold_cmd(&tmp)
        .args([
            "--tool",
            "StartWatcher",
            "--payload",
            r#"{"debounceMs":250}"#,
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let start_text = String::from_utf8(start).expect("utf8");
    assert!(
        start_text.starts_with("Started watching "),
        "unexpected start output: {start_text}"
    );
    assert!(
        start_text.contains(" for incremental sync (debounce: 250ms)"),
        "unexpected start output: {start_text}"
    );

    let stop = maenifold_cmd(&tmp)
        .args(["--tool", "StopWatcher", "--payload", "{}"])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let stop_text = String::from_utf8(stop).expect("utf8");
    assert_eq!(
        stop_text.trim(),
        "Watcher not running",
        "unexpected stop output: {stop_text}"
    );
}

#[test]
fn sync_cli_json_returns_csharp_text_contract() {
    let tmp = isolated_root();
    let output = maenifold_cmd(&tmp)
        .args(["--tool", "Sync", "--payload", "{}", "--json"])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(output).expect("utf8");
    assert!(text.contains("Sync complete:"), "unexpected output: {text}");
    assert!(
        text.contains("- STATUS: degraded"),
        "unexpected output: {text}"
    );
    assert!(
        text.contains("- WARNING_CODE VECTOR_TABLES_EMPTY_AFTER_SYNC"),
        "unexpected output: {text}"
    );
}

#[test]
fn sync_and_extract_concepts_cli_should_exclude_code_block_wikilinks() {
    let tmp = isolated_root();
    maenifold_cmd(&tmp)
        .args([
            "--tool",
            "WriteMemory",
            "--payload",
            r#"{"title":"Markdown Reader CLI Seed","content":"Visible [[authentication]] concept.\n\n```md\n[[ignored-in-code]]\n```\n\nInline `[[ignored-inline]]` should be excluded."}"#,
            "--json",
        ])
        .assert()
        .code(0);

    maenifold_cmd(&tmp)
        .args(["--tool", "Sync", "--payload", "{}", "--json"])
        .assert()
        .code(0);

    let output = maenifold_cmd(&tmp)
        .args([
            "--tool",
            "ExtractConceptsFromFile",
            "--payload",
            r#"{"identifier":"markdown-reader-cli-seed"}"#,
            "--json",
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).expect("utf8");

    assert!(
        text.contains("authentication"),
        "expected visible concept missing: {text}"
    );
    assert!(
        !text.contains("ignored-in-code"),
        "code-block concept should be excluded: {text}"
    );
    assert!(
        !text.contains("ignored-inline"),
        "inline-code concept should be excluded: {text}"
    );
}
