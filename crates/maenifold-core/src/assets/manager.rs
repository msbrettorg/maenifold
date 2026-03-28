use super::support::{
    asset_files_in_root, asset_is_multi_agent, emit_asset_degraded_diagnostic,
    resolve_packaged_assets_root,
};
use super::types::{AssetCatalog, AssetKind};
use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct AssetManager {
    root: PathBuf,
}

impl AssetManager {
    pub fn new(config: &Config) -> Self {
        Self {
            root: config.assets_path.clone(),
        }
    }

    // T-ASSET-PATH: RTM FR-3, FR-20 — use runtime asset resolution instead
    // of compile-time CARGO_MANIFEST_DIR so the binary works on any machine.
    pub fn initialize_assets(config: &Config) -> Result<()> {
        config.ensure_directories()?;
        let packaged_root = match resolve_packaged_assets_root() {
            Some(p) => p,
            None => return Ok(()), // No packaged assets found — graceful
        };

        for entry in walkdir::WalkDir::new(&packaged_root)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            let src = entry.path();
            let relative = match src.strip_prefix(&packaged_root) {
                Ok(relative) if !relative.as_os_str().is_empty() => relative,
                _ => continue,
            };
            if should_skip_packaged_symlink_entry(src)? {
                continue;
            }
            let dst = config.assets_path.join(relative);
            if entry.file_type().is_dir() {
                fs::create_dir_all(&dst)?;
                continue;
            }
            if dst.exists() {
                continue;
            }
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }
            if let Err(error) = fs::copy(src, &dst) {
                if should_skip_initialize_asset_copy(src, &error) {
                    continue;
                }
                return Err(error.into());
            }
        }
        Ok(())
    }

    pub fn list_assets(&self, kind: Option<AssetKind>) -> Result<Value> {
        if let Some(kind) = kind {
            let values = self.load_kind_metadata(kind)?;
            Ok(Value::Array(values))
        } else {
            Ok(Value::Array(
                AssetKind::all()
                    .into_iter()
                    .map(|kind| Value::String(kind.as_str().to_string()))
                    .collect(),
            ))
        }
    }

    pub fn catalog(&self) -> Result<AssetCatalog> {
        let workflows = self.load_kind_metadata_from_root(&self.root, AssetKind::Workflow)?;
        // T-0.2: RTM NFR-3 - catalog shape mirrors the C# oracle by splitting workflows on `multiAgent`.
        let (multi_agent_workflows, single_agent_workflows): (Vec<_>, Vec<_>) =
            workflows.into_iter().partition(asset_is_multi_agent);
        Ok(AssetCatalog {
            single_agent_workflows,
            multi_agent_workflows,
            roles: self.load_kind_metadata_from_root(&self.root, AssetKind::Role)?,
            colors: self.load_kind_metadata_from_root(&self.root, AssetKind::Color)?,
            perspectives: self.load_kind_metadata_from_root(&self.root, AssetKind::Perspective)?,
        })
    }

    pub fn read_resource(&self, uri: &str) -> Result<Value> {
        if uri.eq_ignore_ascii_case("asset://catalog") {
            let catalog = self.catalog()?;
            return serde_json::to_value(catalog).map_err(Into::into);
        }
        let Some(rest) = uri.strip_prefix("asset://") else {
            return Err(MaenifoldError::InvalidResourceUri(format!(
                "{uri}. Expected 'asset://type/id'"
            )));
        };
        let mut parts = rest.splitn(2, '/');
        let kind = parts.next().ok_or_else(|| {
            MaenifoldError::InvalidResourceUri(format!("{uri}. Expected 'asset://type/id'"))
        })?;
        let id = parts.next().ok_or_else(|| {
            MaenifoldError::InvalidResourceUri(format!("{uri}. Expected 'asset://type/id'"))
        })?;
        let kind = AssetKind::parse(kind)?;
        self.read_asset(kind, id)
    }

    pub fn read_asset(&self, kind: AssetKind, id: &str) -> Result<Value> {
        let candidates = self.asset_files(kind)?;
        let target = id.to_ascii_lowercase();
        for path in &candidates {
            // T-0.3g: RTM FR-3, RTM-047 - keep asset lookup behavior while satisfying clippy.
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                && stem.eq_ignore_ascii_case(&target)
            {
                return Ok(serde_json::from_str(&fs::read_to_string(path)?)?);
            }
        }

        // T-0.2: RTM NFR-3 - ReadMcpResource must fall back to the JSON `id` field like the C# oracle.
        for path in candidates {
            let content = fs::read_to_string(&path)?;
            let value: Value = match serde_json::from_str(&content) {
                Ok(value) => value,
                Err(error) => {
                    // T-0.1: RTM FR-25.2 preserves degraded asset lookup diagnostics when malformed JSON is skipped.
                    emit_asset_degraded_diagnostic("FindAssetFileById", &path, &error);
                    continue;
                }
            };
            if value
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|asset_id| asset_id.eq_ignore_ascii_case(&target))
            {
                return Ok(value);
            }
        }

        Err(MaenifoldError::AssetNotFound(
            kind.as_str().to_string(),
            id.to_string(),
        ))
    }

    fn load_kind_metadata(&self, kind: AssetKind) -> Result<Vec<Value>> {
        self.load_kind_metadata_from_root(&self.root, kind)
    }

    fn load_kind_metadata_from_root(&self, root: &Path, kind: AssetKind) -> Result<Vec<Value>> {
        let mut items = Vec::new();
        for path in asset_files_in_root(root, kind)? {
            let text = fs::read_to_string(&path)?;
            let value: Value = match serde_json::from_str(&text) {
                Ok(value) => value,
                Err(error) => {
                    // T-0.1: RTM FR-25.2 preserves degraded metadata listing diagnostics when malformed JSON is skipped.
                    emit_asset_degraded_diagnostic("GetAssetMetadata", &path, &error);
                    continue;
                }
            };
            let mut metadata = serde_json::Map::new();
            for field in ["id", "name", "emoji", "description"] {
                if let Some(v) = value.get(field).cloned() {
                    metadata.insert(field.to_string(), v);
                }
            }
            // T-0.2: RTM NFR-3 - C# metadata always includes `multiAgent`, defaulting to false.
            metadata.insert(
                "multiAgent".to_string(),
                Value::Bool(
                    value
                        .get("multiAgent")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                ),
            );
            items.push(Value::Object(metadata));
        }
        Ok(items)
    }

    fn asset_files(&self, kind: AssetKind) -> Result<Vec<PathBuf>> {
        asset_files_in_root(&self.root, kind)
    }

    pub(crate) fn read_adopt_asset(&self, kind: AssetKind, identifier: &str) -> Result<String> {
        let identifier = super::support::validate_adopt_identifier(identifier)?;

        // T-1.1: RTM FR-1 - prefer direct filename lookup before scanning asset metadata.
        let fast_path = self
            .root
            .join(kind.dir_name())
            .join(format!("{identifier}.json"));
        if fast_path.is_file() {
            return Ok(fs::read_to_string(fast_path)?);
        }

        // T-1.1: RTM FR-1 - fall back to matching the JSON `id` field, skipping malformed JSON.
        for path in self.asset_files(kind)? {
            let content = fs::read_to_string(&path)?;
            let Ok(value) = serde_json::from_str::<Value>(&content) else {
                continue;
            };

            if value.get("id").and_then(Value::as_str) == Some(identifier) {
                return Ok(content);
            }
        }

        Err(MaenifoldError::AssetNotFound(
            kind.as_str().to_string(),
            identifier.to_string(),
        ))
    }
}

fn should_skip_packaged_symlink_entry(src: &Path) -> Result<bool> {
    if !src.is_symlink() {
        return Ok(false);
    }

    match fs::metadata(src) {
        Ok(metadata) => Ok(metadata.is_dir()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(error) => Err(error.into()),
    }
}

fn should_skip_initialize_asset_copy(src: &Path, error: &std::io::Error) -> bool {
    if !src.is_symlink() {
        return false;
    }

    if error.kind() == std::io::ErrorKind::NotFound {
        return true;
    }

    if error.kind() != std::io::ErrorKind::InvalidInput {
        return false;
    }

    fs::metadata(src)
        .map(|metadata| metadata.is_dir())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;
    #[cfg(unix)]
    use std::os::unix::fs::symlink;
    use tempfile::TempDir;

    #[cfg(unix)]
    #[test]
    fn initialize_assets_skips_broken_symlink_copy_errors() {
        let tmp = TempDir::new().expect("temp dir");
        let src = tmp.path().join("broken-symlink");
        symlink("missing-target", &src).expect("create broken symlink");

        assert!(should_skip_packaged_symlink_entry(&src).expect("helper result"));
        assert!(should_skip_initialize_asset_copy(
            &src,
            &std::io::Error::from(ErrorKind::NotFound)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn initialize_assets_skips_directory_symlink_copy_errors() {
        let tmp = TempDir::new().expect("temp dir");
        let target = tmp.path().join("skill-dir");
        fs::create_dir_all(&target).expect("create target dir");
        let src = tmp.path().join("skill-link");
        symlink(&target, &src).expect("create dir symlink");

        assert!(should_skip_packaged_symlink_entry(&src).expect("helper result"));
        assert!(should_skip_initialize_asset_copy(
            &src,
            &std::io::Error::from(ErrorKind::InvalidInput)
        ));
    }
}
