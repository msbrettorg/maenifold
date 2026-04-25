use serde::{Deserialize, Serialize};
use std::path::Path;

/// Bundled catalog JSON, embedded at compile time. Used when no trusted external
/// `{assets_path}/models/catalog.json` override is present.
const BUNDLED_CATALOG_JSON: &str = include_str!("../../assets/models/catalog.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CatalogFile {
    pub name: String,
    pub source_path: String,
    pub source: String,
    pub sha256: String,
    pub size_bytes: u64,
    #[serde(default)]
    pub optional: bool,
}

/// Owned catalog entry deserialized from JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CatalogModel {
    pub id: String,
    pub name: String,
    pub dimensions: usize,
    pub max_tokens: usize,
    pub size_bytes: u64,
    pub license: String,
    pub huggingface_repo: String,
    #[serde(default)]
    pub onnx_repo: Option<String>,
    pub revision: String,
    #[serde(default)]
    pub onnx_revision: Option<String>,
    pub released: String,
    #[serde(default)]
    pub mrl_dimensions: Option<Vec<usize>>,
    #[serde(default)]
    pub mteb_retrieval: Option<f32>,
    pub files: Vec<CatalogFile>,
}

pub(crate) fn load_catalog(assets_path: &Path) -> Vec<CatalogModel> {
    load_catalog_with_external_trust(assets_path, false)
}

/// Load the curated embedding model catalog.
///
/// External catalog overrides are ignored unless the caller explicitly opts in
/// with `trustExternalCatalog=true`. When trusted, provenance is logged so
/// operators can distinguish bundled catalog downloads from local overrides.
pub(crate) fn load_catalog_with_external_trust(
    assets_path: &Path,
    trust_external: bool,
) -> Vec<CatalogModel> {
    let external = assets_path.join("models").join("catalog.json");
    if external.is_file() {
        if !trust_external {
            tracing::warn!(
                "Ignoring external embedding model catalog override because explicit trust was not provided; using bundled catalog"
            );
        } else {
            tracing::warn!(
                "Using explicitly trusted external embedding model catalog override from configured assets directory"
            );
            match std::fs::read_to_string(&external) {
                Ok(text) => match serde_json::from_str::<Vec<CatalogModel>>(&text) {
                    Ok(models) => return models,
                    Err(e) => tracing::warn!(
                        "Failed to parse external embedding model catalog: {e}; falling back to bundled catalog"
                    ),
                },
                Err(e) => tracing::warn!(
                    "Failed to read external embedding model catalog: {e}; falling back to bundled catalog"
                ),
            }
        }
    }

    match serde_json::from_str::<Vec<CatalogModel>>(BUNDLED_CATALOG_JSON) {
        Ok(models) => models,
        Err(e) => {
            tracing::error!("Failed to parse bundled embedding model catalog JSON: {e}");
            Vec::new()
        }
    }
}
