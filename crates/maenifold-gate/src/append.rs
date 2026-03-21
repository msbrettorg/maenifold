use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Result, anyhow, bail};

use crate::db::GateDb;

pub struct AppendArgs {
    pub id: String,
    pub notes: String,
    pub agent: String,
    pub db_path: PathBuf,
    pub root: PathBuf,
}

pub fn run(args: &AppendArgs) -> Result<()> {
    let db = GateDb::open(&args.db_path)?;
    let file = db
        .resolve_id_to_file(&args.id)?
        .ok_or_else(|| anyhow!("unknown task ID: {}", args.id))?;
    let path = resolve_task_path(&args.root, &file)?;
    let mut file = OpenOptions::new().append(true).open(&path)?;
    write!(
        file,
        "\n## {} Notes\n\n{}\n",
        capitalize_agent(&args.agent),
        args.notes
    )?;
    eprintln!("{}: appended {} notes", args.id, args.agent);
    Ok(())
}

fn capitalize_agent(agent: &str) -> String {
    agent.chars().next().map_or_else(String::new, |c| {
        c.to_uppercase().chain(agent.chars().skip(1)).collect()
    })
}

fn resolve_task_path(root: &Path, file: &str) -> Result<PathBuf> {
    for path in [root.join("todo").join(file), root.join("done").join(file)] {
        if path.is_file() {
            return Ok(path);
        }
    }
    bail!(
        "task file {file} not found under {}/todo or {}/done",
        root.display(),
        root.display()
    )
}

#[cfg(test)]
#[path = "append_tests.rs"]
mod tests;
