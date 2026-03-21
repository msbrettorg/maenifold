use crate::types::TaskLocation;
use anyhow::{Context, Result, bail};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredTask {
    pub path: PathBuf,
    pub location: TaskLocation,
}

fn is_task(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with("T-") && n.ends_with(".md"))
}

pub fn discover_tasks(root: &Path) -> Result<Vec<DiscoveredTask>> {
    let mut found = Vec::new();
    for (dir, location) in [("todo", TaskLocation::Todo), ("done", TaskLocation::Done)] {
        let base = root.join(dir);
        if !base.is_dir() {
            continue;
        }
        for entry in walkdir::WalkDir::new(&base) {
            let path = entry
                .with_context(|| format!("walking {}", base.display()))?
                .into_path();
            if path.is_file() && is_task(&path) {
                found.push(DiscoveredTask { path, location });
            }
        }
    }
    if found.is_empty() && !root.join("todo").is_dir() && !root.join("done").is_dir() {
        bail!(
            "expected {}/todo or {}/done to exist",
            root.display(),
            root.display()
        );
    }
    Ok(found)
}

pub fn read_task(path: &Path) -> Result<String> {
    String::from_utf8(
        fs::read(path).with_context(|| format!("reading task file {}", path.display()))?,
    )
    .with_context(|| format!("task file {} is not valid UTF-8", path.display()))
}

pub fn move_task(from: &Path, to_dir: &Path) -> Result<PathBuf> {
    let name = from
        .file_name()
        .context("task path is missing a filename")?;
    fs::create_dir_all(to_dir).with_context(|| format!("creating {}", to_dir.display()))?;
    let to = to_dir.join(name);
    if to.exists() {
        bail!("destination task already exists: {}", to.display());
    }
    fs::rename(from, &to)
        .with_context(|| format!("moving {} to {}", from.display(), to.display()))?;
    Ok(to)
}

pub fn move_to_done(root: &Path, file: &str) -> Result<PathBuf> {
    let (from, location) = resolve_task_path(root, file)?;
    if location == TaskLocation::Done {
        bail!("task file {file} is already under {}/done", root.display());
    }
    move_task(&from, &root.join("done"))
}

pub fn resolve_file_path(root: &Path, file: &str) -> Result<PathBuf> {
    Ok(resolve_task_path(root, file)?.0)
}

pub fn resolve_task_path(root: &Path, file: &str) -> Result<(PathBuf, TaskLocation)> {
    for (path, location) in [
        (root.join("todo").join(file), TaskLocation::Todo),
        (root.join("done").join(file), TaskLocation::Done),
    ] {
        if path.is_file() {
            return Ok((path, location));
        }
    }
    bail!(
        "task file {file} not found under {}/todo or {}/done",
        root.display(),
        root.display()
    )
}

pub fn append_orchestrator_section(path: &Path, verdict: &str, notes: Option<&str>) -> Result<()> {
    let mut file = OpenOptions::new()
        .append(true)
        .open(path)
        .with_context(|| format!("opening task file {}", path.display()))?;
    write!(
        file,
        "\n## Orchestrator — {verdict}\n\n{}\n",
        notes.unwrap_or("")
    )
    .with_context(|| format!("writing task file {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn discover_tasks_finds_matching_files() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path();
        fs::create_dir_all(root.join("todo/sub"))?;
        fs::create_dir_all(root.join("done"))?;
        for path in [
            root.join("todo/T-1.md"),
            root.join("todo/sub/T-2.md"),
            root.join("done/T-3.md"),
            root.join("todo/note.md"),
            root.join("done/T-4.txt"),
        ] {
            fs::write(path, "# x")?;
        }
        let mut got = discover_tasks(root)?;
        got.sort_by(|a, b| a.path.cmp(&b.path));
        assert_eq!(
            got.iter()
                .map(|t| (&t.path, t.location))
                .collect::<Vec<_>>(),
            vec![
                (&root.join("done/T-3.md"), TaskLocation::Done),
                (&root.join("todo/T-1.md"), TaskLocation::Todo),
                (&root.join("todo/sub/T-2.md"), TaskLocation::Todo)
            ]
        );
        Ok(())
    }

    #[test]
    fn move_task_moves_and_returns_new_path() -> Result<()> {
        let dir = tempdir()?;
        let from = dir.path().join("todo/T-1.md");
        fs::create_dir_all(from.parent().unwrap())?;
        fs::write(&from, "# x")?;
        let to = move_task(&from, &dir.path().join("done"))?;
        assert_eq!(to, dir.path().join("done/T-1.md"));
        assert!(!from.exists() && to.exists());
        Ok(())
    }

    #[test]
    fn move_task_errors_when_destination_exists() -> Result<()> {
        let dir = tempdir()?;
        let from = dir.path().join("todo/T-1.md");
        let done = dir.path().join("done");
        fs::create_dir_all(from.parent().unwrap())?;
        fs::create_dir_all(&done)?;
        fs::write(&from, "# x")?;
        fs::write(done.join("T-1.md"), "# y")?;
        assert!(move_task(&from, &done).is_err());
        Ok(())
    }

    #[test]
    fn resolve_task_path_checks_both_locations() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path();
        fs::create_dir_all(root.join("todo"))?;
        fs::create_dir_all(root.join("done"))?;
        fs::write(root.join("todo/T-1.md"), "# x")?;
        fs::write(root.join("done/T-2.md"), "# y")?;
        assert_eq!(
            resolve_task_path(root, "T-1.md")?,
            (root.join("todo/T-1.md"), TaskLocation::Todo)
        );
        assert_eq!(
            resolve_task_path(root, "T-2.md")?,
            (root.join("done/T-2.md"), TaskLocation::Done)
        );
        assert!(resolve_task_path(root, "T-3.md").is_err());
        Ok(())
    }
}
