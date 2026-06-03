//! Cross-platform test fixtures for jockey tests. Mirrors the helper set in
//! `maenifold-core::test_util` but kept local because cross-crate test-only
//! exports require declaring `pub` on the helper plus `dev-dependencies` cycle.

/// Cross-platform "always exits 0" stub binary.
#[allow(dead_code)]
pub(crate) fn fake_exit_zero_binary() -> std::path::PathBuf {
    use std::sync::OnceLock;
    static BIN: OnceLock<std::path::PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        let dir = std::env::temp_dir().join("maenifold-jockey-test-fake-bins");
        std::fs::create_dir_all(&dir).expect("create fake-bin dir");
        #[cfg(windows)]
        {
            let bin = dir.join("exit-zero.cmd");
            std::fs::write(&bin, "@exit 0\r\n").expect("write fake exit-zero stub");
            bin
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let bin = dir.join("exit-zero.sh");
            std::fs::write(&bin, "#!/bin/sh\nexit 0\n").expect("write fake exit-zero stub");
            std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755))
                .expect("chmod fake exit-zero stub");
            bin
        }
    })
    .clone()
}

/// Cross-platform "echo-equivalent" stub binary that prints its arguments.
pub(crate) fn fake_echo_binary() -> std::path::PathBuf {
    use std::sync::OnceLock;
    static BIN: OnceLock<std::path::PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        let dir = std::env::temp_dir().join("maenifold-jockey-test-fake-bins");
        std::fs::create_dir_all(&dir).expect("create fake-bin dir");
        #[cfg(windows)]
        {
            let bin = dir.join("echo.cmd");
            std::fs::write(&bin, "@echo off\r\nsetlocal\r\necho %*\r\n")
                .expect("write fake echo stub");
            bin
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let bin = dir.join("echo.sh");
            std::fs::write(&bin, "#!/bin/sh\nprintf '%s\\n' \"$*\"\n")
                .expect("write fake echo stub");
            std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755))
                .expect("chmod fake echo stub");
            bin
        }
    })
    .clone()
}

/// Cross-platform "always exits 1" stub binary (false-equivalent).
pub(crate) fn fake_exit_one_binary() -> std::path::PathBuf {
    use std::sync::OnceLock;
    static BIN: OnceLock<std::path::PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        let dir = std::env::temp_dir().join("maenifold-jockey-test-fake-bins");
        std::fs::create_dir_all(&dir).expect("create fake-bin dir");
        #[cfg(windows)]
        {
            let bin = dir.join("exit-one.cmd");
            std::fs::write(&bin, "@exit 1\r\n").expect("write fake exit-one stub");
            bin
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let bin = dir.join("exit-one.sh");
            std::fs::write(&bin, "#!/bin/sh\nexit 1\n").expect("write fake exit-one stub");
            std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755))
                .expect("chmod fake exit-one stub");
            bin
        }
    })
    .clone()
}
