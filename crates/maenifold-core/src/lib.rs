#[path = "acp/config.rs"]
pub(crate) mod acp_config;
pub(crate) mod api;
pub mod assets;
pub mod benchmark;
pub mod cli;
pub mod community;
pub mod config;
pub mod db;
pub mod decay;
pub(crate) mod endophasia;
pub mod error;
pub mod graph;
pub mod lineage;
pub mod logging;
pub mod markdown;
// T-12.0: RTM NFR-1 — preserve the crate::mcp facade while the implementation lives in src/mcp/.
pub mod mcp;
pub mod memory;
#[cfg(test)]
pub(crate) mod test_util;
// T-12.6: RTM NFR-1 - preserve the public utility surface while moving helper modules under util/.
pub mod util;
pub use util::json;
pub use util::paths;
pub mod registry;
pub(crate) mod registry_gateway;
// T-12.2: RTM FR-6 / FR-2 - expose extracted repair/analysis module.
pub mod repair;
pub mod schedule;
pub mod search;
pub mod sessions;
pub mod similar;
pub mod sync;
pub mod thinking;
pub mod tools;
// T-29.2: RTM FR-35 — system tray icon for --http mode.
#[cfg(feature = "app")]
pub mod tray;
#[cfg(feature = "app")]
pub(crate) mod tray_helpers;
#[cfg(feature = "app")]
pub mod tray_menu;
pub mod vector;
pub mod workflow;

pub use assets::{AssetCatalog, AssetKind, AssetManager};
pub use cli::{CliAction, CliApp};
pub use config::Config;
pub use db::GraphDatabase;
pub use error::{MaenifoldError, Result};
pub use mcp::{McpToolDefinition, list_mcp_tools};
pub use registry::{ToolDescriptor, ToolRegistry};
