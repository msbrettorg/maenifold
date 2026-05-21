use super::routes::{ApiError, ApiResult};
use std::path::{Path as FsPath, PathBuf};

pub(crate) fn resolve_requested_file_path(path: &str, cwd: Option<&str>) -> ApiResult<PathBuf> {
    let requested = PathBuf::from(path);

    if requested.is_absolute() {
        return Ok(requested);
    }

    let cwd = cwd.ok_or_else(|| {
        ApiError::bad_request("relative file paths require a cwd query parameter")
    })?;
    let resolved = FsPath::new(cwd).join(requested);

    if resolved.is_absolute() {
        Ok(resolved)
    } else {
        Err(ApiError::bad_request("resolved file path must be absolute"))
    }
}
