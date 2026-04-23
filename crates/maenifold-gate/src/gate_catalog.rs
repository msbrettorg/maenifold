use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

use crate::error::GateError;
use crate::gate_definition::GateDefinition;
use crate::task_metadata::TaskMetadata;

const RESERVED_BUILTIN_NAMES: &[&str] = &["sdd", "tdd", "gsd", "lightweight", "mdd", "slc", "sdd-c", "tdd-c", "test"];
const SDD_JSON: &str = include_str!("../assets/sdd.json");
const TDD_JSON: &str = include_str!("../assets/tdd.json");
const GSD_JSON: &str = include_str!("../assets/gsd.json");
const LIGHTWEIGHT_JSON: &str = include_str!("../assets/lightweight.json");
const MDD_JSON: &str = include_str!("../assets/mdd.json");
const SLC_JSON: &str = include_str!("../assets/slc.json");
const SDD_C_JSON: &str = include_str!("../assets/sdd-c.json");
const TDD_C_JSON: &str = include_str!("../assets/tdd-c.json");
const TEST_JSON: &str = include_str!("../assets/test.json");
static MAENIFOLD_ROOT_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[derive(Debug, Clone)]
pub struct GateDefinitionCatalog;

impl GateDefinitionCatalog {
    pub fn load_builtin(name: &str) -> Result<GateDefinition, GateError> {
        let _guard = maenifold_root_lock()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        load_builtin_inner(name)
    }

    pub fn load_custom(root: &Path) -> Result<GateDefinition, GateError> {
        let path = root.join("pipeline.json");
        let text = fs::read_to_string(&path)?;
        GateDefinition::from_json_text(&text)
    }

    pub fn resolve(root: &Path, name: &str) -> Result<GateDefinition, GateError> {
        let custom_path = root.join("pipeline.json");
        if custom_path.exists() && !RESERVED_BUILTIN_NAMES.contains(&name) {
            return Self::load_custom(root);
        }

        Self::load_builtin(name)
    }

    pub fn resolve_for_task(
        root: &Path,
        metadata: &TaskMetadata,
        requested_pipeline_name: Option<&str>,
    ) -> Result<GateDefinition, GateError> {
        if let Some(requested) = requested_pipeline_name
            && requested != metadata.pipeline
        {
            return Err(GateError::PipelineOverrideMismatch {
                requested: requested.to_string(),
                bound: metadata.pipeline.clone(),
            });
        }

        Self::resolve(root, &metadata.pipeline)
    }

    fn gates_dir() -> PathBuf {
        env::var_os("MAENIFOLD_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                env::var_os("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("maenifold")
            })
            .join("assets")
            .join("gates")
    }

    fn ensure_all_builtin_assets() -> Result<(), GateError> {
        let dir = Self::gates_dir();
        fs::create_dir_all(&dir)?;
        for name in RESERVED_BUILTIN_NAMES {
            let path = dir.join(format!("{name}.json"));
            let content = builtin_asset_text(name)?;
            match fs::read_to_string(&path) {
                Ok(existing) if existing == content => continue,
                Ok(_) => {
                    eprintln!(
                        "warning: refreshed bundled built-in gate definition '{name}' at {}",
                        path.display()
                    );
                    fs::write(path, content)?
                }
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => fs::write(path, content)?,
                Err(err) => return Err(err.into()),
            }
        }
        Ok(())
    }
}

fn load_builtin_inner(name: &str) -> Result<GateDefinition, GateError> {
    GateDefinitionCatalog::ensure_all_builtin_assets()?;
    let asset_path = GateDefinitionCatalog::gates_dir().join(format!("{name}.json"));
    let text = fs::read_to_string(&asset_path)?;
    GateDefinition::from_json_text(&text)
}

fn maenifold_root_lock() -> &'static Mutex<()> {
    &MAENIFOLD_ROOT_LOCK
}

fn builtin_asset_text(name: &str) -> Result<&'static str, GateError> {
    match name {
        "sdd" => Ok(SDD_JSON),
        "tdd" => Ok(TDD_JSON),
        "gsd" => Ok(GSD_JSON),
        "lightweight" => Ok(LIGHTWEIGHT_JSON),
        "mdd" => Ok(MDD_JSON),
        "slc" => Ok(SLC_JSON),
        "sdd-c" => Ok(SDD_C_JSON),
        "tdd-c" => Ok(TDD_C_JSON),
        "test" => Ok(TEST_JSON),
        _ => Err(GateError::UnknownBuiltinGate(name.to_string())),
    }
}

#[cfg(test)]
#[path = "gate_catalog_tests.rs"]
mod tests;
