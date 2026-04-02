pub fn display_label(stage: &str) -> String {
    match stage {
        "swe" => "SWE".to_string(),
        "red" => "Red".to_string(),
        "blue" => "Blue".to_string(),
        "done" => "Done".to_string(),
        other => title_case(other),
    }
}

pub fn record_heading(stage: &str) -> String {
    match stage {
        "swe" => "SWE Remediation".to_string(),
        "red" => "Red-team Audit".to_string(),
        "blue" => "Blue-team Verification".to_string(),
        other => title_case(other),
    }
}

fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
