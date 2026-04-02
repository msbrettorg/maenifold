use crate::cli::VerdictArgs;
use crate::error::GateError;
use crate::pass_command::execute_shared_verdict;

#[derive(Debug, Clone)]
pub struct RejectCommand;

impl RejectCommand {
    pub fn execute(args: VerdictArgs) -> Result<(), GateError> {
        execute_shared_verdict(args, "reject")
    }
}
