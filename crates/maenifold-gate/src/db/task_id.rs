/// Extracts task ID from filename.
/// "T-55.1-pipeline-definition-schema.md" → "T-55.1"
/// "some-other-file.md" → "some-other-file"
pub fn extract_task_id(filename: &str) -> String {
    let stem = filename.strip_suffix(".md").unwrap_or(filename);
    if stem.is_empty() {
        return filename.to_string();
    }
    if !stem.starts_with("T-") {
        return stem.to_string();
    }

    let bytes = stem.as_bytes();
    let (mut end, mut saw_digit) = (2, false);
    while let Some(&byte) = bytes.get(end) {
        match byte {
            b'0'..=b'9' => saw_digit = true,
            b'.' if saw_digit => {}
            _ => break,
        }
        end += 1;
    }

    while let Some(&byte) = bytes.get(end) {
        if byte.is_ascii_alphabetic() {
            end += 1;
        } else {
            break;
        }
    }

    if saw_digit {
        stem[..end].trim_end_matches('.').to_string()
    } else {
        stem.to_string()
    }
}
