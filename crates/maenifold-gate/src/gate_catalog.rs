use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::GateError;
use crate::gate_definition::GateDefinition;
use crate::task_metadata::TaskMetadata;

const DEFAULT_RUNTIME_GATES_DIR: &str = "/Users/brett/maenifold/assets/gates";
const GATES_DIR_ENV: &str = "MAENIFOLD_GATE_ASSETS_DIR";
const RESERVED_BUILTIN_NAMES: &[&str] = &["sdd", "tdd", "gsd", "lightweight"];
const SDD_JSON: &str = include_str!("../assets/sdd.json");
const TDD_JSON: &str = include_str!("../assets/tdd.json");
const GSD_JSON: &str = include_str!("../assets/gsd.json");
const LIGHTWEIGHT_JSON: &str = include_str!("../assets/lightweight.json");

#[derive(Debug, Clone)]
pub struct GateDefinitionCatalog;

impl GateDefinitionCatalog {
    pub fn load_builtin(name: &str) -> Result<GateDefinition, GateError> {
        Self::ensure_all_builtin_assets()?;
        let asset_path = Self::runtime_gates_dir().join(format!("{name}.json"));
        let bytes = fs::read(&asset_path)?;
        let digest = digest_hex(&bytes);
        let text = String::from_utf8(bytes)
            .map_err(|error| GateError::TaskMetadataParse(format!("gate asset is not utf-8: {error}")))?;
        GateDefinition::from_json_text(&text, digest)
    }

    pub fn load_custom(root: &Path) -> Result<GateDefinition, GateError> {
        let path = root.join("pipeline.json");
        let bytes = fs::read(&path)?;
        let digest = digest_hex(&bytes);
        let text = String::from_utf8(bytes).map_err(|error| {
            GateError::TaskMetadataParse(format!("custom pipeline asset is not utf-8: {error}"))
        })?;
        GateDefinition::from_json_text(&text, digest)
    }

    pub fn resolve(root: &Path, name: &str) -> Result<GateDefinition, GateError> {
        let custom_path = root.join("pipeline.json");
        if custom_path.exists() && !RESERVED_BUILTIN_NAMES.contains(&name) {
            return Self::load_custom(root);
        }

        Self::load_builtin(name)
    }

    pub fn resolve_bound(root: &Path, name: &str, digest: &str) -> Result<GateDefinition, GateError> {
        let definition = Self::resolve(root, name)?;
        if definition.digest != digest {
            return Err(GateError::PipelineDigestMismatch {
                pipeline: name.to_string(),
                task_digest: digest.to_string(),
                authoritative_digest: definition.digest.clone(),
            });
        }

        Ok(definition)
    }

    pub fn resolve_for_task(
        root: &Path,
        metadata: &TaskMetadata,
        requested_pipeline_name: Option<&str>,
    ) -> Result<GateDefinition, GateError> {
        if let Some(requested) = requested_pipeline_name {
            if requested != metadata.pipeline {
                return Err(GateError::PipelineOverrideMismatch {
                    requested: requested.to_string(),
                    bound: metadata.pipeline.clone(),
                });
            }
        }

        Self::resolve_bound(root, &metadata.pipeline, &metadata.pipeline_digest)
    }

    fn runtime_gates_dir() -> PathBuf {
        env::var_os(GATES_DIR_ENV)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_RUNTIME_GATES_DIR))
    }

    fn ensure_all_builtin_assets() -> Result<(), GateError> {
        let dir = Self::runtime_gates_dir();
        fs::create_dir_all(&dir)?;
        for name in RESERVED_BUILTIN_NAMES {
            let path = dir.join(format!("{name}.json"));
            if path.exists() {
                continue;
            }
            let content = builtin_asset_text(name)?;
            fs::write(&path, content)?;
        }
        Ok(())
    }
}

fn builtin_asset_text(name: &str) -> Result<&'static str, GateError> {
    match name {
        "sdd" => Ok(SDD_JSON),
        "tdd" => Ok(TDD_JSON),
        "gsd" => Ok(GSD_JSON),
        "lightweight" => Ok(LIGHTWEIGHT_JSON),
        _ => Err(GateError::UnknownBuiltinGate(name.to_string())),
    }
}

fn digest_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}
