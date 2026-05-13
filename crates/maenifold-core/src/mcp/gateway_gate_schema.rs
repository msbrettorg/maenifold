use serde_json::{Map, Value, json};

pub(super) fn base_properties() -> Map<String, Value> {
    let mut properties = Map::new();
    properties.insert(
        "root".to_string(),
        json!({"type": "string", "description": "Absolute repository root containing todo/ and done/."}),
    );
    properties.insert(
        "pipeline".to_string(),
        json!({"type": "string", "description": "Optional pipeline id to require for task resolution."}),
    );
    properties
}

pub(super) fn create_schema() -> Value {
    let mut properties = Map::new();
    properties.insert(
        "root".to_string(),
        json!({"type": "string", "description": "Absolute repository root containing todo/ and done/."}),
    );
    properties.insert(
        "taskId".to_string(),
        json!({"type": "string", "pattern": "^T-\\d+\\.\\d+$"}),
    );
    properties.insert(
        "title".to_string(),
        json!({"type": "string", "minLength": 1}),
    );
    properties.insert("pipeline".to_string(), json!({"type": "string"}));
    properties.insert("body".to_string(), json!({"type": "string"}));
    properties.insert(
        "priority".to_string(),
        json!({"type": "string", "enum": ["P0", "P1", "P2", "P3", "P4"]}),
    );
    properties.insert("sprint".to_string(), json!({"type": "integer"}));
    properties.insert("parent".to_string(), json!({"type": "string"}));
    json!({"type": "object", "properties": properties, "required": ["root", "taskId", "title", "pipeline"], "additionalProperties": false})
}

pub(super) fn status_schema() -> Value {
    let mut properties = base_properties();
    properties.insert("state".to_string(), json!({"type": "string"}));
    properties.insert("ready".to_string(), json!({"type": "boolean"}));
    properties.insert("blocked".to_string(), json!({"type": "boolean"}));
    properties.insert("includeDone".to_string(), json!({"type": "boolean"}));
    properties.insert("summary".to_string(), json!({"type": "boolean"}));
    json!({"type": "object", "properties": properties, "required": ["root"], "additionalProperties": false})
}

pub(super) fn note_schema() -> Value {
    let mut properties = base_properties();
    properties.insert("taskId".to_string(), json!({"type": "string"}));
    properties.insert("agent".to_string(), json!({"type": "string"}));
    properties.insert("notes".to_string(), json!({"type": "string"}));
    properties.insert("model".to_string(), json!({"type": "string"}));
    properties.insert(
        "expectedStage".to_string(),
        json!({"type": "string", "description": "Optional guard requiring the task's current stage to match before advancing."}),
    );
    json!({"type": "object", "properties": properties, "required": ["root", "taskId", "agent", "notes"], "additionalProperties": false})
}

pub(super) fn task_schema() -> Value {
    let mut properties = base_properties();
    properties.insert("taskId".to_string(), json!({"type": "string"}));
    json!({"type": "object", "properties": properties, "required": ["root", "taskId"], "additionalProperties": false})
}

pub(super) fn sync_schema() -> Value {
    let mut properties = base_properties();
    properties.insert("rebuild".to_string(), json!({"type": "boolean"}));
    json!({"type": "object", "properties": properties, "required": ["root"], "additionalProperties": false})
}
