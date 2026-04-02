use crate::cli::AppendArgs;
use crate::error::GateError;
use crate::gate_catalog::GateDefinitionCatalog;
use crate::section_factory::CanonicalSectionFactory;
use crate::state_machine::StateMachine;
use crate::task_file::{TaskFileReader, TaskFileWriter};
use crate::task_repository::TaskRepository;
use crate::task_status::TaskStatusDeriver;

#[derive(Debug, Clone)]
pub struct AppendCommand;

impl AppendCommand {
    pub fn execute(args: AppendArgs) -> Result<(), GateError> {
        let task = TaskRepository::find_task(args.context.root.clone(), &args.task_id)?;
        let document = TaskFileReader::read(&task.path)?;
        let definition = GateDefinitionCatalog::resolve_for_task(
            &args.context.root,
            &document.metadata,
            args.context.requested_pipeline_name.as_deref(),
        )?;
        let state = TaskStatusDeriver::derive(&document, &definition, &task.location)?;
        let acting_stage = StateMachine::acting_stage_name(&state.state, &definition)?;
        if args.agent != acting_stage {
            return Err(GateError::IllegalTransition {
                state: state.state,
                outcome: format!("append by {}", args.agent),
            });
        }

        let section =
            CanonicalSectionFactory::build_append_section(&args.agent, &args.agent, &args.notes, args.model.as_deref());
        TaskFileWriter::append_section(&task.path, &section)?;
        Ok(())
    }
}
