use super::*;

#[test]
fn build_child_path_env_adds_common_user_tool_dirs_and_command_parent() {
    let path = build_child_path_env(
        Some(OsString::from("/usr/bin:/bin")),
        Some(OsString::from("/Users/tester")),
        "/Users/tester/.npm-global/bin/workiq",
    )
    .expect("path should be constructed");
    let entries: Vec<_> = std::env::split_paths(OsStr::new(&path))
        .map(|entry| entry.to_string_lossy().to_string())
        .collect();

    assert!(entries.iter().any(|entry| entry == "/usr/bin"));
    assert!(entries.iter().any(|entry| entry == "/bin"));
    assert!(
        entries
            .iter()
            .any(|entry| entry == "/Users/tester/.npm-global/bin")
    );
    assert!(
        entries
            .iter()
            .any(|entry| entry == "/Users/tester/.dotnet/tools")
    );
    assert!(entries.iter().any(|entry| entry == "/opt/homebrew/bin"));
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
