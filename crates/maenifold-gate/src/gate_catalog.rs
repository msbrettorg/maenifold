use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::GateError;
use crate::gate_definition::GateDefinition;
use crate::task_metadata::TaskMetadata;

const BUILTIN_GATES_DIR: &str = "/Users/brett/maenifold/assets/gates";
const RESERVED_BUILTIN_NAMES: &[&str] = &["sdd", "tdd", "gsd", "lightweight"];

#[derive(Debug, Clone)]
pub struct GateDefinitionCatalog;

impl GateDefinitionCatalog {
    pub fn load_builtin(name: &str) -> Result<GateDefinition, GateError> {
        let path = Path::new(BUILTIN_GATES_DIR).join(format!("{name}.json"));
        if !path.exists() {
            return Err(GateError::UnknownBuiltinGate(name.to_string()));
        }

        let bytes = fs::read(&path)?;
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
}

fn digest_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}
