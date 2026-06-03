use crate::config::Config;

#[cfg(test)]
pub(crate) struct ConfigResetGuard;

#[cfg(test)]
static CONFIG_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
thread_local! {
    static CONFIG_TEST_GUARD: std::cell::RefCell<Option<std::sync::MutexGuard<'static, ()>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
impl ConfigResetGuard {
    pub(crate) fn acquire_test_lock() {
        // The mutex guards a unit `()` — there is no invariant to protect, so
        // poisoning is purely cosmetic. Recover the guard even when poisoned to
        // prevent a single panicking test from cascading into hundreds of
        // PoisonError-cascade failures across the whole suite.
        let guard = CONFIG_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        CONFIG_TEST_GUARD.with(|slot| {
            *slot.borrow_mut() = Some(guard);
        });
    }
}

#[cfg(test)]
impl Drop for ConfigResetGuard {
    fn drop(&mut self) {
        Config::reset_overrides();
        CONFIG_TEST_GUARD.with(|slot| {
            *slot.borrow_mut() = None;
        });
    }
}

/// Cross-platform path to a tiny "always exits 0" stub binary that lives in the
/// system temp dir for the lifetime of the test process. Used by tests that need
/// `MAENIFOLD_PIPELINE_DRIVER` or other "any-executable-path" fixtures to point
/// at a real on-disk binary. The stub is a Windows `.cmd` script or a unix shell
/// script with the executable bit set — both spawnable via
/// `std::process::Command::new(path)`.
#[cfg(test)]
pub(crate) fn fake_exit_zero_binary() -> std::path::PathBuf {
    use std::sync::OnceLock;
    static BIN: OnceLock<std::path::PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        let dir = std::env::temp_dir().join("maenifold-test-fake-bins");
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

/// Cross-platform "echo-equivalent" stub binary. Prints all arguments separated
/// by spaces, followed by a newline, to stdout — usable as a stand-in for
/// `/bin/echo` in env-resolver and adapter tests.
#[cfg(test)]
pub(crate) fn fake_echo_binary() -> std::path::PathBuf {
    use std::sync::OnceLock;
    static BIN: OnceLock<std::path::PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        let dir = std::env::temp_dir().join("maenifold-test-fake-bins");
        std::fs::create_dir_all(&dir).expect("create fake-bin dir");
        #[cfg(windows)]
        {
            let bin = dir.join("echo.cmd");
            // `echo` in cmd echoes its args literally; %* passes all args through.
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

/// Locate a Python 3 interpreter for stdio MCP server tests. Walks PATH for
/// the first matching candidate so spawn does not depend on the test process's
/// PATH being preserved by every subprocess crate.
#[cfg(test)]
pub(crate) fn locate_python3() -> std::path::PathBuf {
    use std::sync::OnceLock;
    static PY: OnceLock<std::path::PathBuf> = OnceLock::new();
    PY.get_or_init(|| {
        let candidates: &[&str] = if cfg!(windows) {
            &["python3.exe", "python.exe", "python3", "python"]
        } else {
            &["python3", "python"]
        };
        let path = std::env::var("PATH").unwrap_or_default();
        let sep = if cfg!(windows) { ';' } else { ':' };
        for dir in path.split(sep) {
            if dir.is_empty() {
                continue;
            }
            for candidate in candidates {
                let p = std::path::Path::new(dir).join(candidate);
                if p.is_file() {
                    return p;
                }
            }
        }
        panic!(
            "no python3/python interpreter on PATH; install Python 3 to run \
             stdio MCP integration tests"
        );
    })
    .clone()
}
