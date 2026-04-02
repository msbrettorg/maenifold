use crate::cli::OutcomeArgs;
use crate::error::GateError;
use crate::pass_command::execute_shared_outcome;

#[derive(Debug, Clone)]
pub struct FailCommand;

impl FailCommand {
    pub fn execute(args: OutcomeArgs) -> Result<(), GateError> {
        execute_shared_outcome(args, "fail")
    }
}
