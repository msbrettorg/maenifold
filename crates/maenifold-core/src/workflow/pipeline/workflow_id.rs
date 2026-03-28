use crate::error::{MaenifoldError, Result};

use super::PipelineNode;

pub(super) fn workflow_id_for_node(node: &PipelineNode) -> Result<&str> {
    node.workflow_id().ok_or_else(|| {
        MaenifoldError::Message(format!(
            "ERROR: Pipeline node '{}' of type '{}' does not have a workflowId",
            node.id(),
            node.node_type_name()
        ))
    })
}
