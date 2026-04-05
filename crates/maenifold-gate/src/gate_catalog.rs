use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

use crate::error::GateError;
use crate::gate_definition::GateDefinition;
use crate::task_metadata::TaskMetadata;

const RESERVED_BUILTIN_NAMES: &[&str] = &["sdd", "tdd", "gsd", "lightweight", "mdd"];
const SDD_JSON: &str = include_str!("../assets/sdd.json");
const TDD_JSON: &str = include_str!("../assets/tdd.json");
const GSD_JSON: &str = include_str!("../assets/gsd.json");
const LIGHTWEIGHT_JSON: &str = include_str!("../assets/lightweight.json");
const MDD_JSON: &str = include_str!("../assets/mdd.json");
static MAENIFOLD_ROOT_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[derive(Debug, Clone)]
pub struct GateDefinitionCatalog;

impl GateDefinitionCatalog {
    pub fn load_builtin(name: &str) -> Result<GateDefinition, GateError> {
        let _guard = maenifold_root_lock().lock().unwrap_or_else(|e| e.into_inner());
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
        _ => Err(GateError::UnknownBuiltinGate(name.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use tempfile::TempDir;

    use super::{GateDefinitionCatalog, SDD_JSON};

    fn set_maenifold_root(temp: &std::path::Path) -> (Option<OsString>, std::path::PathBuf) {
        let previous = std::env::var_os("MAENIFOLD_ROOT");
        unsafe {
            std::env::set_var("MAENIFOLD_ROOT", temp);
        }
        let gates_dir = temp.join("assets").join("gates");
        std::fs::create_dir_all(&gates_dir).expect("create gates dir");
        (previous, gates_dir)
    }

    fn restore_maenifold_root(previous: Option<OsString>) {
        match previous {
            Some(value) => unsafe {
                std::env::set_var("MAENIFOLD_ROOT", value);
            },
            None => unsafe {
                std::env::remove_var("MAENIFOLD_ROOT");
            },
        }
    }

    #[test]
    fn builtin_assets_refresh_reserved_runtime_copies_when_bundled_content_changes() {
        let _guard = super::maenifold_root_lock()
            .lock()
            .expect("lock env guard");
        let temp = TempDir::new().expect("create temp dir");
        let (previous, gates_dir) = set_maenifold_root(temp.path());

        let sdd_path = gates_dir.join("sdd.json");
        let stale_sdd = r#"{
  "schemaVersion": 1,
  "id": "sdd",
  "name": "Stale Runtime SDD",
  "initial": "pending",
  "states": {
    "pending": { "on": { "pass": "done" } },
    "done": { "terminal": true }
  }
}"#;
        std::fs::write(&sdd_path, stale_sdd).expect("write stale runtime asset");

        let definition = super::load_builtin_inner("sdd").expect("load builtin");
        let preserved = std::fs::read_to_string(&sdd_path).expect("read runtime asset");

        restore_maenifold_root(previous);

        assert_eq!(
            definition.ordered_stage_ids().expect("stage ids"),
            vec!["pending", "swe", "red-team", "blue-team", "done"]
        );
        assert_eq!(preserved, SDD_JSON);
        assert!(gates_dir.join("tdd.json").exists());
        assert!(gates_dir.join("gsd.json").exists());
        assert!(gates_dir.join("lightweight.json").exists());
    }

    #[test]
    fn builtin_bootstrap_does_not_overwrite_non_reserved_runtime_definitions() {
        let _guard = super::maenifold_root_lock()
            .lock()
            .expect("lock env guard");
        let temp = TempDir::new().expect("create temp dir");
        let (previous, gates_dir) = set_maenifold_root(temp.path());

        let custom_path = gates_dir.join("custom.json");
        let custom_definition = r#"{
  "schemaVersion": 1,
  "id": "custom",
  "name": "Custom Runtime Gate",
  "initial": "pending",
  "states": {
    "pending": { "on": { "pass": "done" } },
    "done": { "terminal": true }
  }
}"#;
        std::fs::write(&custom_path, custom_definition).expect("write custom runtime definition");

        super::load_builtin_inner("tdd").expect("load builtin");
        let preserved =
            std::fs::read_to_string(&custom_path).expect("read custom runtime definition");

        restore_maenifold_root(previous);

        assert_eq!(preserved, custom_definition);
    }

    #[test]
    fn loads_custom_definition_by_name_from_gates_dir() {
        let _guard = super::maenifold_root_lock()
            .lock()
            .expect("lock env guard");
        let temp = TempDir::new().expect("create temp dir");
        let (previous, gates_dir) = set_maenifold_root(temp.path());

        let dkd = r#"{
  "schemaVersion": 1,
  "id": "dkd",
  "name": "Dunning Kruger Development",
  "initial": "pending",
  "states": {
    "pending": { "on": { "pass": "swe-plan" } },
    "swe-plan": { "on": { "pass": "done" } },
    "done": { "terminal": true }
  }
}"#;
        std::fs::write(gates_dir.join("dkd.json"), dkd).expect("write dkd");

        let definition = super::load_builtin_inner("dkd").expect("load dkd");

        restore_maenifold_root(previous);

        assert_eq!(
            definition.ordered_stage_ids().expect("stage ids"),
            vec!["pending", "swe-plan", "done"]
        );
    }

    #[test]
    fn gates_dir_uses_maenifold_root() {
        let _guard = super::maenifold_root_lock()
            .lock()
            .expect("lock env guard");
        let temp = TempDir::new().expect("create temp dir");
        let (previous, _gates_dir) = set_maenifold_root(temp.path());

        let dir = GateDefinitionCatalog::gates_dir();

        restore_maenifold_root(previous);

        assert_eq!(dir, temp.path().join("assets").join("gates"));
    }
}
