use super::params::PipelineNodeSchema;
use super::support::schema_value;
use serde_json::Value;

pub(super) fn pipeline_node_schema() -> Value {
    schema_value::<PipelineNodeSchema>()
}
