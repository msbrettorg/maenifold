use agent_client_protocol::{self as acp};
use serde_json::Value;
use std::path::{Component, Path, PathBuf};

pub(super) async fn read_text_file(
    workspace_root: &Path,
    args: acp::ReadTextFileRequest,
) -> acp::Result<acp::ReadTextFileResponse> {
    let root = canonical_workspace_root(workspace_root)?;
    let path = resolve_existing_path(&args.path, &root)?;
    let full = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| acp::Error::new(acp::ErrorCode::InternalError.into(), e.to_string()))?;

    let content = match (args.line, args.limit) {
        (None, None) => full,
        (start, limit) => {
            let start_idx = start.map(|l| l.saturating_sub(1) as usize).unwrap_or(0);
            let lines: Vec<&str> = full.lines().collect();
            let slice = &lines[start_idx.min(lines.len())..];
            let slice = if let Some(n) = limit {
                &slice[..n.min(slice.len() as u32) as usize]
            } else {
                slice
            };
            slice.join("\n")
        }
    };

    Ok(acp::ReadTextFileResponse::new(content))
}

pub(super) async fn write_text_file(
    workspace_root: &Path,
    args: acp::WriteTextFileRequest,
) -> acp::Result<acp::WriteTextFileResponse> {
    let root = canonical_workspace_root(workspace_root)?;
    let create_intermediates = create_intermediates(&args.meta);
    let path = resolve_write_path(&args.path, &root, create_intermediates)?;
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        tokio::fs::create_dir_all(parent).await.map_err(|e| {
            acp::Error::new(
                acp::ErrorCode::InternalError.into(),
                format!("failed to create write parent '{}': {e}", parent.display()),
            )
        })?;
        let canonical_parent = std::fs::canonicalize(parent).map_err(|e| {
            fs_error(format!(
                "failed to resolve created write parent '{}': {e}",
                parent.display()
            ))
        })?;
        ensure_contained(&canonical_parent, &root)?;
    }
    tokio::fs::write(&path, &args.content)
        .await
        .map_err(|e| acp::Error::new(acp::ErrorCode::InternalError.into(), e.to_string()))?;
    Ok(acp::WriteTextFileResponse::new())
}

pub(super) fn fs_error(message: impl Into<String>) -> acp::Error {
    acp::Error::new(acp::ErrorCode::InvalidParams.into(), message.into())
}

pub(super) fn canonical_workspace_root(root: &Path) -> acp::Result<PathBuf> {
    std::fs::canonicalize(root).map_err(|e| {
        acp::Error::new(
            acp::ErrorCode::InternalError.into(),
            format!(
                "failed to resolve ACP workspace root '{}': {e}",
                root.display()
            ),
        )
    })
}

pub(super) fn ensure_contained(path: &Path, root: &Path) -> acp::Result<()> {
    if path.starts_with(root) {
        Ok(())
    } else {
        Err(fs_error(format!(
            "ACP filesystem path '{}' escapes workspace root '{}'",
            path.display(),
            root.display()
        )))
    }
}

pub(super) fn resolve_existing_path(path: &Path, root: &Path) -> acp::Result<PathBuf> {
    validate_absolute(path)?;
    let canonical = std::fs::canonicalize(path).map_err(|e| {
        fs_error(format!(
            "failed to resolve filesystem path '{}': {e}",
            path.display()
        ))
    })?;
    ensure_contained(&canonical, root)?;
    Ok(canonical)
}

pub(super) fn resolve_write_path(
    path: &Path,
    root: &Path,
    create_intermediates: bool,
) -> acp::Result<PathBuf> {
    validate_absolute(path)?;
    if create_intermediates && path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(fs_error(
            "Write paths must not contain '..' components when creating intermediates.",
        ));
    }
    let existing_ancestor = canonical_existing_ancestor(path)?;
    ensure_contained(&existing_ancestor, root)?;

    if std::fs::symlink_metadata(path).is_ok() {
        return resolve_existing_path(path, root);
    }

    let parent = path.parent().ok_or_else(|| {
        fs_error(format!(
            "ACP write path has no parent: '{}'",
            path.display()
        ))
    })?;

    if parent.exists() {
        let canonical_parent = std::fs::canonicalize(parent).map_err(|e| {
            fs_error(format!(
                "failed to resolve write parent '{}': {e}",
                parent.display()
            ))
        })?;
        ensure_contained(&canonical_parent, root)?;
    } else if !create_intermediates {
        return Err(fs_error(format!(
            "ACP write parent '{}' does not exist; set _meta.createIntermediates=true to create it",
            parent.display()
        )));
    }

    Ok(path.to_path_buf())
}

pub(super) fn create_intermediates(meta: &Option<acp::Meta>) -> bool {
    meta.as_ref()
        .and_then(|m| m.get("createIntermediates"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn validate_absolute(path: &Path) -> acp::Result<()> {
    if path.is_absolute() {
        Ok(())
    } else {
        Err(fs_error(format!(
            "ACP filesystem path must be absolute: '{}'",
            path.display()
        )))
    }
}

fn canonical_existing_ancestor(path: &Path) -> acp::Result<PathBuf> {
    let mut current = path;
    loop {
        if std::fs::symlink_metadata(current).is_ok() {
            return std::fs::canonicalize(current).map_err(|e| {
                fs_error(format!(
                    "failed to resolve filesystem path '{}': {e}",
                    current.display()
                ))
            });
        }
        current = current.parent().ok_or_else(|| {
            fs_error(format!(
                "ACP filesystem path has no existing ancestor: '{}'",
                path.display()
            ))
        })?;
    }
}
