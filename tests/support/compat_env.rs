use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub struct IsolatedRoot {
    root_dir: PathBuf,
    database_path: PathBuf,
}

impl IsolatedRoot {
    pub fn new(label: &str) -> io::Result<Self> {
        let nonce = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let safe_label = label
            .chars()
            .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
            .collect::<String>();
        let root_dir = env::temp_dir().join("maenifold-rs-compat").join(format!(
            "{safe_label}-{timestamp}-{nonce}-{}",
            std::process::id()
        ));
        let database_path = root_dir.join("memory.db");

        fs::create_dir_all(root_dir.join("memory"))?;
        fs::create_dir_all(root_dir.join("assets"))?;

        Ok(Self {
            root_dir,
            database_path,
        })
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    pub fn apply_to_command<'a>(&self, command: &'a mut Command) -> &'a mut Command {
        command
            .env("MAENIFOLD_ROOT", &self.root_dir)
            .env("MAENIFOLD_DATABASE_PATH", &self.database_path)
    }
}

impl Drop for IsolatedRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root_dir);
    }
}

#[cfg(test)]
mod tests {
    use super::IsolatedRoot;

    #[test]
    fn creates_isolated_layout() {
        let sandbox = IsolatedRoot::new("layout").expect("sandbox");
        assert!(sandbox.root_dir().exists());
        assert!(sandbox.root_dir().join("memory").exists());
        assert!(sandbox.root_dir().join("assets").exists());
        assert_eq!(
            sandbox.database_path(),
            sandbox.root_dir().join("memory.db")
        );
    }
}
