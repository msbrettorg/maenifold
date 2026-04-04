use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::GateError;
use crate::gate_definition::GateDefinition;
use crate::task_metadata::TaskMetadata;

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
        let text = fs::read_to_string(&asset_path)?;
        GateDefinition::from_json_text(&text)
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
        if let Some(requested) = requested_pipeline_name {
            if requested != metadata.pipeline {
                return Err(GateError::PipelineOverrideMismatch {
                    requested: requested.to_string(),
                    bound: metadata.pipeline.clone(),
                });
            }
        }

        Self::resolve(root, &metadata.pipeline)
    }

    fn runtime_gates_dir() -> PathBuf {
        env::var_os(GATES_DIR_ENV)
            .map(PathBuf::from)
            .unwrap_or_else(default_runtime_gates_dir)
    }

    fn ensure_all_builtin_assets() -> Result<(), GateError> {
        let dir = Self::runtime_gates_dir();
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

fn default_runtime_gates_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        home_dir_from_env()
            .unwrap_or_else(fallback_runtime_base_dir)
            .join("Library")
            .join("Application Support")
            .join("maenifold")
            .join("gates")
    }

    #[cfg(target_os = "windows")]
    {
        env::var_os("APPDATA")
            .map(PathBuf::from)
            .or_else(|| home_dir_from_env().map(|home| home.join("AppData").join("Roaming")))
            .unwrap_or_else(fallback_runtime_base_dir)
            .join("maenifold")
            .join("gates")
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| home_dir_from_env().map(|home| home.join(".config")))
            .unwrap_or_else(fallback_runtime_base_dir)
            .join("maenifold")
            .join("gates")
    }
}

fn home_dir_from_env() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}

fn fallback_runtime_base_dir() -> PathBuf {
    env::temp_dir().join("maenifold")
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

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::sync::{LazyLock, Mutex};

    use tempfile::TempDir;

    use super::{GATES_DIR_ENV, GateDefinitionCatalog, SDD_JSON, default_runtime_gates_dir};

    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    #[test]
    fn builtin_assets_refresh_reserved_runtime_copies_when_bundled_content_changes() {
        let _guard = ENV_LOCK.lock().expect("lock env guard");
        let temp = TempDir::new().expect("create temp dir");
        let gates_dir = temp.path().join("gates");
        std::fs::create_dir_all(&gates_dir).expect("create gates dir");

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

        let previous = std::env::var_os(GATES_DIR_ENV);
        unsafe {
            std::env::set_var(GATES_DIR_ENV, &gates_dir);
        }

        let definition = GateDefinitionCatalog::load_builtin("sdd").expect("load builtin");
        let preserved = std::fs::read_to_string(&sdd_path).expect("read runtime asset");

        restore_env(previous);

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
        let _guard = ENV_LOCK.lock().expect("lock env guard");
        let temp = TempDir::new().expect("create temp dir");
        let gates_dir = temp.path().join("gates");
        std::fs::create_dir_all(&gates_dir).expect("create gates dir");

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

        let previous = std::env::var_os(GATES_DIR_ENV);
        unsafe {
            std::env::set_var(GATES_DIR_ENV, &gates_dir);
        }

        GateDefinitionCatalog::load_builtin("tdd").expect("load builtin");
        let preserved =
            std::fs::read_to_string(&custom_path).expect("read custom runtime definition");

        restore_env(previous);

        assert_eq!(preserved, custom_definition);
    }

    #[test]
    fn default_runtime_gates_dir_uses_platform_config_location() {
        let _guard = ENV_LOCK.lock().expect("lock env guard");
        let home = TempDir::new().expect("create temp home");
        let previous_home = std::env::var_os("HOME");
        let previous_gate_dir = std::env::var_os(GATES_DIR_ENV);
        let previous_xdg = std::env::var_os("XDG_CONFIG_HOME");
        #[cfg(target_os = "windows")]
        let previous_appdata = std::env::var_os("APPDATA");

        unsafe {
            std::env::remove_var(GATES_DIR_ENV);
            std::env::set_var("HOME", home.path());
            std::env::remove_var("XDG_CONFIG_HOME");
            #[cfg(target_os = "windows")]
            std::env::remove_var("APPDATA");
        }

        let runtime_dir = default_runtime_gates_dir();

        restore_env_var("HOME", previous_home);
        restore_env_var(GATES_DIR_ENV, previous_gate_dir);
        restore_env_var("XDG_CONFIG_HOME", previous_xdg);
        #[cfg(target_os = "windows")]
        restore_env_var("APPDATA", previous_appdata);

        #[cfg(target_os = "macos")]
        assert_eq!(
            runtime_dir,
            home.path()
                .join("Library")
                .join("Application Support")
                .join("maenifold")
                .join("gates")
        );

        #[cfg(target_os = "windows")]
        assert_eq!(
            runtime_dir,
            home.path()
                .join("AppData")
                .join("Roaming")
                .join("maenifold")
                .join("gates")
        );

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        assert_eq!(
            runtime_dir,
            home.path().join(".config").join("maenifold").join("gates")
        );
    }

    fn restore_env(previous: Option<OsString>) {
        restore_env_var(GATES_DIR_ENV, previous);
    }

    fn restore_env_var(name: &str, previous: Option<OsString>) {
        match previous {
            Some(value) => unsafe {
                std::env::set_var(name, value);
            },
            None => unsafe {
                std::env::remove_var(name);
            },
        }
    }
}
