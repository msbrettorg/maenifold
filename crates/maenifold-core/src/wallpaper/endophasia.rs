use std::path::PathBuf;

use anyhow::Result;

use crate::config::Config;
use crate::tools::{collect_recent_activity_records, format_recent_activity_report};
use crate::wallpaper::{cache, image_client::ImageClient};

pub async fn generate_endophasia_wallpaper(config: &Config) -> Result<PathBuf> {
    let wallpaper = &config.settings.easter_eggs.wallpaper;
    let rows = collect_recent_activity_records(config, None, Some("all"), 10)?;
    let report = format_recent_activity_report(rows, true);
    let prompt = format!("{}\n\n{report}", wallpaper.prompt);

    let client = ImageClient::from_config(wallpaper)?;
    let cache_dir = cache::cache_dir(&config.memory_path);
    let image_data = if wallpaper.continuity {
        if let Some(prev) = cache::latest_image(&cache_dir, "endophasia") {
            let prev_bytes = std::fs::read(&prev)?;
            let edit_prompt = format!("Update this image to reflect the latest state: {prompt}");
            client
                .edit(
                    &edit_prompt,
                    &prev_bytes,
                    &wallpaper.size,
                    &wallpaper.quality,
                    &wallpaper.input_fidelity,
                )
                .await?
        } else {
            client
                .generate(&prompt, &wallpaper.size, &wallpaper.quality)
                .await?
        }
    } else {
        client
            .generate(&prompt, &wallpaper.size, &wallpaper.quality)
            .await?
    };

    cache::save_image(&cache_dir, "endophasia", "recent", &image_data)
}
