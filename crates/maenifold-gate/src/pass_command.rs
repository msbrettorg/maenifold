use crate::cli::OutcomeArgs;
use crate::error::GateError;
use crate::gate_catalog::GateDefinitionCatalog;
use crate::gate_tool_service::GateToolService;
use crate::section_factory::CanonicalSectionFactory;
use crate::state_machine::StateMachine;
use crate::task_file::TaskFileLock;
use crate::task_repository::TaskRepository;
use crate::task_status::TaskStatusDeriver;

#[derive(Debug, Clone)]
pub struct PassCommand;

impl PassCommand {
    pub fn execute(args: OutcomeArgs) -> Result<(), GateError> {
        let _ = GateToolService::pass(args)?;
        Ok(())
    }
}

pub(crate) fn pass_task(args: OutcomeArgs) -> Result<(), GateError> {
    apply_outcome(args, "pass")
}

pub(crate) fn fail_task(args: OutcomeArgs) -> Result<(), GateError> {
    apply_outcome(args, "fail")
}

fn apply_outcome(args: OutcomeArgs, outcome: &str) -> Result<(), GateError> {
    let _task_lock = TaskFileLock::acquire(&args.context.root, &args.task_id)?;
    let task = TaskRepository::find_task(args.context.root.clone(), &args.task_id)?;
    let document = TaskRepository::read_task_document(&args.context.root, &task)?;
    let definition = GateDefinitionCatalog::resolve_for_task(
        &args.context.root,
        &document.metadata,
        args.context.requested_pipeline_name.as_deref(),
    )?;
    let state = TaskStatusDeriver::derive(&document, &definition, &task.location)?;
    let current_state = definition
        .find_state(&state.state)
        .ok_or_else(|| GateError::UnknownState(state.state.clone()))?;

    if current_state.terminal {
        return match outcome {
            "fail" => reopen_completed_task(args, task, &state.state),
            "pass" => Err(GateError::Cli(format!(
                "task '{}' is already done; use `gate fail {}` to reopen it",
                args.task_id, args.task_id
            ))),
            _ => unreachable!(),
        };
    }

    let transition = match outcome {
        "pass" => StateMachine::next_on_pass(&state.state, &definition)?,
        "fail" => StateMachine::next_on_fail(&state.state, &definition)?,
        _ => unreachable!(),
    };

    let section = CanonicalSectionFactory::build_outcome_section(
        &transition.acting_stage,
        outcome,
        &args.agent,
        &args.notes,
        args.model.as_deref(),
    );
    TaskRepository::append_task_section(&args.context.root, &task, &section)?;
    if definition
        .find_state(&transition.next_state)
        .ok_or_else(|| GateError::UnknownState(transition.next_state.clone()))?
        .terminal
    {
        TaskRepository::move_to_done(&args.context.root, &task)?;
    }
    Ok(())
}

fn reopen_completed_task(
    args: OutcomeArgs,
    task: crate::task_repository::TaskPathSet,
    terminal_state: &str,
) -> Result<(), GateError> {
    let section = CanonicalSectionFactory::build_outcome_section(
        terminal_state,
        "fail",
        &args.agent,
        &args.notes,
        args.model.as_deref(),
    );
    TaskRepository::append_task_section(&args.context.root, &task, &section)?;
    TaskRepository::move_to_todo(&args.context.root, &task)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs as unix_fs;

    use tempfile::TempDir;

    use super::pass_task;
    use crate::cli::{GateContext, OutcomeArgs};

    #[cfg(unix)]
    #[test]
    fn pass_rejects_symlinked_task_path() {
        let root = new_pipeline_root();
        let target = root.path().join("todo").join("pass-target.md");
        fs::write(
            &target,
            "# T-90.19 — Pass target\n\n**Pipeline**: sdd\n\n## Acceptance Criteria\n\n1. pass should not touch symlinks\n",
        )
        .expect("write pass target");
        let task_path = root.path().join("todo").join("T-90.19.md");
        unix_fs::symlink(&target, &task_path).expect("create symlinked task");

        let error = pass_task(OutcomeArgs {
            context: GateContext {
                root: root.path().to_path_buf(),
                requested_pipeline_name: None,
                json_output: false,
            },
            task_id: "T-90.19".to_string(),
            agent: "blue-team".to_string(),
            notes: "approved".to_string(),
            model: None,
        })
        .expect_err("symlinked pass should fail");

        assert!(error.to_string().contains("through a symlink"));
        let unchanged = fs::read_to_string(&target).expect("read pass target");
        assert!(!unchanged.contains("## Pending"));
    }

    fn new_pipeline_root() -> TempDir {
        let temp = tempfile::tempdir().expect("create temp root");
        fs::create_dir_all(temp.path().join("todo")).expect("create todo");
        fs::create_dir_all(temp.path().join("done")).expect("create done");
        temp
    }
}
