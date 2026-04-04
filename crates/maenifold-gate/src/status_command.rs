use crate::cli::StatusArgs;
use crate::error::GateError;
use crate::gate_tool_service::GateToolService;
use crate::json_output::JsonRenderer;

#[derive(Debug, Clone)]
pub struct StatusCommand;

impl StatusCommand {
    pub fn execute(args: StatusArgs) -> Result<(), GateError> {
        let value = GateToolService::status(args.clone())?;

        if args.context.json_output {
            println!("{}", JsonRenderer::render_status(value)?);
        } else {
            let values = value.as_array().ok_or_else(|| {
                GateError::TaskMetadataParse("status output must be an array".to_string())
            })?;
            for value in values {
                let task_id = value["taskId"].as_str().unwrap_or_default();
                let pipeline = value["pipeline"].as_str().unwrap_or("-");
                let state = value["state"].as_str().unwrap_or_default();
                let issue = value["issue"].as_str().unwrap_or_default();
                if issue.is_empty() {
                    println!("{task_id} {pipeline} {state}");
                } else {
                    let path = value["path"].as_str().unwrap_or_default();
                    println!("{task_id} {pipeline} {state} {path}: {issue}");
                }
            }
        }

        Ok(())
    }
}
