use serde_json::{json, Value};

use crate::db::skill::{delete_skill_internal, list_skills_internal, load_skill_by_name, upsert_skill_internal};
use crate::types::{AppSkillUpsert, AppState};

pub(crate) fn list_skills(state: &AppState) -> Result<Value, String> {
    let skills = list_skills_internal(state)?;
    let out: Vec<Value> = skills
        .iter()
        .map(|s| json!({ "id": s.id, "name": s.name, "description": s.description }))
        .collect();
    Ok(json!(out))
}

pub(crate) fn get_skill(state: &AppState, params: Value) -> Result<Value, String> {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("name is required")?;
    let skill = load_skill_by_name(state, name)?.ok_or_else(|| format!("skill not found: {name}"))?;
    Ok(json!({
        "id": skill.id,
        "name": skill.name,
        "description": skill.description,
        "content": skill.content,
    }))
}

pub(crate) fn upsert_skill(state: &AppState, params: Value) -> Result<Value, String> {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("name is required")?
        .to_string();
    let description = params.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let content = params.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let id = params.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
    let skill = upsert_skill_internal(state, AppSkillUpsert { id, name: name.clone(), description, content })?;
    Ok(json!(format!("Skill '{}' saved (id: {})", skill.name, skill.id)))
}

pub(crate) fn delete_skill(state: &AppState, params: Value) -> Result<Value, String> {
    let id = params
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("id is required")?;
    delete_skill_internal(state, id)?;
    Ok(json!(format!("Skill '{id}' deleted")))
}
