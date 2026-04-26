use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;

use crate::api::graph::{get_communities, get_community_concepts};
use crate::config::Config;
use crate::db;
use crate::wallpaper::{cache, image_client::ImageClient};

pub async fn generate_zeitgeist_wallpapers(config: &Config) -> Result<Vec<PathBuf>> {
    let wallpaper = &config.settings.easter_eggs.wallpaper;
    let conn = db::open_connection(&config.database_path)?;
    let mut graph = get_communities(&conn)?;

    if graph.nodes.is_empty() {
        return Ok(vec![]);
    }

    graph
        .nodes
        .sort_by(|left, right| right.size.cmp(&left.size).then_with(|| left.id.cmp(&right.id)));

    let communities = graph
        .nodes
        .into_iter()
        .take(wallpaper.community_count as usize)
        .collect::<Vec<_>>();
    let community_count = communities.len();
    let client = ImageClient::from_config(wallpaper)?;
    let cache_dir = cache::cache_dir(&config.memory_path);
    let mut paths = Vec::with_capacity(community_count);

    for (index, node) in communities.into_iter().enumerate() {
        let concepts = get_community_concepts(&conn, node.id)?;
        let mut concept_prompt = format!("**[[{}]]**", node.anchor);

        for concept in concepts
            .nodes
            .iter()
            .filter(|concept| concept.name != node.anchor)
        {
            concept_prompt.push_str(&format!(" [[{}]]", concept.name));
        }

        let prompt = format!("{}\n\n{concept_prompt}", wallpaper.prompt);
        let data = client
            .generate(&prompt, &wallpaper.size, &wallpaper.quality)
            .await?;
        let path = cache::save_image(&cache_dir, "zeitgeist", &node.anchor, &data)?;
        paths.push(path);

        if index + 1 < community_count {
            tokio::time::sleep(Duration::from_secs(15)).await;
        }
    }

    Ok(paths)
}
