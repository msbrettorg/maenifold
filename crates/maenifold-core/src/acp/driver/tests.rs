use std::path::PathBuf;

use tempfile::tempdir;

use super::{AcpxDriver, AcpxError, map_command_failure, resolve_acpx_path};

#[test]
fn acpx_driver_maps_timeout_failures() {
    let error = map_command_failure(Some(124), "", "request timed out after 300 seconds", 300);

    match error {
        AcpxError::Timeout {
            timeout_secs,
            details,
        } => {
            assert_eq!(timeout_secs, 300);
            assert!(details.contains("timed out"));
        }
        other => panic!("expected timeout error, got {other:?}"),
    }
}

#[test]
fn acpx_driver_maps_non_zero_exit_codes() {
    let error = map_command_failure(Some(2), "", "permission denied", 300);

    match error {
        AcpxError::CommandFailed { code, details } => {
            assert_eq!(code, Some(2));
            assert!(details.contains("permission denied"));
        }
        other => panic!("expected command failure, got {other:?}"),
    }
}

#[test]
fn acpx_driver_resolves_binary_from_path() {
    let temp = tempdir().expect("tempdir");
    let acpx_path = temp.path().join("acpx");
    std::fs::write(&acpx_path, "#!/bin/sh\nexit 0\n").expect("write acpx stub");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&acpx_path)
            .expect("metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&acpx_path, permissions).expect("chmod");
    }

    let original_path = std::env::var_os("PATH");
    let combined_path = match &original_path {
        Some(existing) => {
            let mut paths = vec![temp.path().to_path_buf()];
            paths.extend(std::env::split_paths(existing));
            std::env::join_paths(paths).expect("join PATH")
        }
        None => std::env::join_paths([temp.path()]).expect("join PATH"),
    };

    unsafe {
        std::env::set_var("PATH", &combined_path);
    }
    let resolved = resolve_acpx_path().expect("acpx should resolve");
    match original_path {
        Some(path) => unsafe {
            std::env::set_var("PATH", path);
        },
        None => unsafe {
            std::env::remove_var("PATH");
        },
    }

    assert_eq!(resolved, acpx_path);
}

#[test]
fn acpx_driver_test_constructor_preserves_configuration() {
    let driver = AcpxDriver::new_for_tests(
        "codex".to_string(),
        42,
        PathBuf::from("/tmp/worktree"),
        PathBuf::from("/usr/local/bin/acpx"),
    );

    assert_eq!(driver.agent, "codex");
    assert_eq!(driver.timeout, 42);
    assert_eq!(driver.cwd, PathBuf::from("/tmp/worktree"));
    assert_eq!(driver.acpx_path, PathBuf::from("/usr/local/bin/acpx"));
}
