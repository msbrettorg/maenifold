use super::paths::is_dataless_size_blocks;
#[cfg(unix)]
use super::paths::read_to_string_guarded_for_test;
use super::{read_markdown, write_markdown};
use crate::acp_config::MaenifoldSettings;
use crate::config::Config;
use crate::error::MaenifoldError;
use std::fs;
use std::io::Write as _;
use std::path::Path;
#[cfg(unix)]
use std::time::Duration;

fn test_config(root: &Path) -> Config {
    let memory_path = root.join("memory");
    Config {
        maenifold_root: root.to_path_buf(),
        memory_path: memory_path.clone(),
        assets_path: root.join("assets"),
        thinking_path: memory_path.join("thinking"),
        sequential_path: memory_path.join("thinking").join("sequential"),
        database_path: root.join("memory.db"),
        mcp_allowed_hosts: Vec::new(),
        mcp_allowed_origins: Vec::new(),
        settings: MaenifoldSettings::default(),
    }
}

#[test]
fn read_markdown_returns_dataless_error_when_blocks_zero() {
    assert!(is_dataless_size_blocks(64, 0));
    assert!(!is_dataless_size_blocks(0, 0));
    assert!(!is_dataless_size_blocks(64, 1));
}

#[test]
#[cfg(unix)]
fn read_markdown_times_out_on_blocked_read() {
    let temp = tempfile::tempdir().expect("create temp root");
    let config = test_config(temp.path());
    fs::create_dir_all(&config.memory_path).expect("create memory root");
    let fifo = config.memory_path.join("blocked.md");
    let path = std::ffi::CString::new(fifo.as_os_str().as_encoded_bytes()).expect("fifo path");
    let created = unsafe { libc::mkfifo(path.as_ptr(), 0o600) };
    assert_eq!(created, 0, "create fifo");

    let error = read_to_string_guarded_for_test(&fifo, Duration::from_millis(50))
        .expect_err("fifo read should time out");
    assert!(
        error.to_string().contains("file read timed out:"),
        "unexpected error: {error}"
    );
}

#[test]
fn read_markdown_succeeds_on_normal_file() {
    let temp = tempfile::tempdir().expect("create temp root");
    let config = test_config(temp.path());
    fs::create_dir_all(&config.memory_path).expect("create memory root");
    let mut frontmatter = super::Frontmatter::new();
    frontmatter.insert(
        "title".to_string(),
        serde_yml::Value::String("Normal".to_string()),
    );
    write_markdown("memory://normal", Some(&frontmatter), "body", &config).expect("write markdown");

    let (read_frontmatter, content, checksum) =
        read_markdown("memory://normal", &config).expect("read markdown");

    assert_eq!(content, "body");
    assert!(!checksum.is_empty());
    assert_eq!(
        read_frontmatter
            .expect("frontmatter")
            .get("title")
            .expect("title"),
        &serde_yml::Value::String("Normal".to_string())
    );
}

#[test]
fn read_markdown_returns_file_dataless_for_sparse_file_when_supported() {
    let temp = tempfile::tempdir().expect("create temp root");
    let config = test_config(temp.path());
    fs::create_dir_all(&config.memory_path).expect("create memory root");
    let sparse = config.memory_path.join("sparse.md");
    let mut file = fs::File::create(&sparse).expect("create sparse file");
    file.set_len(4096).expect("size sparse file");
    file.flush().expect("flush sparse file");

    if !super::paths::is_dataless(&fs::metadata(&sparse).expect("metadata")) {
        return;
    }

    let error = read_markdown("memory://sparse", &config).expect_err("dataless error");
    assert!(matches!(error, MaenifoldError::FileDataless { .. }));
}
