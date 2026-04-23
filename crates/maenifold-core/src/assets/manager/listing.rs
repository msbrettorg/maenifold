// T-01.16: AssetManager list/catalog operations extracted from manager.rs.

use super::AssetManager;
use super::super::support::{
    asset_files_in_root, asset_is_multi_agent, emit_asset_degraded_diagnostic,
};
use super::super::types::{AssetCatalog, AssetKind};
use crate::error::Result;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

impl AssetManager {
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

    pub(super) fn load_kind_metadata(&self, kind: AssetKind) -> Result<Vec<Value>> {
        self.load_kind_metadata_from_root(&self.root, kind)
    }

    pub(super) fn load_kind_metadata_from_root(
        &self,
        root: &Path,
        kind: AssetKind,
    ) -> Result<Vec<Value>> {
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

    pub(super) fn asset_files(&self, kind: AssetKind) -> Result<Vec<PathBuf>> {
        asset_files_in_root(&self.root, kind)
    }
}
