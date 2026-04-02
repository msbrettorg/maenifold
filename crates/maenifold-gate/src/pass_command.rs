use crate::cli::OutcomeArgs;
use crate::error::GateError;
use crate::gate_tool_service::GateToolService;
use crate::gate_catalog::GateDefinitionCatalog;
use crate::section_factory::CanonicalSectionFactory;
use crate::state_machine::StateMachine;
use crate::task_file::{TaskFileReader, TaskFileWriter};
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

pub(crate) fn execute_shared_outcome(args: OutcomeArgs, outcome: &str) -> Result<(), GateError> {
    match outcome {
        "pass" => pass_task(args),
        "fail" => fail_task(args),
        _ => unreachable!(),
    }
}

pub(crate) fn pass_task(args: OutcomeArgs) -> Result<(), GateError> {
    apply_outcome(args, "pass")
}

pub(crate) fn fail_task(args: OutcomeArgs) -> Result<(), GateError> {
    apply_outcome(args, "fail")
}

fn apply_outcome(args: OutcomeArgs, outcome: &str) -> Result<(), GateError> {
    let task = TaskRepository::find_task(args.context.root.clone(), &args.task_id)?;
    let document = TaskFileReader::read(&task.path)?;
    let definition = GateDefinitionCatalog::resolve_for_task(
        &args.context.root,
        &document.metadata,
        args.context.requested_pipeline_name.as_deref(),
    )?;
    let state = TaskStatusDeriver::derive(&document, &definition, &task.location)?;
    let transition = match outcome {
        "pass" => StateMachine::next_on_pass(&state.state, &definition)?,
        "fail" => StateMachine::next_on_fail(&state.state, &definition)?,
        _ => unreachable!(),
    };

    let section = CanonicalSectionFactory::build_outcome_section(
        &transition.acting_stage,
        outcome,
        &transition.acting_stage,
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
