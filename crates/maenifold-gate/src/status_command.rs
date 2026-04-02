use serde_json::json;

use crate::cli::StatusArgs;
use crate::error::GateError;
use crate::gate_catalog::GateDefinitionCatalog;
use crate::json_output::JsonRenderer;
use crate::task_file::TaskFileReader;
use crate::task_repository::TaskRepository;
use crate::task_status::TaskStatusDeriver;

#[derive(Debug, Clone)]
pub struct StatusCommand;

impl StatusCommand {
    pub fn execute(args: StatusArgs) -> Result<(), GateError> {
        let tasks = TaskRepository::list_tasks(args.context.root.clone())?;
        let mut values = Vec::with_capacity(tasks.len());

        for task in tasks {
            let document = TaskFileReader::read(&task.path)?;
            let definition = GateDefinitionCatalog::resolve_for_task(
                &args.context.root,
                &document.metadata,
                args.context.requested_pipeline_name.as_deref(),
            )?;
            let state = TaskStatusDeriver::derive(&document, &definition, &task.location)?;
            values.push(json!({
                "taskId": task.task_id,
                "title": document.metadata.title,
                "pipeline": document.metadata.pipeline,
                "pipelineDigest": document.metadata.pipeline_digest,
                "status": state.state,
                "state": state.state,
                "attempts": state.attempts,
                "location": state.location,
                "path": task.path,
            }));
        }

        if args.context.json_output {
            println!("{}", JsonRenderer::render_status(serde_json::Value::Array(values))?);
        } else {
            for value in values {
                println!(
                    "{} {} {}",
                    value["taskId"].as_str().unwrap_or_default(),
                    value["pipeline"].as_str().unwrap_or_default(),
                    value["state"].as_str().unwrap_or_default()
                );
            }
        }

        Ok(())
    }
}
