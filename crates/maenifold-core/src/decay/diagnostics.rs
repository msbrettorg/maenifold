use crate::config::Config;
use std::path::Path;

// T-12.5: RTM NFR-1 centralizes decay diagnostics so disk and DB fallback paths
// keep emitting the same degraded messages after the module split.
pub(crate) fn emit_degraded_decay_diagnostic(
    method: &str,
    message: &str,
    file_identifier: &str,
    config: &Config,
) {
    emit_stderr_line(&format!(
        "[DEGRADED][MemorySearchTools.Fusion][{method}] {message} for '{}'",
        redact_diagnostic_path(file_identifier, config)
    ));
}

pub(crate) fn emit_system_tools_diagnostic(
    method: &str,
    message: &str,
    file_identifier: &str,
    config: &Config,
    detail: &str,
) {
    emit_stderr_line(&format!(
        "[DEGRADED][SystemTools][{method}] {message} for '{}': {detail}",
        redact_diagnostic_path(file_identifier, config)
    ));
}

pub(crate) fn redact_diagnostic_path(file_identifier: &str, config: &Config) -> String {
    if let Some(relative) = file_identifier.strip_prefix("memory://") {
        return relative
            .rsplit('/')
            .next()
            .filter(|segment| !segment.is_empty())
            .unwrap_or(relative)
            .to_string();
    }

    let path = Path::new(file_identifier);
    if path.is_absolute() && !path.starts_with(&config.memory_path) {
        return path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| file_identifier.to_string());
    }

    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .or_else(|| {
            path.file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| file_identifier.to_string())
}

#[cfg(unix)]
pub(crate) fn emit_stderr_line(message: &str) {
    unsafe extern "C" {
        fn write(fd: i32, buf: *const u8, count: usize) -> isize;
    }

    let line = format!("{message}\n");
    // SAFETY: the pointer remains valid for the duration of the syscall and fd=2 is stderr.
    let _ = unsafe { write(2, line.as_ptr(), line.len()) };
}

#[cfg(not(unix))]
pub(crate) fn emit_stderr_line(message: &str) {
    eprintln!("{message}");
}
