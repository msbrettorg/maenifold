use crate::assets::{AssetKind, AssetManager};
use crate::config::Config;
use crate::error::Result;
use rmcp::model::{AnnotateAble, RawResource, ReadResourceResult, Resource, ResourceContents};
use serde_json::Value;

pub(super) fn list_resources() -> Result<Vec<Resource>> {
    let asset_manager = current_asset_manager()?;
    let mut resources = Vec::new();
    for kind in AssetKind::all() {
        let metadata = asset_manager.list_assets(Some(kind))?;
        let Value::Array(items) = metadata else {
            continue;
        };
        for item in items {
            if let Some(resource) = resource_from_metadata(kind, item) {
                resources.push(resource);
            }
        }
    }
    Ok(resources)
}

pub(super) fn handle_read_resource(uri: &str) -> Result<ReadResourceResult> {
    let value = current_asset_manager()?.read_resource(uri)?;
    let text = serde_json::to_string_pretty(&value)?;
    Ok(ReadResourceResult::new(vec![ResourceContents::text(
        text,
        uri.to_string(),
    )
    .with_mime_type("application/json")]))
}

fn current_asset_manager() -> Result<AssetManager> {
    let config = Config::current();
    Ok(AssetManager::new(&config))
}

fn resource_from_metadata(kind: AssetKind, metadata: Value) -> Option<Resource> {
    let object = metadata.as_object()?;
    let id = object.get("id")?.as_str()?;
    let name = object
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(id)
        .to_string();
    let mut resource = RawResource::new(asset_uri(kind, id), name)
        .with_mime_type("application/json")
        .optional_annotate(None);
    if let Some(title) = object.get("name").and_then(Value::as_str) {
        resource.title = Some(title.to_string());
    }
    if let Some(description) = object.get("description").and_then(Value::as_str) {
        resource.description = Some(description.to_string());
    }
    Some(resource)
}

fn asset_uri(kind: AssetKind, id: &str) -> String {
    format!("asset://{}/{id}", kind.dir_name())
}
