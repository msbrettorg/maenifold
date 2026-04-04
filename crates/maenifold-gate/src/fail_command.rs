use crate::cli::OutcomeArgs;
use crate::error::GateError;
use crate::gate_tool_service::GateToolService;
use crate::pass_command::fail_task as pass_fail_task;

#[derive(Debug, Clone)]
pub struct FailCommand;

impl FailCommand {
    pub fn execute(args: OutcomeArgs) -> Result<(), GateError> {
        let _ = GateToolService::fail(args)?;
        Ok(())
    }
}

pub(crate) fn fail_task(args: OutcomeArgs) -> Result<(), GateError> {
    pass_fail_task(args)
}
