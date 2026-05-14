//! Process-local asset-store lock.
//!
//! Held across workflow-asset validation/read AND the synchronous frontmatter
//! write that persists a pipeline `definition` referencing that asset, AND across
//! workflow-asset deletion (ref-check + unlink). Closes the create-vs-delete TOCTOU
//! race between writing a new pipeline session referencing workflow X and deleting
//! workflow X concurrently.
//!
//! Lock ordering contract: this lock is the OUTER lock. Do not acquire a blocking
//! pipeline session lease (`lease::with_exclusive_session_lock`) while holding it.
//! Non-blocking `session_lock_is_held` checks are fine.
//!
//! Lock scope contract: drop the guard BEFORE emitting resource notifications
//! (`emit_pipeline_resource_updated`), since notification dispatch may run async/
//! block_on code and we MUST NOT hold a `std::sync::Mutex` across an `await`.

use crate::error::{MaenifoldError, Result};
use std::sync::{LazyLock, Mutex, MutexGuard};

static WORKFLOW_ASSET_STORE_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

pub(crate) fn lock_asset_store<'a>() -> Result<MutexGuard<'a, ()>> {
    WORKFLOW_ASSET_STORE_LOCK.lock().map_err(|_| {
        MaenifoldError::Message("Workflow asset store lock is poisoned".to_string())
    })
}
