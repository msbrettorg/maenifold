use std::path::{Path, PathBuf};

use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;

pub(super) fn resolve_node_cwd(context: &ToolContext, cwd: Option<&Path>) -> Result<PathBuf> {
    let resolved = match cwd {
        Some(path) if path.is_absolute() => path.to_path_buf(),
        Some(path) => context.config.maenifold_root.join(path),
        None => context.config.maenifold_root.clone(),
    };
    let canonical = resolved.canonicalize().map_err(|error| {
        MaenifoldError::Message(format!(
            "ERROR: Pipeline node cwd '{}' could not be resolved: {error}",
            resolved.display()
        ))
    })?;
    let root = context
        .config
        .maenifold_root
        .canonicalize()
        .map_err(|error| {
            MaenifoldError::Message(format!(
                "ERROR: Maenifold root '{}' could not be resolved: {error}",
                context.config.maenifold_root.display()
            ))
        })?;
    if !canonical.starts_with(&root) {
        return Err(MaenifoldError::Message(format!(
            "ERROR: Pipeline node cwd '{}' escapes maenifold root '{}'",
            canonical.display(),
            root.display()
        )));
    }
    if !canonical.is_dir() {
        return Err(MaenifoldError::Message(format!(
            "ERROR: Pipeline node cwd '{}' is not a directory",
            canonical.display()
        )));
    }
    Ok(canonical)
}
