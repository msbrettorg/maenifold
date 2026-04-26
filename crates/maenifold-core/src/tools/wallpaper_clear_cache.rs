use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use crate::wallpaper::cache;
use serde_json::{Value, json};

pub fn wallpaper_clear_cache(context: &ToolContext, payload: &Value) -> Result<String> {
    let mode = payload.get("mode").and_then(Value::as_str);
    let cache_dir = cache::cache_dir(&context.config.memory_path);
    cache::clear_cache(&cache_dir, mode)
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    Ok(json!({ "cleared": true, "mode": mode }).to_string())
}
