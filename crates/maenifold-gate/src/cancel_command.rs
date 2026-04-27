use crate::cli::CancelArgs;
use crate::error::GateError;
use crate::gate_catalog::GateDefinitionCatalog;
use crate::gate_tool_service::GateToolService;
use crate::section_factory::CanonicalSectionFactory;
use crate::task_file::TaskFileLock;
use crate::task_repository::TaskRepository;
use crate::task_status::TaskStatusDeriver;

#[derive(Debug, Clone)]
pub struct CancelCommand;

impl CancelCommand {
    pub fn execute(args: CancelArgs) -> Result<(), GateError> {
        let _ = GateToolService::cancel(args)?;
        Ok(())
    }
}

pub(crate) fn cancel_task(args: CancelArgs) -> Result<(), GateError> {
    let _task_lock = TaskFileLock::acquire(&args.context.root, &args.task_id)?;
    let task = TaskRepository::find_task(args.context.root.clone(), &args.task_id)?;
    let document = TaskRepository::read_task_document(&args.context.root, &task)?;
    let definition = GateDefinitionCatalog::resolve_for_task(
        &args.context.root,
        &document.metadata,
        args.context.requested_pipeline_name.as_deref(),
    )?;
    let state = TaskStatusDeriver::derive(&document, &definition, &task.location)?;

    if state.state == "cancelled" {
        return Err(GateError::Cli(format!(
            "task '{}' is already cancelled",
            args.task_id
        )));
    }

    if let Some(current) = definition.find_state(&state.state)
        && current.terminal
    {
        return Err(GateError::Cli(format!(
            "task '{}' is already done; cancel is not allowed on completed tasks. To record follow-up work, create a new successor task file in todo/ referencing this task, then run gate_Sync to register it.",
            args.task_id
        )));
    }

    let section = CanonicalSectionFactory::build_cancel_section(
        &args.agent,
        &args.notes,
        args.model.as_deref(),
    );
    // Move the file to done/ first, then append the cancel section in its new
    // location. If the append fails, roll the move back so we never leave the
    // repo in a half-cancelled state (which would poison `gate sync` either
    // way: a cancelled task stranded in todo/, or a non-cancelled file stuck
    // in done/ that TaskStatusDeriver rejects because its derived state is
    // non-terminal).
    TaskRepository::move_to_done(&args.context.root, &task)?;
    let moved_task = crate::task_repository::TaskPathSet {
        task_id: task.task_id.clone(),
        path: args
            .context
            .root
            .join("done")
            .join(format!("{}.md", task.task_id)),
        location: "done".to_string(),
    };
    if let Err(append_error) =
        TaskRepository::append_task_section(&args.context.root, &moved_task, &section)
    {
        // Best-effort rollback: swing the file back to todo/ so the caller
        // can retry. A rollback failure is logged via stderr but does not
        // shadow the primary append error, which is what the caller needs.
        if let Err(rollback_error) = rollback_move_to_todo(&args.context.root, &moved_task) {
            eprintln!(
                "gate cancel: failed to roll back move of '{}' to todo/ after append failure: {rollback_error}",
                moved_task.task_id
            );
        }
        return Err(append_error);
    }
    Ok(())
}

/// Move a task file from `done/` back to `todo/`. Used by cancel_task to
/// unwind a partially-applied cancel when the section append fails after
/// the file has already been moved.
pub(crate) fn rollback_move_to_todo(
    root: &std::path::Path,
    moved_task: &crate::task_repository::TaskPathSet,
) -> Result<(), GateError> {
    let source = root.join("done").join(format!("{}.md", moved_task.task_id));
    let destination = root.join("todo").join(format!("{}.md", moved_task.task_id));
    std::fs::rename(&source, &destination).map_err(GateError::Io)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs as unix_fs;

    use tempfile::TempDir;

    use super::cancel_task;
    use crate::cli::{CancelArgs, GateContext};

    #[test]
    fn cancel_appends_section_and_moves_task_to_done() {
        let root = new_pipeline_root();
        let task_path = root.path().join("todo").join("T-77.1.md");
        fs::write(
            &task_path,
            "# T-77.1 — Cancel target\n\n**Pipeline**: sdd\n\n## Acceptance Criteria\n\n1. cancel moves task to done\n",
        )
        .expect("write task");

        cancel_task(CancelArgs {
            context: GateContext {
                root: root.path().to_path_buf(),
                requested_pipeline_name: None,
                json_output: false,
            },
            task_id: "T-77.1".to_string(),
            agent: "pm".to_string(),
            notes: "superseded".to_string(),
            model: None,
        })
        .expect("cancel succeeds");

        // Strong post-conditions: the cancel section and the file location
        // must agree. If either of these fails, a half-applied cancel has
        // leaked through (file in todo/ with a cancel section, or file in
        // done/ without one).
        assert!(!task_path.exists(), "task should be moved out of todo/");
        let done_path = root.path().join("done").join("T-77.1.md");
        assert!(done_path.exists(), "task should land in done/");
        let updated = fs::read_to_string(&done_path).expect("read moved task");
        assert!(updated.contains("## Cancelled"));
        assert!(updated.contains("- Stage ID: cancelled"));
        assert!(updated.contains("- Record Kind: cancel"));
        assert!(updated.contains("- Agent: pm"));
        assert!(updated.contains("- Notes: superseded"));

        // Invariant: no stale copy left behind in todo/.
        let todo_entries: Vec<_> = fs::read_dir(root.path().join("todo"))
            .expect("read todo dir")
            .map(|entry| entry.expect("dir entry").file_name())
            .collect();
        assert!(
            todo_entries.is_empty(),
            "todo/ should be empty after cancel, found: {todo_entries:?}"
        );
    }

    #[test]
    fn rollback_move_to_todo_returns_file_from_done_to_todo() {
        // Directly exercises the rollback helper that cancel_task invokes when
        // the post-move append fails, proving that a failed-append state can
        // be unwound to a clean pre-cancel state.
        let root = new_pipeline_root();
        let stranded_path = root.path().join("done").join("T-77.5.md");
        fs::write(
            &stranded_path,
            "# T-77.5 — Stranded in done after hypothetical append failure\n",
        )
        .expect("write stranded task");

        let moved_task = crate::task_repository::TaskPathSet {
            task_id: "T-77.5".to_string(),
            path: stranded_path.clone(),
            location: "done".to_string(),
        };

        super::rollback_move_to_todo(root.path(), &moved_task)
            .expect("rollback should return the file to todo/");

        assert!(
            !stranded_path.exists(),
            "rollback should remove the file from done/"
        );
        let todo_path = root.path().join("todo").join("T-77.5.md");
        assert!(
            todo_path.exists(),
            "rollback should restore the file to todo/"
        );
        let restored = fs::read_to_string(&todo_path).expect("read restored task");
        assert!(
            !restored.contains("## Cancelled"),
            "rolled-back file must not carry a cancel section"
        );
    }

    #[test]
    fn cancel_rejects_already_cancelled_task() {
        let root = new_pipeline_root();
        let task_path = root.path().join("done").join("T-77.2.md");
        fs::write(
            &task_path,
            "# T-77.2 — Already cancelled\n\n**Pipeline**: sdd\n\n## Acceptance Criteria\n\n1. cancel rejects double cancel\n\n## Cancelled\n\n- Stage ID: cancelled\n- Record Kind: cancel\n- Agent: pm\n- Recorded At: 2026-01-01T00:00:00Z\n- Outcome: CANCEL\n- Notes: original cancellation\n",
        )
        .expect("write cancelled task");

        let error = cancel_task(CancelArgs {
            context: GateContext {
                root: root.path().to_path_buf(),
                requested_pipeline_name: None,
                json_output: false,
            },
            task_id: "T-77.2".to_string(),
            agent: "pm".to_string(),
            notes: "again".to_string(),
            model: None,
        })
        .expect_err("double cancel should fail");

        assert!(error.to_string().contains("already cancelled"));
    }

    #[test]
    fn cancel_rejects_completed_task() {
        let root = new_pipeline_root();
        let task_path = root.path().join("done").join("T-77.3.md");
        fs::write(
            &task_path,
            "# T-77.3 — Completed task\n\n**Pipeline**: sdd\n\n## Acceptance Criteria\n\n1. cancel rejects done\n\n## Pending\n\n- Stage ID: pending\n- Record Kind: outcome\n- Agent: pm\n- Recorded At: 2026-01-01T00:00:00Z\n- Outcome: PASS\n- Notes: spec approved\n\n## SWE\n\n- Stage ID: swe\n- Record Kind: outcome\n- Agent: swe\n- Recorded At: 2026-01-02T00:00:00Z\n- Outcome: PASS\n- Notes: implemented\n\n## Red-Team\n\n- Stage ID: red-team\n- Record Kind: outcome\n- Agent: red\n- Recorded At: 2026-01-03T00:00:00Z\n- Outcome: PASS\n- Notes: reviewed\n\n## Blue-Team\n\n- Stage ID: blue-team\n- Record Kind: outcome\n- Agent: blue\n- Recorded At: 2026-01-04T00:00:00Z\n- Outcome: PASS\n- Notes: verified\n",
        )
        .expect("write done task");

        let error = cancel_task(CancelArgs {
            context: GateContext {
                root: root.path().to_path_buf(),
                requested_pipeline_name: None,
                json_output: false,
            },
            task_id: "T-77.3".to_string(),
            agent: "pm".to_string(),
            notes: "too late".to_string(),
            model: None,
        })
        .expect_err("cancel on done should fail");

        assert!(error.to_string().contains("already done"));
    }

    #[cfg(unix)]
    #[test]
    fn cancel_rejects_symlinked_task_path() {
        let root = new_pipeline_root();
        let target = root.path().join("todo").join("cancel-target.md");
        fs::write(
            &target,
            "# T-77.4 — Cancel target\n\n**Pipeline**: sdd\n\n## Acceptance Criteria\n\n1. cancel should not touch symlinks\n",
        )
        .expect("write cancel target");
        let task_path = root.path().join("todo").join("T-77.4.md");
        unix_fs::symlink(&target, &task_path).expect("create symlinked task");

        let error = cancel_task(CancelArgs {
            context: GateContext {
                root: root.path().to_path_buf(),
                requested_pipeline_name: None,
                json_output: false,
            },
            task_id: "T-77.4".to_string(),
            agent: "pm".to_string(),
            notes: "should fail".to_string(),
            model: None,
        })
        .expect_err("symlinked cancel should fail");

        assert!(error.to_string().contains("through a symlink"));
        let unchanged = fs::read_to_string(&target).expect("read cancel target");
        assert!(!unchanged.contains("## Cancelled"));
    }

    fn new_pipeline_root() -> TempDir {
        let temp = tempfile::tempdir().expect("create temp root");
        fs::create_dir_all(temp.path().join("todo")).expect("create todo");
        fs::create_dir_all(temp.path().join("done")).expect("create done");
        temp
    }
}
