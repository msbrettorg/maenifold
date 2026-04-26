use crate::{
    error::GateError, gate_definition::GateDefinition, task_file::TaskDocument,
    task_sections::TaskStageRecord,
};

pub use maenifold_gate_core::task_status::{
    EffectiveTaskState, TaskDocumentLike, TaskStageRecordLike,
};

#[derive(Debug, Clone)]
pub struct TaskStatusDeriver;

impl TaskStatusDeriver {
    pub fn derive(
        document: &TaskDocument,
        definition: &GateDefinition,
        location: &str,
    ) -> Result<EffectiveTaskState, GateError> {
        Ok(maenifold_gate_core::task_status::TaskStatusDeriver::derive(
            document,
            definition.as_core(),
            location,
        )?)
    }
}

impl TaskStageRecordLike for TaskStageRecord {
    fn stage(&self) -> &str {
        &self.stage
    }

    fn record_kind(&self) -> &str {
        &self.record_kind
    }

    fn outcome(&self) -> Option<&str> {
        self.outcome.as_deref()
    }
}

impl TaskDocumentLike for TaskDocument {
    type Record = TaskStageRecord;

    fn metadata_state(&self) -> Option<&str> {
        self.metadata.state.as_deref()
    }

    fn records(&self) -> &[Self::Record] {
        &self.records
    }
}
