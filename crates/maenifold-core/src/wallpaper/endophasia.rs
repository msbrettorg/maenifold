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
    let image_data = client
        .generate(&prompt, &wallpaper.size, &wallpaper.quality)
        .await?;

    cache::save_image(
        &cache::cache_dir(&config.memory_path),
        "endophasia",
        "recent",
        &image_data,
    )
}
