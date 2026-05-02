use crate::error::Result;
use rusqlite::{Connection, params};
use serde_json::{Value, json};

pub(crate) struct PipelineLookup {
    pub(crate) file_path: String,
    pub(crate) status: Option<String>,
}

pub(crate) fn lookup_pipeline_by_uri(
    connection: &Connection,
    uri: &str,
) -> Result<Option<PipelineLookup>> {
    let mut statement =
        connection.prepare("SELECT file_path, status FROM file_content WHERE file_path = ?1")?;
    let mut rows = statement.query(params![uri])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    Ok(Some(PipelineLookup {
        file_path: row.get(0)?,
        status: row.get(1)?,
    }))
}

pub(crate) fn lookup_pipeline_degraded_json(
    connection: &Connection,
    uri: &str,
    reason: &str,
    read_error: &str,
) -> Result<Option<Value>> {
    Ok(lookup_pipeline_by_uri(connection, uri)?.map(|row| {
        json!({ "pipelineId": row.file_path, "status": row.status, "nodeCount": Value::Null,
            "completedCount": Value::Null,
            "degraded": { "source": "db", "reason": reason, "readError": read_error } })
    }))
}
