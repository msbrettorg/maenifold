use serde_json::{json, Value};

use crate::db::workflow::{
    create_workflow_internal, delete_workflow_internal, list_workflows_internal, load_workflow,
    update_workflow_internal,
};
use crate::types::AppState;

pub(crate) fn get_workflow(state: &AppState, params: Value) -> Result<Value, String> {
    let id = params
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("id is required")?;
    let wf = load_workflow(state, id)?;
    Ok(json!({
        "id": wf.id,
        "name": wf.name,
        "steps": wf.steps,
        "createdAt": wf.created_at,
        "updatedAt": wf.updated_at,
    }))
}

pub(crate) fn update_workflow(state: &AppState, params: Value) -> Result<Value, String> {
    let id = params
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("id is required")?;
    let name = params.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
    let steps: Option<Vec<String>> = params.get("steps").and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|s| s.as_str().map(|x| x.to_string()))
            .collect()
    });
    let wf = update_workflow_internal(state, id, name, steps)?;
    Ok(json!(format!("Workflow '{}' updated", wf.name)))
}

pub(crate) fn list_workflows(state: &AppState) -> Result<Value, String> {
    let workflows = list_workflows_internal(state)?;
    let out: Vec<Value> = workflows
        .iter()
        .map(|w| json!({ "id": w.id, "name": w.name, "steps": w.steps }))
        .collect();
    Ok(json!(out))
}

pub(crate) fn create_workflow(state: &AppState, params: Value) -> Result<Value, String> {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("name is required")?
        .to_string();
    let steps: Vec<String> = params
        .get("steps")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|s| s.as_str().map(|x| x.to_string())).collect())
        .unwrap_or_default();
    let wf = create_workflow_internal(state, name, steps)?;
    Ok(json!(format!("Workflow '{}' created (id: {})", wf.name, wf.id)))
}

pub(crate) fn delete_workflow(state: &AppState, params: Value) -> Result<Value, String> {
    let id = params
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("id is required")?;
    delete_workflow_internal(state, id)?;
    Ok(json!(format!("Workflow '{id}' deleted")))
}
