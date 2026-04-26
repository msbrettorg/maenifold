// T-01.16: facade over the split acp/config/* submodules. The original
// 717 LOC file was decomposed into types/settings/loader/tests; the public
// surface (re-exported here) is unchanged.
//
// We use explicit `#[path]` attributes because the parent module is itself
// mounted via `#[path = "acp/config.rs"]` in lib.rs, and the implicit
// non-mod.rs submodule directory resolution interacts with that mount in a
// way that differs from the conventional `<stem>/` lookup.

#[path = "config/easter_eggs.rs"]
mod easter_eggs;
#[path = "config/gateway_settings.rs"]
mod gateway_settings;
#[path = "config/loader.rs"]
mod loader;
#[path = "config/settings.rs"]
mod settings;
#[path = "config/types.rs"]
mod types;

#[cfg(test)]
#[path = "config/gateway_settings_tests.rs"]
mod gateway_settings_tests;
#[cfg(test)]
#[path = "config/loader_tests.rs"]
mod loader_tests;
#[cfg(test)]
#[path = "config/tests.rs"]
mod tests;

#[allow(unused_imports)]
pub use easter_eggs::*;
#[allow(unused_imports)]
pub use gateway_settings::*;
pub use loader::AgentsConfig;
#[allow(unused_imports)]
pub use settings::{
    CommunitySettings, DebugSettings, DecaySettings, MaenifoldSettings, TeamsMonitorConfig,
    VectorSettings,
};
pub use types::{AgentEntry, McpServerEntry};
