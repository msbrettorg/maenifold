// T-01.16: AssetManager resource/asset/adopt lookups extracted from manager.rs.

use super::super::support::emit_asset_degraded_diagnostic;
use super::super::types::AssetKind;
use super::AssetManager;
use crate::error::{MaenifoldError, Result};
use serde_json::Value;
use std::fs;

impl AssetManager {
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

    pub(crate) fn read_adopt_asset(&self, kind: AssetKind, identifier: &str) -> Result<String> {
        let identifier = super::super::support::validate_adopt_identifier(identifier)?;

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
