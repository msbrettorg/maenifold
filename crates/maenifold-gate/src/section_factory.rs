use chrono::Utc;

#[derive(Debug, Clone)]
pub struct CanonicalSectionFactory;

impl CanonicalSectionFactory {
    pub fn build_append_section(stage: &str, agent: &str, notes: &str, model: Option<&str>) -> String {
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
        let heading = canonical_heading(stage);
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

fn canonical_heading(stage: &str) -> String {
    match stage {
        "swe" => "SWE Remediation".to_string(),
        "red" => "Red-team Audit".to_string(),
        "blue" => "Blue-team Verification".to_string(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        }
    }
}
