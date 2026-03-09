use std::env;
use std::io;
use std::path::PathBuf;
use std::process::Command;

use crate::compat_env::IsolatedRoot;

const DEFAULT_CSHARP_PROJECT: &str =
    "/Users/brett/src/ma-collective/maenifold/src/Maenifold.csproj";

#[derive(Debug)]
pub struct CliRun {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl CliRun {
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

pub fn run_csharp_cli(args: &[&str], sandbox: &IsolatedRoot) -> io::Result<CliRun> {
    let project = env::var_os("MAENIFOLD_CSHARP_PROJECT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CSHARP_PROJECT));

    let mut command = Command::new("dotnet");
    command.arg("run").arg("--project").arg(project).arg("--");
    command.args(args);
    sandbox.apply_to_command(&mut command);
    capture(command)
}

pub fn run_rust_cli(args: &[&str], sandbox: &IsolatedRoot) -> io::Result<CliRun> {
    if let Some(program) = env::var_os("MAENIFOLD_RUST_CLI")
        .or_else(|| env::var_os("CARGO_BIN_EXE_maenifold-cli"))
        .or_else(|| env::var_os("CARGO_BIN_EXE_maenifold"))
        .map(PathBuf::from)
    {
        let mut command = Command::new(program);
        command.args(args);
        sandbox.apply_to_command(&mut command);
        return capture(command);
    }

    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut command = Command::new("cargo");
    command
        .current_dir(&workspace_root)
        .arg("run")
        .arg("-q")
        .arg("-p")
        .arg("maenifold-cli")
        .arg("--");
    command.args(args);
    sandbox.apply_to_command(&mut command);
    capture(command)
}

fn capture(mut command: Command) -> io::Result<CliRun> {
    let output = command.output()?;
    Ok(CliRun {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    })
}
