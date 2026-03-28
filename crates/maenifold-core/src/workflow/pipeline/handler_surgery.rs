use crate::error::Result;
use crate::tools::ToolContext;
use serde::Deserialize;
use serde_json::Value;

#[path = "handler_surgery_fork.rs"]
mod fork;
#[path = "handler_surgery_graft.rs"]
mod graft;
#[path = "handler_surgery_join.rs"]
mod join;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JoinRequest {
    source_session_id: String,
    edges: Vec<super::PipelineEdge>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForkRequest {
    from_node_id: String,
    #[serde(default)]
    new_session_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GraftRequest {
    #[serde(default)]
    nodes: Vec<super::PipelineNode>,
    #[serde(default)]
    edges: Vec<super::PipelineEdge>,
}

pub(super) fn handle_surgery_operation(
    context: &ToolContext,
    session_id: &str,
    payload: &Value,
) -> Result<Option<String>> {
    if let Some(join_value) = payload.get("join") {
        let join: JoinRequest = serde_json::from_value(join_value.clone())?;
        return join::join_pipeline(context, session_id, join).map(Some);
    }
    if let Some(fork_value) = payload.get("fork") {
        let fork: ForkRequest = serde_json::from_value(fork_value.clone())?;
        return fork::fork_pipeline(context, session_id, fork).map(Some);
    }
    if let Some(graft_value) = payload.get("graft") {
        let graft: GraftRequest = serde_json::from_value(graft_value.clone())?;
        return graft::graft_pipeline(context, session_id, graft).map(Some);
    }

    Ok(None)
}
