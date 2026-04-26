use super::context::{load_help_doc, render, ToolContext};
pub(crate) use super::embedding_catalog::{
    load_catalog, load_catalog_with_external_trust, CatalogModel,
};
#[cfg(feature = "model-download")]
pub(crate) use super::embedding_catalog::CatalogFile;
use super::embedding_models_actions::{handle_activate, handle_download};
use super::embedding_models_detect::{handle_delete, handle_list, handle_regenerate};
use super::embedding_models_scan::active_model_status;
use crate::db;
use crate::error::Result;
use crate::mcp::schema::{EmbeddingModelsAction, EmbeddingModelsParams};
use crate::util::json::{success, OutputFormat};
use crate::vector;
use serde_json::{json, Value};

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
    let catalog = load_catalog_with_external_trust(
        &context.config.assets_path,
        params.trust_external_catalog,
    );
    let active_status = active_model_status(context, &catalog);
    let active_model_id = active_status.model_id.clone();
    let active_model_status = active_status.to_json();
    let model_installed = active_status.installed_layout.is_some();
    let vector_status = vector_index_status(context);
    let loaded = vector::is_initialized();
    if !loaded {
        return json!({
            "loaded": false,
            "activeModelId": active_model_id,
            "activeModelStatus": active_model_status,
            "modelInstalled": model_installed,
            "vec0": vector_status,
        });
    }

    json!({
        "loaded": true,
        "activeModelId": active_model_id,
        "activeModelStatus": active_model_status,
        "modelInstalled": model_installed,
        "vec0": vector_status,
        "model": {
            "filename": vector::model_filename().unwrap_or_default(),
            "dimensions": vector::embedding_dimensions().unwrap_or(vector::DEFAULT_EMBEDDING_DIMENSIONS),
            "maxTokens": vector::max_tokens().unwrap_or(vector::DEFAULT_MAX_TOKENS),
        }
    })
}

fn vector_index_status(context: &ToolContext) -> Value {
    let conn = match db::open_connection(&context.config.database_path) {
        Ok(conn) => conn,
        Err(error) => {
            let load_error = sanitize_status_error(&error.to_string());
            return json!({
                "extensionPath": null,
                "installed": false,
                "rustLoadable": false,
                "loadError": load_error,
                "tableCreationProbe": {
                    "success": false,
                    "error": "Database could not be opened for vec0 verification.",
                },
                "vectorTablesReady": false,
                "embeddingsPopulated": false,
                "rebuildRecommended": true,
                "message": "Open the database successfully, then run sync to initialize vector tables.",
            });
        }
    };

    let verification = db::verify_vec0_load(&conn, &context.config.assets_path);
    let vec_concepts = vector_table_status(&conn, "vec_concepts");
    let vec_memory_files = vector_table_status(&conn, "vec_memory_files");
    let vector_tables_ready = vec_concepts.ready && vec_memory_files.ready;
    let embeddings_populated =
        vec_concepts.count.unwrap_or(0) > 0 || vec_memory_files.count.unwrap_or(0) > 0;
    let rebuild_recommended =
        verification.load_success && verification.table_probe_success && !embeddings_populated;

    json!({
        "extensionPath": verification.extension_path.as_ref().map(redacted_extension_path),
        "installed": verification.installed,
        "rustLoadable": verification.load_success,
        "loadError": verification.load_error,
        "tableCreationProbe": {
            "success": verification.table_probe_success,
            "error": verification.table_probe_error,
        },
        "vectorTablesReady": vector_tables_ready,
        "embeddingsPopulated": embeddings_populated,
        "rebuildRecommended": rebuild_recommended,
        "message": vec0_status_message(
            verification.installed,
            verification.load_success,
            verification.table_probe_success,
            vector_tables_ready,
            embeddings_populated,
        ),
        "tables": {
            "vecConcepts": vec_concepts.to_json(),
            "vecMemoryFiles": vec_memory_files.to_json(),
        },
    })
}

fn redacted_extension_path(path: &std::path::PathBuf) -> String {
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("[resolved]");
    let platform_dir = path
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str());

    match platform_dir {
        Some(platform_dir) => format!("{platform_dir}/{filename}"),
        None => filename.to_string(),
    }
}

fn sanitize_status_error(error: &str) -> &'static str {
    let lower = error.to_ascii_lowercase();
    if lower.contains("not found") || lower.contains("no such file") {
        "resource_not_found"
    } else if lower.contains("database") || lower.contains("sqlite") {
        "database_open_failed"
    } else if lower.contains("load") {
        "extension_load_failed"
    } else {
        "verification_failed"
    }
}

struct VectorTableStatus {
    exists: bool,
    ready: bool,
    count: Option<i64>,
    error: Option<String>,
}

impl VectorTableStatus {
    fn to_json(&self) -> Value {
        json!({
            "exists": self.exists,
            "ready": self.ready,
            "count": self.count,
            "error": self.error.as_deref(),
        })
    }
}

fn vector_table_status(conn: &rusqlite::Connection, table_name: &str) -> VectorTableStatus {
    let exists = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table_name],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count > 0)
        .unwrap_or(false);

    if !exists {
        return VectorTableStatus {
            exists,
            ready: false,
            count: None,
            error: None,
        };
    }

    let sql = format!("SELECT COUNT(*) FROM {table_name}");
    match conn.query_row(&sql, [], |row| row.get::<_, i64>(0)) {
        Ok(count) => VectorTableStatus {
            exists,
            ready: true,
            count: Some(count),
            error: None,
        },
        Err(error) => VectorTableStatus {
            exists,
            ready: false,
            count: None,
            error: Some(error.to_string()),
        },
    }
}

fn vec0_status_message(
    installed: bool,
    rust_loadable: bool,
    table_probe_success: bool,
    vector_tables_ready: bool,
    embeddings_populated: bool,
) -> &'static str {
    if !installed {
        "vec0 is not installed for this platform. Download vec0 before rebuilding embeddings."
    } else if !rust_loadable {
        "vec0 is installed but Rust/rusqlite could not load it."
    } else if !table_probe_success {
        "vec0 loaded in Rust, but the virtual table creation probe failed."
    } else if !vector_tables_ready {
        "vec0 is loadable in Rust. Run sync to create vector tables."
    } else if !embeddings_populated {
        "vec0 vector tables are ready. Run sync to populate embeddings."
    } else {
        "vec0 is loadable in Rust, vector tables are ready, and embeddings are populated."
    }
}
