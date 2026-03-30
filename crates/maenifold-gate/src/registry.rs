use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Registry {
    pub gates: Vec<GateEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateEntry {
    pub root: String,
    pub name: String,
    pub registered_at: String,
}

pub fn registry_path() -> PathBuf {
    ProjectDirs::from("com", "ma-collective", "gate")
        .map(|dirs| dirs.data_dir().join("registry.json"))
        .unwrap_or_else(|| PathBuf::from("registry.json"))
}

pub fn load() -> Result<Registry> {
    load_from_path(&registry_path())
}

pub fn save(registry: &Registry) -> Result<()> {
    save_to_path(&registry_path(), registry)
}

pub fn auto_register(root: &Path) -> Result<()> {
    auto_register_at(&registry_path(), root)
}

fn load_from_path(path: &Path) -> Result<Registry> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .with_context(|| format!("parsing registry file {}", path.display())),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(Registry::default()),
        Err(error) => Err(error).with_context(|| format!("reading {}", path.display())),
    }
}

fn save_to_path(path: &Path, registry: &Registry) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    let json = serde_json::to_vec_pretty(registry).context("serializing gate registry")?;
    let temp_path = temp_registry_path(path);
    fs::write(&temp_path, json).with_context(|| format!("writing {}", temp_path.display()))?;
    fs::rename(&temp_path, path)
        .with_context(|| format!("renaming {} to {}", temp_path.display(), path.display()))
}

fn auto_register_at(registry_file: &Path, root: &Path) -> Result<()> {
    let root = normalize_root(root)?;
    if is_temp_root(&root) {
        return Ok(());
    }

    let root_value = root.to_string_lossy().to_string();
    let name = root_name(&root);
    let registered_at = format_timestamp(SystemTime::now())?;
    let mut registry = load_from_path(registry_file)?;

    match registry
        .gates
        .iter_mut()
        .find(|entry| entry.root == root_value)
    {
        Some(entry) => {
            entry.name = name;
            entry.registered_at = registered_at;
        }
        None => registry.gates.push(GateEntry {
            root: root_value,
            name,
            registered_at,
        }),
    }

    registry
        .gates
        .sort_by(|left, right| left.root.cmp(&right.root));
    save_to_path(registry_file, &registry)
}

fn root_name(root: &Path) -> String {
    root.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| root.as_os_str().to_string_lossy().to_string())
}

fn temp_registry_path(path: &Path) -> PathBuf {
    path.file_name()
        .map(|name| path.with_file_name(format!("{}.tmp", name.to_string_lossy())))
        .unwrap_or_else(|| path.with_extension("tmp"))
}

fn normalize_root(root: &Path) -> Result<PathBuf> {
    match fs::canonicalize(root) {
        Ok(path) => Ok(path),
        Err(_) if root.is_absolute() => Ok(root.to_path_buf()),
        Err(_) => Ok(std::env::current_dir()
            .context("reading current working directory for relative gate root")?
            .join(root)),
    }
}

fn is_temp_root(root: &Path) -> bool {
    root.starts_with(Path::new("/tmp"))
        || root.starts_with(Path::new("/var/folders"))
        || root.starts_with(Path::new("/private/tmp"))
        || root.starts_with(Path::new("/private/var/folders"))
        || std::env::var_os("TMPDIR")
            .is_some_and(|tmp_dir| !tmp_dir.is_empty() && root.starts_with(Path::new(&tmp_dir)))
}

fn format_timestamp(time: SystemTime) -> Result<String> {
    let seconds = time
        .duration_since(UNIX_EPOCH)
        .context("system clock is before UNIX_EPOCH")?
        .as_secs() as i64;
    let days = seconds.div_euclid(86_400);
    let second_of_day = seconds.rem_euclid(86_400);
    let hour = second_of_day / 3_600;
    let minute = (second_of_day % 3_600) / 60;
    let second = second_of_day % 60;
    let (year, month, day) = civil_from_days(days);
    Ok(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z"
    ))
}

fn civil_from_days(days_since_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if month <= 2 { 1 } else { 0 };
    (year as i32, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_returns_empty_registry_when_file_is_missing() -> Result<()> {
        let dir = tempdir()?;
        let registry = load_from_path(&dir.path().join("registry.json"))?;
        assert_eq!(registry, Registry::default());
        Ok(())
    }

    #[test]
    fn auto_register_is_idempotent_by_root() -> Result<()> {
        let dir = tempdir()?;
        let registry_file = dir.path().join("registry.json");
        let root = std::env::current_dir()?
            .join("gate-registry-tests")
            .join("project-a");

        auto_register_at(&registry_file, &root)?;
        auto_register_at(&registry_file, &root)?;

        let registry = load_from_path(&registry_file)?;
        assert_eq!(registry.gates.len(), 1);
        assert_eq!(registry.gates[0].root, root.to_string_lossy());
        assert_eq!(registry.gates[0].name, "project-a");
        assert!(!registry.gates[0].registered_at.is_empty());
        Ok(())
    }

    #[test]
    fn auto_register_skips_temp_roots() -> Result<()> {
        let dir = tempdir()?;
        let registry_file = dir.path().join("registry.json");
        let temp_root = dir.path().join("project-a");

        auto_register_at(&registry_file, &temp_root)?;

        let registry = load_from_path(&registry_file)?;
        assert!(registry.gates.is_empty());
        Ok(())
    }

    #[test]
    fn save_uses_atomic_rename_path() -> Result<()> {
        let dir = tempdir()?;
        let registry_file = dir.path().join("registry.json");
        let registry = Registry {
            gates: vec![GateEntry {
                root: "/tmp/example".into(),
                name: "example".into(),
                registered_at: "1970-01-01T00:00:00Z".into(),
            }],
        };

        save_to_path(&registry_file, &registry)?;

        assert_eq!(load_from_path(&registry_file)?, registry);
        assert!(!temp_registry_path(&registry_file).exists());
        Ok(())
    }

    #[test]
    fn format_timestamp_outputs_iso_8601_utc() -> Result<()> {
        let time = UNIX_EPOCH + std::time::Duration::from_secs(90_123);
        assert_eq!(format_timestamp(time)?, "1970-01-02T01:02:03Z");
        Ok(())
    }
}
