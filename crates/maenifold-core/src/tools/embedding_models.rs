use super::context::{ToolContext, load_help_doc, render};
use super::embedding_models_actions::{handle_activate, handle_download};
use super::embedding_models_detect::{handle_delete, handle_list, handle_regenerate};
use super::embedding_models_scan::active_model_status;
use crate::error::Result;
use crate::mcp::schema::{EmbeddingModelsAction, EmbeddingModelsParams};
use crate::util::json::{OutputFormat, success};
use crate::vector;
use serde_json::{Value, json};
pub(crate) use super::embedding_catalog::{
    CatalogFile, CatalogModel, load_catalog, load_catalog_with_external_trust,
};

pub fn embedding_models(context: &ToolContext, payload: &Value) -> Result<String> {
    let learn = payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if learn {
        let help = load_help_doc(&context.config.assets_path, "EmbeddingModels")
            .unwrap_or_else(|| "ERROR: Help file not found for EmbeddingModels".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }

    let params: EmbeddingModelsParams = serde_json::from_value(payload.clone()).map_err(|e| {
        crate::error::MaenifoldError::Message(format!("Invalid EmbeddingModels parameters: {e}"))
    })?;

    let response = match &params.action {
        EmbeddingModelsAction::Status => handle_status(context, &params),
        EmbeddingModelsAction::List => handle_list(context, &params),
        EmbeddingModelsAction::Download => handle_download(context, &params)?,
        EmbeddingModelsAction::Activate => handle_activate(context, &params)?,
        EmbeddingModelsAction::Delete => handle_delete(context, payload)?,
        EmbeddingModelsAction::Regenerate => handle_regenerate(context, payload)?,
    };

    Ok(render(context.output_format, response))
}

fn handle_status(context: &ToolContext, params: &EmbeddingModelsParams) -> Value {
    let catalog =
        load_catalog_with_external_trust(&context.config.assets_path, params.trust_external_catalog);
    let active_status = active_model_status(context, &catalog);
    let active_model_id = active_status.model_id.clone();
    let active_model_status = active_status.to_json();
    let loaded = vector::is_initialized();
    if !loaded {
        return json!({
            "loaded": false,
            "activeModelId": active_model_id,
            "activeModelStatus": active_model_status,
        });
    }

    json!({
        "loaded": true,
        "activeModelId": active_model_id,
        "activeModelStatus": active_model_status,
        "model": {
            "filename": vector::model_filename().unwrap_or_default(),
            "dimensions": vector::embedding_dimensions().unwrap_or(vector::DEFAULT_EMBEDDING_DIMENSIONS),
            "maxTokens": vector::max_tokens().unwrap_or(vector::DEFAULT_MAX_TOKENS),
        }
    })
}
