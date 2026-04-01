use crate::error::Result;
use crate::tools::ToolContext;
use crate::workflow::read_workflow_definition;

use super::session_io::NodeFile;

pub(super) fn validate_workflow_node_definitions(
    context: &ToolContext,
    nodes: &[super::PipelineNode],
) -> Result<()> {
    for node in nodes {
        if let Some(workflow_id) = node.workflow_id() {
            read_workflow_definition(&context.config, workflow_id)?;
        }
    }

    Ok(())
}

pub(super) fn workflow_node_file(node: &super::PipelineNode, session_id: String) -> NodeFile {
    NodeFile {
        session_id,
        node_id: node.id().to_string(),
        workflow_id: node.effective_workflow_id(),
    }
}
