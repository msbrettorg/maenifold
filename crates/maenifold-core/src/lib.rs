#![warn(clippy::too_many_lines)]
#![warn(clippy::cognitive_complexity)]

#[path = "acp/config.rs"]
pub(crate) mod acp_config;
#[path = "acp/driver.rs"]
pub mod acpx_driver;
pub mod assets;
pub mod benchmark;
pub mod cli;
pub mod community;
pub mod config;
pub mod db;
pub mod decay;
pub mod error;
pub mod graph;
pub mod markdown;
// T-12.0: RTM NFR-1 — preserve the crate::mcp facade while the implementation lives in src/mcp/.
pub mod mcp;
pub mod memory;
// T-12.6: RTM NFR-1 - preserve the public utility surface while moving helper modules under util/.
pub mod util;
pub use util::json;
pub use util::paths;
pub mod registry;
// T-12.2: RTM FR-6 / FR-2 - expose extracted repair/analysis module.
pub mod repair;
pub mod search;
pub mod sessions;
pub mod similar;
pub mod sync;
pub mod thinking;
pub mod tools;
// T-29.2: RTM FR-35 — system tray icon for --http mode.
pub mod tray;
pub mod vector;
pub mod workflow;

pub use assets::{AssetCatalog, AssetKind, AssetManager};
pub use cli::{CliAction, CliApp};
pub use config::Config;
pub use db::GraphDatabase;
pub use error::{MaenifoldError, Result};
pub use mcp::{McpToolDefinition, list_mcp_tools};
pub use registry::{ToolDescriptor, ToolRegistry};
