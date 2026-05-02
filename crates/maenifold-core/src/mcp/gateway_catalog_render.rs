use serde_json::Value;

pub(crate) fn strip_schemas(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.remove("schema");
            for child in map.values_mut() {
                strip_schemas(child);
            }
        }
        Value::Array(items) => {
            for item in items {
                strip_schemas(item);
            }
        }
        _ => {}
    }
}
