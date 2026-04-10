use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
};
use uuid::Uuid;

#[tauri::command]
pub fn save_workflow_asset(path: String, json: String) -> Result<String, String> {
    let root = workflow_assets_root()?;
    fs::create_dir_all(&root).map_err(|error| error.to_string())?;

    let target = PathBuf::from(path);
    validate_target_path(&target, &root)?;
    write_atomically(&target, &json)?;

    Ok(target.to_string_lossy().to_string())
}

fn workflow_assets_root() -> Result<PathBuf, String> {
    Ok(env::var_os("MAENIFOLD_ROOT")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join("maenifold")))
        .ok_or_else(|| "Could not find maenifold root".to_string())?
        .join("assets")
        .join("workflows"))
}

fn validate_target_path(target: &Path, root: &Path) -> Result<(), String> {
    if !target.is_absolute() {
        return Err("Workflow save path must be absolute".to_string());
    }

    if target.parent() != Some(root) {
        return Err(format!(
            "Workflow save path must live under {}",
            root.display()
        ));
    }

    Ok(())
}

fn write_atomically(path: &Path, json: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("Workflow save path has no parent: {}", path.display()))?;

    for _ in 0..8 {
        let temp_path = create_sidecar_path(path, "workflow-tmp");
        let backup_path = create_sidecar_path(path, "workflow-prev");
        let result = (|| -> std::io::Result<()> {
            let mut file = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temp_path)?;
            file.write_all(json.as_bytes())?;
            file.sync_all()?;
            drop(file);

            let existing_moved = if path.exists() {
                fs::rename(path, &backup_path)?;
                true
            } else {
                false
            };

            if let Err(error) = fs::rename(&temp_path, path) {
                if existing_moved {
                    let _ = fs::rename(&backup_path, path);
                    sync_directory(parent);
                }
                return Err(error);
            }

            sync_directory(parent);

            if existing_moved {
                fs::remove_file(&backup_path)?;
                sync_directory(parent);
            }

            Ok(())
        })();

        let _ = fs::remove_file(&temp_path);
        let _ = fs::remove_file(&backup_path);

        match result {
            Ok(()) => return Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.to_string()),
        }
    }

    Err(format!(
        "failed to allocate atomic temp file for {}",
        path.display()
    ))
}

fn create_sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("workflow.json");

    path.parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!(".{file_name}.{suffix}-{}", Uuid::new_v4()))
}

fn sync_directory(path: &Path) {
    let _ = fs::File::open(path).and_then(|file| file.sync_all());
}
