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
