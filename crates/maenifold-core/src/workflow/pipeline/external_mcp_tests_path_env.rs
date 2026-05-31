use super::*;

#[test]
fn build_child_path_env_adds_common_user_tool_dirs_and_command_parent() {
    // Use platform-native PATH and HOME so the assertions match what
    // `build_child_path_env` actually constructs via `Path::join` on the
    // current OS.
    let home = if cfg!(windows) {
        std::path::PathBuf::from(r"C:\Users\tester")
    } else {
        std::path::PathBuf::from("/Users/tester")
    };
    let command = home
        .join(if cfg!(unix) { ".npm-global/bin" } else { r".dotnet\tools" })
        .join("workiq");
    let existing_path: std::ffi::OsString = std::env::join_paths(if cfg!(unix) {
        vec![std::path::PathBuf::from("/usr/bin"), std::path::PathBuf::from("/bin")]
    } else {
        vec![
            std::path::PathBuf::from(r"C:\Windows\System32"),
            std::path::PathBuf::from(r"C:\Windows"),
        ]
    })
    .expect("join existing path");

    let path = build_child_path_env(
        Some(existing_path),
        Some(home.clone().into_os_string()),
        command.to_str().expect("utf-8 command"),
    )
    .expect("path should be constructed");
    let entries: Vec<std::path::PathBuf> = std::env::split_paths(OsStr::new(&path)).collect();

    let expect_contains = |needle: &std::path::Path, label: &str| {
        assert!(
            entries.iter().any(|entry| entry == needle),
            "{label} '{}' not in {:?}",
            needle.display(),
            entries
        );
    };

    if cfg!(unix) {
        expect_contains(std::path::Path::new("/usr/bin"), "/usr/bin");
        expect_contains(std::path::Path::new("/bin"), "/bin");
        expect_contains(&home.join(".npm-global/bin"), "~/.npm-global/bin");
        expect_contains(&home.join(".dotnet/tools"), "~/.dotnet/tools");
        expect_contains(std::path::Path::new("/opt/homebrew/bin"), "/opt/homebrew/bin");
    } else {
        // On Windows the production helper still adds the unix paths verbatim
        // (they are harmless because nothing on Windows resolves under them);
        // the cross-platform invariant we care about is that the existing PATH
        // entries are preserved and the command's parent is added.
        expect_contains(std::path::Path::new(r"C:\Windows\System32"), "System32");
        expect_contains(std::path::Path::new(r"C:\Windows"), "Windows");
    }
    // Cross-platform: command's parent dir is always pushed.
    expect_contains(
        command.parent().expect("command has parent"),
        "command parent",
    );
}

#[test]
fn build_child_path_env_deduplicates_existing_entries() {
    let path = build_child_path_env(
        Some(OsString::from("/usr/bin:/bin:/usr/bin")),
        Some(OsString::from("/Users/tester")),
        "/usr/bin/tool",
    )
    .expect("path should be constructed");
    let entries: Vec<_> = std::env::split_paths(OsStr::new(&path))
        .map(|entry| entry.to_string_lossy().to_string())
        .collect();

    assert_eq!(
        entries.iter().filter(|entry| *entry == "/usr/bin").count(),
        1
    );
}
