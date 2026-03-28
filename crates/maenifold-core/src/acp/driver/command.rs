use std::path::Path;
use std::process::Stdio;

use tokio::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CommandSpec {
    pub(super) args: Vec<String>,
}

impl CommandSpec {
    pub(super) fn shell_string(&self, binary: &Path) -> String {
        let mut parts = vec![binary.display().to_string()];
        parts.extend(self.args.iter().cloned());
        parts.join(" ")
    }
}

pub(super) fn create_session_spec(agent: &str, name: &str) -> CommandSpec {
    CommandSpec {
        args: vec![
            agent.to_string(),
            "sessions".to_string(),
            "new".to_string(),
            "--name".to_string(),
            name.to_string(),
        ],
    }
}

pub(super) fn close_session_spec(agent: &str, name: &str) -> CommandSpec {
    CommandSpec {
        args: vec![
            agent.to_string(),
            "sessions".to_string(),
            "close".to_string(),
            name.to_string(),
        ],
    }
}

pub(super) fn prompt_spec(agent: &str, timeout: u64, session: &str, prompt: &str) -> CommandSpec {
    let mut args = vec!["--format".to_string(), "json".to_string()];
    if timeout > 0 {
        args.push("--timeout".to_string());
        args.push(timeout.to_string());
    }
    args.extend([
        "--approve-all".to_string(),
        agent.to_string(),
        "-s".to_string(),
        session.to_string(),
        prompt.to_string(),
    ]);
    CommandSpec { args }
}

pub(super) fn cancel_spec(agent: &str, session: &str) -> CommandSpec {
    CommandSpec {
        args: vec![
            agent.to_string(),
            "cancel".to_string(),
            "-s".to_string(),
            session.to_string(),
        ],
    }
}

pub(super) fn build_command(acpx_path: &Path, cwd: &Path, spec: &CommandSpec) -> Command {
    let mut command = Command::new(acpx_path);
    command
        .args(&spec.args)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    command
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::acpx_driver::AcpxDriver;

    use super::{CommandSpec, cancel_spec, close_session_spec, create_session_spec, prompt_spec};

    fn test_driver() -> AcpxDriver {
        AcpxDriver::new_for_tests(
            "codex".to_string(),
            0,
            PathBuf::from("/tmp/worktree"),
            PathBuf::from("/usr/local/bin/acpx"),
        )
    }

    #[test]
    fn acpx_driver_builds_expected_prompt_command_args() {
        let driver = test_driver();
        let spec = prompt_spec(
            &driver.agent,
            driver.timeout,
            "workflow-42",
            "review changes",
        );

        assert_eq!(
            spec.args,
            vec![
                "--format",
                "json",
                "--approve-all",
                "codex",
                "-s",
                "workflow-42",
                "review changes",
            ]
        );
    }

    #[test]
    fn acpx_driver_builds_expected_session_and_cancel_commands() {
        let driver = test_driver();

        assert_eq!(
            create_session_spec(&driver.agent, "workflow-42").args,
            vec!["codex", "sessions", "new", "--name", "workflow-42"]
        );
        assert_eq!(
            close_session_spec(&driver.agent, "workflow-42").args,
            vec!["codex", "sessions", "close", "workflow-42"]
        );
        assert_eq!(
            cancel_spec(&driver.agent, "workflow-42").args,
            vec!["codex", "cancel", "-s", "workflow-42"]
        );
    }

    #[test]
    fn command_spec_renders_shell_string() {
        let spec = CommandSpec {
            args: vec!["codex".to_string(), "cancel".to_string()],
        };

        assert_eq!(
            spec.shell_string(Path::new("/usr/local/bin/acpx")),
            "/usr/local/bin/acpx codex cancel"
        );
    }
}
