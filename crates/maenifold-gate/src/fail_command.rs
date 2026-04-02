use crate::cli::VerdictArgs;
use crate::error::GateError;
use crate::pass_command::execute_shared_verdict;

#[derive(Debug, Clone)]
pub struct FailCommand;

impl FailCommand {
    pub fn execute(args: VerdictArgs) -> Result<(), GateError> {
        execute_shared_verdict(args, "fail")
    }
}
