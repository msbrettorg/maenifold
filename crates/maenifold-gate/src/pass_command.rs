use crate::cli::VerdictArgs;
use crate::error::GateError;
use crate::gate_catalog::GateDefinitionCatalog;
use crate::section_factory::CanonicalSectionFactory;
use crate::state_machine::StateMachine;
use crate::task_file::{TaskFileReader, TaskFileWriter};
use crate::task_repository::TaskRepository;
use crate::task_status::TaskStatusDeriver;

#[derive(Debug, Clone)]
pub struct PassCommand;

impl PassCommand {
    pub fn execute(args: VerdictArgs) -> Result<(), GateError> {
        execute_verdict(args, "pass")
    }
}

fn execute_verdict(args: VerdictArgs, verdict: &str) -> Result<(), GateError> {
    let task = TaskRepository::find_task(args.context.root.clone(), &args.task_id)?;
    let document = TaskFileReader::read(&task.path)?;
    let definition = GateDefinitionCatalog::resolve_for_task(
        &args.context.root,
        &document.metadata,
        args.context.requested_pipeline_name.as_deref(),
    )?;
    let state = TaskStatusDeriver::derive(&document, &definition, &task.location)?;
    let transition = match verdict {
        "pass" => StateMachine::next_on_pass(&state.state, &definition)?,
        "reject" => StateMachine::next_on_reject(&state.state, &definition)?,
        "fail" => StateMachine::next_on_fail(&state.state, &definition)?,
        _ => unreachable!(),
    };

    let section = CanonicalSectionFactory::build_verdict_section(
        &transition.acting_stage,
        verdict,
        &args.notes,
        args.model.as_deref(),
    );
    TaskFileWriter::append_section(&task.path, &section)?;
    if transition.next_state == "done" {
        TaskRepository::move_to_done(&task)?;
    }
    Ok(())
}

pub(crate) fn execute_shared_verdict(args: VerdictArgs, verdict: &str) -> Result<(), GateError> {
    execute_verdict(args, verdict)
}
