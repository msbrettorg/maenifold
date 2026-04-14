pub(crate) mod global_mcp;
pub(crate) mod pool;

pub(crate) use pool::DbPool;

use crate::types::AppState;

pub(crate) fn with_db<T>(
    state: &AppState,
    f: impl FnOnce(&rusqlite::Connection) -> Result<T, String>,
) -> Result<T, String> {
    let conn = state.db.get().map_err(|e| e.to_string())?;
    f(&conn)
}
