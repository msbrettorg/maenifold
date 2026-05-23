use super::context::{ToolContext, load_help_doc, render};
use crate::error::{MaenifoldError, Result};
use crate::util::json::OutputFormat;
use serde_json::{Value, json};
use tokio::runtime::{Builder, Handle, RuntimeFlavor};

pub fn get_copilot_models(context: &ToolContext, payload: &Value) -> Result<String> {
    let learn = payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if learn {
        let help = load_help_doc(&context.config.assets_path, "GetCopilotModels")
            .unwrap_or_else(|| "ERROR: Help file not found for GetCopilotModels".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => crate::util::json::success(help),
            OutputFormat::Markdown => help,
        });
    }
    let refresh = payload
        .get("refresh")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let root = context.config.maenifold_root.clone();
    let future = async move {
        let services = jockey::copilot_models::CopilotModelsServices::for_config_root(root)
            .map_err(|error| MaenifoldError::Message(error.to_string()))?;
        jockey::copilot_models::get_copilot_models(&services, refresh)
            .await
            .map_err(map_copilot_models_error)
    };
    let result = match Handle::try_current() {
        Ok(handle) => {
            if !matches!(handle.runtime_flavor(), RuntimeFlavor::MultiThread) {
                return Err(MaenifoldError::Message(
                    "GetCopilotModels requires a multi-thread Tokio runtime when invoked from async context".to_string(),
                ));
            }
            tokio::task::block_in_place(|| handle.block_on(future))
        }
        Err(_) => Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| {
                MaenifoldError::Message(format!(
                    "Failed to build runtime for GetCopilotModels: {error}"
                ))
            })?
            .block_on(future),
    }?;
    Ok(render(
        context.output_format,
        json!({
            "models": result.models,
            "cachedAt": result.cached_at.to_rfc3339(),
            "source": result.source,
        }),
    ))
}

fn map_copilot_models_error(error: jockey::copilot_models::GetCopilotModelsError) -> MaenifoldError {
    let message = match error {
        jockey::copilot_models::GetCopilotModelsError::TokenMissing => {
            "Copilot token missing from ~/.config/github-copilot".to_string()
        }
        jockey::copilot_models::GetCopilotModelsError::EndpointUnreachable => {
            "Copilot /models endpoint unreachable".to_string()
        }
        jockey::copilot_models::GetCopilotModelsError::ParseFailed => {
            "Copilot /models response could not be parsed".to_string()
        }
        jockey::copilot_models::GetCopilotModelsError::Forbidden => {
            "Copilot /models request returned HTTP 403".to_string()
        }
        jockey::copilot_models::GetCopilotModelsError::Other => {
            "Copilot /models request failed".to_string()
        }
    };
    MaenifoldError::Message(message)
}
