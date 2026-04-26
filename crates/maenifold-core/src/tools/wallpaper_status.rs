use crate::error::Result;
use crate::tools::ToolContext;
use crate::wallpaper::{cache, service_state};
use serde_json::{Value, json};

pub fn wallpaper_status(context: &ToolContext, _payload: &Value) -> Result<String> {
    let wallpaper = &context.config.settings.easter_eggs.wallpaper;
    let state = service_state().read();
    let images = cache::list_images(
        &cache::cache_dir(&context.config.memory_path),
        &wallpaper.mode,
    )
    .into_iter()
    .map(|path| path.to_string_lossy().to_string())
    .collect::<Vec<_>>();

    let status = json!({
        "enabled": wallpaper.enabled,
        "mode": &wallpaper.mode,
        "configured": azure_env_configured(wallpaper),
        "images": images,
        "lastGenerated": state.last_generated.as_ref().map(|time| time.to_rfc3339()),
        "error": state.error.clone(),
    });

    Ok(serde_json::to_string_pretty(&status)?)
}

fn azure_env_configured(wallpaper: &crate::acp_config::WallpaperSettings) -> bool {
    env_var_is_set(&wallpaper.azure.endpoint_env_var)
        && env_var_is_set(&wallpaper.azure.api_key_env_var)
}

fn env_var_is_set(name: &str) -> bool {
    std::env::var(name)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}
