use crate::error::GateError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskStageRecord {
    pub stage: String,
    pub record_kind: String,
    pub outcome: Option<String>,
    pub agent: String,
    pub model: Option<String>,
    pub recorded_at: String,
    pub notes: String,
}

#[derive(Debug, Clone)]
pub struct TaskSectionParser;

impl TaskSectionParser {
    pub fn parse(lines: &[&str]) -> Result<Vec<TaskStageRecord>, GateError> {
        let mut records = Vec::new();
        let mut index = 0;

        while index < lines.len() {
            if !lines[index].starts_with("## ") {
                index += 1;
                continue;
            }

            index += 1;
            let mut stage = None;
            let mut record_kind = None;
            let mut outcome = None;
            let mut agent = None;
            let mut model = None;
            let mut recorded_at = None;
            let mut notes = None;
            let mut saw_gate_field = false;

            while index < lines.len() && !lines[index].starts_with("## ") {
                if let Some(rest) = lines[index].strip_prefix("- ")
                    && let Some((label, value)) = rest.split_once(':')
                {
                    let value = value.trim().to_string();
                    match label.trim() {
                        "Stage ID" => {
                            saw_gate_field = true;
                            stage = Some(value);
                        }
                        "Record Kind" => {
                            saw_gate_field = true;
                            record_kind = Some(value);
                        }
                        "Agent" => {
                            saw_gate_field = true;
                            agent = Some(value);
                        }
                        "Recorded At" => {
                            saw_gate_field = true;
                            recorded_at = Some(value);
                        }
                        "Model" => {
                            saw_gate_field = true;
                            model = Some(value);
                        }
                        "Outcome" => {
                            saw_gate_field = true;
                            outcome = Some(value.to_ascii_lowercase());
                        }
                        "Notes" => {
                            saw_gate_field = true;
                            notes = Some(value);
                        }
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
                    outcome,
                    agent: agent.ok_or_else(|| {
                        GateError::TaskMetadataParse("missing section Agent".to_string())
                    })?,
                    model,
                    recorded_at: recorded_at.ok_or_else(|| {
                        GateError::TaskMetadataParse("missing section Recorded At".to_string())
                    })?,
                    notes: notes.ok_or_else(|| {
                        GateError::TaskMetadataParse("missing section Notes".to_string())
                    })?,
                });
            } else if saw_gate_field {
                return Err(GateError::TaskMetadataParse(
                    "gate section is missing Stage ID".to_string(),
                ));
            }
        }

        Ok(records)
    }
}


#[cfg(test)]
mod tests {
    use super::TaskSectionParser;

    #[test]
    fn non_gate_section_produces_no_records() {
        let lines = vec![
            "## Acceptance Criteria",
            "",
            "1. valid login returns token",
            "2. invalid login returns 401",
        ];
        let records = TaskSectionParser::parse(&lines).expect("parse should succeed");
        assert!(records.is_empty());
    }

    #[test]
    fn missing_required_field_returns_error() {
        let lines = vec![
            "## SWE",
            "",
            "- Stage ID: swe",
            "- Record Kind: outcome",
            "- Recorded At: 2026-01-01T00:00:00Z",
            "- Notes: implemented",
        ];
        let result = TaskSectionParser::parse(&lines);
        assert!(result.is_err(), "missing Agent should cause an error");
    }

    #[test]
    fn multiple_sections_parse_independently() {
        let lines = vec![
            "## Pending",
            "",
            "- Stage ID: pending",
            "- Record Kind: outcome",
            "- Agent: pm",
            "- Recorded At: 2026-01-01T00:00:00Z",
            "- Outcome: PASS",
            "- Notes: spec approved",
            "",
            "## SWE",
            "",
            "- Stage ID: swe",
            "- Record Kind: append",
            "- Agent: swe-agent",
            "- Recorded At: 2026-01-02T00:00:00Z",
            "- Notes: implementation started",
        ];
        let records = TaskSectionParser::parse(&lines).expect("parse should succeed");
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].stage, "pending");
        assert_eq!(records[0].outcome.as_deref(), Some("pass"));
        assert_eq!(records[1].stage, "swe");
        assert_eq!(records[1].record_kind, "append");
        assert_eq!(records[1].outcome, None);
    }

    #[test]
    fn unknown_fields_are_silently_ignored() {
        let lines = vec![
            "## SWE",
            "",
            "- Stage ID: swe",
            "- Record Kind: outcome",
            "- Agent: swe-agent",
            "- Recorded At: 2026-01-01T00:00:00Z",
            "- Outcome: PASS",
            "- Custom: extra field",
            "- Notes: done",
        ];
        let records = TaskSectionParser::parse(&lines).expect("parse should succeed");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].agent, "swe-agent");
        assert_eq!(records[0].notes, "done");
    }

    #[test]
    fn mixed_gate_and_non_gate_sections() {
        let lines = vec![
            "## Acceptance Criteria",
            "",
            "1. valid login returns token",
            "",
            "## Pending",
            "",
            "- Stage ID: pending",
            "- Record Kind: outcome",
            "- Agent: pm",
            "- Recorded At: 2026-01-01T00:00:00Z",
            "- Outcome: PASS",
            "- Notes: spec approved",
        ];
        let records = TaskSectionParser::parse(&lines).expect("parse should succeed");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].stage, "pending");
    }
}
