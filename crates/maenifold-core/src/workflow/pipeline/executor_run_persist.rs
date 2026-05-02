use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, Mutex as StdMutex},
};

use tokio::sync::Mutex as AsyncMutex;
use tracing::warn;

use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;

use super::super::NodeStatus;
use super::super::session_io::{
    derive_pipeline_node_states, frontmatter_value, persist_pipeline_state, read_pipeline_session,
};

static PIPELINE_SESSION_PERSIST_LOCKS: LazyLock<StdMutex<HashMap<String, Arc<AsyncMutex<()>>>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));

pub(crate) async fn persist_active_node_session(
    context: &ToolContext,
    session_id: &str,
    node_id: &str,
    acp_session_id: Option<&str>,
    thinking_session_id: Option<&str>,
    model: Option<&str>,
) -> Result<()> {
    let lock = pipeline_session_persist_lock(session_id)?;
    let _guard = lock.lock().await;
    let (uri, mut frontmatter, pipeline_content) = read_pipeline_session(context, session_id)?;
    let raw_node_states = frontmatter_value(&frontmatter, "nodeStates")?;
    let mut node_states = derive_pipeline_node_states(context, &raw_node_states)?;
    let Some(state) = node_states.get_mut(node_id) else {
        return Err(MaenifoldError::Message(format!(
            "ERROR: Pipeline session missing node state for '{node_id}'"
        )));
    };
    if !matches!(state.status, NodeStatus::Active) {
        return Ok(());
    }
    let mut changed = false;
    if let Some(acp_session_id) = acp_session_id {
        match state.acp_session_id.clone() {
            Some(existing) if existing == acp_session_id => {}
            Some(existing) => {
                warn!(
                    session_id,
                    node_id,
                    previous_acp_session_id = %existing,
                    new_acp_session_id = %acp_session_id,
                    "updating active node ACP session id"
                );
                state.acp_session_id = Some(acp_session_id.to_string());
                changed = true;
            }
            None => {
                state.acp_session_id = Some(acp_session_id.to_string());
                changed = true;
            }
        }
    }
    if let Some(thinking_session_id) = thinking_session_id {
        match state.thinking_session_id.as_deref() {
            Some(existing) if existing == thinking_session_id => {}
            _ => {
                state.thinking_session_id = Some(thinking_session_id.to_string());
                changed = true;
            }
        }
    }
    if state.model.is_none() {
        state.model = model.map(str::to_string);
        changed |= state.model.is_some();
    }
    if !changed {
        return Ok(());
    }
    persist_pipeline_state(
        &mut frontmatter,
        &node_states,
        &uri,
        &pipeline_content,
        context,
    )
}

fn pipeline_session_persist_lock(session_id: &str) -> Result<Arc<AsyncMutex<()>>> {
    let mut locks = PIPELINE_SESSION_PERSIST_LOCKS.lock().map_err(|_| {
        MaenifoldError::Message("internal error: pipeline session persist lock was poisoned".into())
    })?;
    Ok(locks
        .entry(session_id.to_string())
        .or_insert_with(|| Arc::new(AsyncMutex::new(())))
        .clone())
}
