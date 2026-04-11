use crate::cli::AppendArgs;
use crate::error::GateError;
use crate::gate_catalog::GateDefinitionCatalog;
use crate::gate_tool_service::GateToolService;
use crate::section_factory::CanonicalSectionFactory;
use crate::state_machine::StateMachine;
use crate::task_file::TaskFileLock;
use crate::task_repository::TaskRepository;
use crate::task_status::TaskStatusDeriver;

#[derive(Debug, Clone)]
pub struct AppendCommand;

impl AppendCommand {
    pub fn execute(args: AppendArgs) -> Result<(), GateError> {
        let _ = GateToolService::append(args)?;
        Ok(())
    }
}

pub(crate) fn append_task(args: AppendArgs) -> Result<(), GateError> {
    let _task_lock = TaskFileLock::acquire(&args.context.root, &args.task_id)?;
    let task = TaskRepository::find_task(args.context.root.clone(), &args.task_id)?;
    let document = TaskRepository::read_task_document(&args.context.root, &task)?;
    let definition = GateDefinitionCatalog::resolve_for_task(
        &args.context.root,
        &document.metadata,
        args.context.requested_pipeline_name.as_deref(),
    )?;
    let state = TaskStatusDeriver::derive(&document, &definition, &task.location)?;
    if definition
        .find_state(&state.state)
        .ok_or_else(|| GateError::UnknownState(state.state.clone()))?
        .terminal
    {
        return Err(GateError::Cli(format!(
            "task '{}' is already done; append is not allowed on completed tasks",
            args.task_id
        )));
    }
    let acting_stage = StateMachine::acting_stage_name(&state.state, &definition)?;

    let section = CanonicalSectionFactory::build_append_section(
        &acting_stage,
        &args.agent,
        &args.notes,
        args.model.as_deref(),
    );
    TaskRepository::append_task_section(&args.context.root, &task, &section)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs as unix_fs;

    use tempfile::TempDir;

    use super::append_task;
    use crate::cli::{AppendArgs, GateContext};

    #[test]
    fn pending_append_stamps_pending_stage_id() {
        let root = new_pipeline_root();
        let task_path = root.path().join("todo").join("T-90.17.md");
        fs::write(
            &task_path,
            "# T-90.17 — Pending append\n\n**Pipeline**: sdd\n\n## Acceptance Criteria\n\n1. append uses pending stage\n",
        )
        .expect("write task");

        append_task(AppendArgs {
            context: GateContext {
                root: root.path().to_path_buf(),
                requested_pipeline_name: None,
                json_output: false,
            },
            task_id: "T-90.17".to_string(),
            agent: "swe".to_string(),
            notes: "captured progress".to_string(),
            model: None,
        })
        .expect("append succeeds");

        let updated = fs::read_to_string(&task_path).expect("read task");
        assert!(updated.contains("- Stage ID: pending"));
    }

    #[cfg(unix)]
    #[test]
    fn append_rejects_symlinked_task_path() {
        let root = new_pipeline_root();
        let target = root.path().join("todo").join("append-target.md");
        fs::write(
            &target,
            "# T-90.18 — Append target\n\n**Pipeline**: sdd\n\n## Acceptance Criteria\n\n1. append should not touch symlinks\n",
        )
        .expect("write append target");
        let task_path = root.path().join("todo").join("T-90.18.md");
        unix_fs::symlink(&target, &task_path).expect("create symlinked task");

        let error = append_task(AppendArgs {
            context: GateContext {
                root: root.path().to_path_buf(),
                requested_pipeline_name: None,
                json_output: false,
            },
            task_id: "T-90.18".to_string(),
            agent: "swe".to_string(),
            notes: "captured progress".to_string(),
            model: None,
        })
        .expect_err("symlinked append should fail");

        assert!(error.to_string().contains("through a symlink"));
        let unchanged = fs::read_to_string(&target).expect("read append target");
        assert!(!unchanged.contains("## Pending"));
    }

    fn new_pipeline_root() -> TempDir {
        let temp = tempfile::tempdir().expect("create temp root");
        fs::create_dir_all(temp.path().join("todo")).expect("create todo");
        fs::create_dir_all(temp.path().join("done")).expect("create done");
        temp
    }
}
