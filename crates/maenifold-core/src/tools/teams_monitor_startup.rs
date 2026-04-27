use crate::acp_config::TeamsMonitorConfig;
use crate::error::Result;
use crate::teams::set_teams_error;
use std::path::{Path, PathBuf};

use super::teams_monitor::start_monitor_supervisor;
use super::teams_monitor_auth::acquire_token_for_monitor;

pub fn restore_monitor(monitor: &TeamsMonitorConfig, maenifold_root: &Path) -> Result<()> {
    let monitor = monitor.clone();
    let root: PathBuf = maenifold_root.to_path_buf();
    std::thread::Builder::new()
        .name("teams-restore".into())
        .spawn(move || {
            let mut monitor = monitor;
            match acquire_token_for_monitor(&mut monitor) {
                Ok(token) => {
                    if let Err(e) = start_monitor_supervisor(&monitor, token, &root) {
                        set_teams_error(format!("Teams restore supervisor: {e}"));
                    }
                }
                Err(e) => set_teams_error(e.to_string()),
            }
        })
        .map_err(|e| crate::error::MaenifoldError::Message(format!("spawn restore: {e}")))?;
    Ok(())
}
