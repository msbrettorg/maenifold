use crate::cli::OutcomeArgs;
use crate::error::GateError;
use crate::pass_command::{execute_shared_outcome, fail_task as pass_fail_task};

#[derive(Debug, Clone)]
pub struct FailCommand;

impl FailCommand {
    pub fn execute(args: OutcomeArgs) -> Result<(), GateError> {
        execute_shared_outcome(args, "fail")
    }
}

pub(crate) fn fail_task(args: OutcomeArgs) -> Result<(), GateError> {
    pass_fail_task(args)
}
