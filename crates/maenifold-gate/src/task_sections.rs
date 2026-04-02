use crate::error::GateError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskStageRecord {
    pub stage: String,
    pub record_kind: String,
    pub verdict: Option<String>,
    pub agent: String,
    pub model: Option<String>,
    pub recorded_at: String,
}

#[derive(Debug, Clone)]
pub struct TaskSectionParser;

impl TaskSectionParser {
    pub fn parse(lines: &[&str]) -> Result<Vec<TaskStageRecord>, GateError> {
        let mut records = Vec::new();
        let mut index = 0;

        while index < lines.len() {
            let Some(_heading) = lines[index].strip_prefix("## ") else {
                index += 1;
                continue;
            };

            index += 1;
            let mut stage = None;
            let mut record_kind = None;
            let mut verdict = None;
            let mut agent = None;
            let mut model = None;
            let mut recorded_at = None;

            while index < lines.len() && !lines[index].starts_with("## ") {
                if let Some(rest) = lines[index].strip_prefix("- ") {
                    let Some((label, value)) = rest.split_once(':') else {
                        return Err(GateError::TaskMetadataParse(format!(
                            "invalid section line: {}",
                            lines[index]
                        )));
                    };
                    let value = value.trim().to_string();
                    match label.trim() {
                        "Stage ID" => stage = Some(value),
                        "Record Kind" => record_kind = Some(value),
                        "Agent" => agent = Some(value),
                        "Recorded At" | "Date" => recorded_at = Some(value),
                        "Model" => model = Some(value),
                        "Verdict" => verdict = Some(value.to_ascii_lowercase()),
                        _ => {}
                    }
                }
                index += 1;
            }

            if let Some(stage) = stage {
                records.push(TaskStageRecord {
                    stage,
                    record_kind: record_kind.ok_or_else(|| {
                        GateError::TaskMetadataParse("missing section Record Kind".to_string())
                    })?,
                    verdict,
                    agent: agent.ok_or_else(|| {
                        GateError::TaskMetadataParse("missing section Agent".to_string())
                    })?,
                    model,
                    recorded_at: recorded_at.ok_or_else(|| {
                        GateError::TaskMetadataParse("missing section Recorded At".to_string())
                    })?,
                });
            }
        }

        Ok(records)
    }
}
