use chrono::Utc;

#[derive(Debug, Clone)]
pub struct CanonicalSectionFactory;

const STAGE_HEADINGS: &[(&str, &str)] = &[
    ("pending", "Pending"),
    ("swe", "SWE"),
    ("red-team", "Red-Team"),
    ("blue-team", "Blue-Team"),
    ("research", "Research"),
    ("plan", "Plan"),
    ("execute", "Execute"),
    ("verify", "Verify"),
    ("done", "Done"),
];

impl CanonicalSectionFactory {
    pub fn build_append_section(
        stage: &str,
        agent: &str,
        notes: &str,
        model: Option<&str>,
    ) -> String {
        Self::build(stage, "append", None, agent, notes, model)
    }

    pub fn build_outcome_section(
        stage: &str,
        outcome: &str,
        agent: &str,
        notes: &str,
        model: Option<&str>,
    ) -> String {
        Self::build(stage, "outcome", Some(outcome), agent, notes, model)
    }

    fn build(
        stage: &str,
        record_kind: &str,
        outcome: Option<&str>,
        agent: &str,
        notes: &str,
        model: Option<&str>,
    ) -> String {
        let heading = record_heading(stage);
        let mut section = format!(
            "## {heading}\n\n- Stage ID: {stage}\n- Record Kind: {record_kind}\n- Agent: {agent}\n- Recorded At: {}\n",
            Utc::now().to_rfc3339()
        );

        if let Some(model) = model {
            section.push_str(&format!("- Model: {model}\n"));
        }
        if let Some(outcome) = outcome {
            section.push_str(&format!("- Outcome: {}\n", outcome.to_uppercase()));
        }
        section.push_str(&format!("- Notes: {notes}\n"));
        section
    }
}

fn record_heading(stage: &str) -> String {
    if let Some((_, heading)) = STAGE_HEADINGS
        .iter()
        .find(|(candidate, _)| *candidate == stage)
    {
        return (*heading).to_string();
    }

    stage
        .split('-')
        .map(title_case_segment)
        .collect::<Vec<_>>()
        .join("-")
}

fn title_case_segment(segment: &str) -> String {
    let mut chars = segment.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let rest = chars.as_str().to_ascii_lowercase();
            first.to_uppercase().to_string() + &rest
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CanonicalSectionFactory;
    use crate::task_sections::TaskSectionParser;

    #[test]
    fn outcome_section_roundtrips_all_fields() {
        let section = CanonicalSectionFactory::build_outcome_section(
            "red-team", "pass", "red-team-agent", "all checks passed", Some("goldeneye"),
        );
        let lines: Vec<&str> = section.lines().collect();
        let records = TaskSectionParser::parse(&lines).expect("parse outcome section");
        assert_eq!(records.len(), 1);
        let record = &records[0];
        assert_eq!(record.stage, "red-team");
        assert_eq!(record.record_kind, "outcome");
        assert_eq!(record.outcome.as_deref(), Some("pass"));
        assert_eq!(record.agent, "red-team-agent");
        assert_eq!(record.model.as_deref(), Some("goldeneye"));
        assert_eq!(record.notes, "all checks passed");
        assert!(!record.recorded_at.is_empty());
    }

    #[test]
    fn outcome_section_without_model_roundtrips() {
        let section = CanonicalSectionFactory::build_outcome_section(
            "swe", "fail", "swe-agent", "build failed: missing import", None,
        );
        let lines: Vec<&str> = section.lines().collect();
        let records = TaskSectionParser::parse(&lines).expect("parse outcome without model");
        assert_eq!(records.len(), 1);
        let record = &records[0];
        assert_eq!(record.stage, "swe");
        assert_eq!(record.outcome.as_deref(), Some("fail"));
        assert_eq!(record.agent, "swe-agent");
        assert_eq!(record.model, None);
        assert_eq!(record.notes, "build failed: missing import");
    }

    #[test]
    fn append_section_roundtrips_all_fields() {
        let section = CanonicalSectionFactory::build_append_section(
            "pending", "pm", "spec reviewed and approved", Some("claude-opus-4.5"),
        );
        let lines: Vec<&str> = section.lines().collect();
        let records = TaskSectionParser::parse(&lines).expect("parse append section");
        assert_eq!(records.len(), 1);
        let record = &records[0];
        assert_eq!(record.stage, "pending");
        assert_eq!(record.record_kind, "append");
        assert_eq!(record.outcome, None);
        assert_eq!(record.agent, "pm");
        assert_eq!(record.model.as_deref(), Some("claude-opus-4.5"));
        assert_eq!(record.notes, "spec reviewed and approved");
    }
}

