// T-1.3: RTM FR-3 / T-10.17: RTM FR-20 — preserve the public `assets` facade
// while splitting the implementation into focused submodules below 300 LOC each.
mod adopt;
mod manager;
mod support;
mod types;
mod update;

pub use adopt::adopt;
pub use manager::AssetManager;
pub use support::is_watchable_asset_json_path;
pub use types::{AssetCatalog, AssetKind};
pub use update::update_assets;
