use std::path::Path;

/// Returns `true` when `path` carries the `com.apple.decmpfs` extended
/// attribute, which marks an OneDrive-managed dataless (placeholder) file on
/// macOS.  Reading such files via `fs::read_to_string` blocks the calling
/// thread until OneDrive materialises the content — potentially indefinitely
/// under sync pressure, causing MCP request timeouts.
///
/// On non-macOS platforms this always returns `false`.
///
/// The check is a pure existence query: we ask the kernel for the attribute
/// size only (null buffer, size 0) which never triggers a network fetch.
#[cfg(target_os = "macos")]
pub fn is_dataless(path: &Path) -> bool {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let Ok(path_c) = CString::new(path.as_os_str().as_bytes()) else {
        return false;
    };
    let Ok(name_c) = CString::new("com.apple.decmpfs") else {
        return false;
    };
    // macOS getxattr(path, name, value, size, position, options).
    // Passing NULL + size 0 returns the attribute's byte-length (>= 0) if the
    // attribute exists, or -1 (ENOATTR) if it is absent.  The call never
    // reads file content and never triggers OneDrive materialisation.
    let result = unsafe {
        libc::getxattr(
            path_c.as_ptr(),
            name_c.as_ptr(),
            std::ptr::null_mut(),
            0,
            0,
            0,
        )
    };
    result >= 0
}

#[cfg(not(target_os = "macos"))]
pub fn is_dataless(_path: &Path) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn is_dataless_returns_false_for_normal_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("normal.md");
        std::fs::File::create(&file)
            .expect("create")
            .write_all(b"# Normal file\n")
            .expect("write");
        assert!(!is_dataless(&file), "normal file must not be flagged dataless");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn is_dataless_returns_true_when_xattr_set() {
        use std::process::Command;
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("dataless.md");
        std::fs::File::create(&file).expect("create file");
        // "fpmc" bytes in hex — the OneDrive compressed/dataless marker
        let status = Command::new("xattr")
            .args(["-wx", "com.apple.decmpfs", "66706d63"])
            .arg(&file)
            .status()
            .expect("xattr command");
        assert!(status.success(), "xattr -wx failed");
        assert!(is_dataless(&file), "file with com.apple.decmpfs must be flagged dataless");
    }
}
