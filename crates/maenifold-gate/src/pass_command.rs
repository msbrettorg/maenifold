use crate::cli::OutcomeArgs;
use crate::error::GateError;
use crate::gate_catalog::GateDefinitionCatalog;
use crate::gate_tool_service::GateToolService;
use crate::section_factory::CanonicalSectionFactory;
use crate::state_machine::StateMachine;
use crate::task_file::{TaskFileLock, TaskFileReader, TaskFileWriter};
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
    let document = TaskFileReader::read(&task.path)?;
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
    TaskFileWriter::append_section(&task.path, &section)?;
    if definition
        .find_state(&transition.next_state)
        .ok_or_else(|| GateError::UnknownState(transition.next_state.clone()))?
        .terminal
    {
        TaskRepository::move_to_done(&task)?;
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
    TaskFileWriter::append_section(&task.path, &section)?;
    TaskRepository::move_to_todo(&task)?;
    Ok(())
}
