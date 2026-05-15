use crate::error::Result;
use std::collections::{BTreeMap, HashMap};

use super::super::super::PredecessorOutput;

pub(super) fn apply_fork_predecessor_context_to_frontmatter(
    node_states_frontmatter: &mut BTreeMap<String, serde_yml::Value>,
    fork_root_predecessor_context: &HashMap<String, Vec<PredecessorOutput>>,
    node_id_map: &HashMap<String, String>,
) -> Result<()> {
    for (original_node_id, predecessor_outputs) in fork_root_predecessor_context {
        let forked_node_id = &node_id_map[original_node_id];
        if let Some(serde_yml::Value::Mapping(map)) =
            node_states_frontmatter.get_mut(forked_node_id)
        {
            map.insert(
                serde_yml::Value::String("predecessorContext".to_string()),
                serde_yml::to_value(predecessor_outputs)?,
            );
        }
    }
    Ok(())
}
