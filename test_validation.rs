// Test to validate the validation logic
use std::path::Path;

// Simplified version of validate_relative_folder
fn validate_relative_folder(folder: &str) -> Option<String> {
    if folder.is_empty() {
        return None;
    }

    if Path::new(folder).is_absolute() {
        return Some("Absolute paths are not allowed. Use relative paths only.".to_string());
    }

    let decoded = folder.to_string();
    let normalized = decoded.replace('\\', "/");
    for segment in normalized.split('/').filter(|segment| !segment.is_empty()) {
        let compact: String = segment.chars().filter(|c| !c.is_whitespace()).collect();
        if !compact.is_empty() && compact.chars().all(|c| c == '.') {
            return Some("Path traversal components (dot sequences) are not allowed.".to_string());
        }
    }

    None
}

fn main() {
    let test_cases = vec![
        "memory-path-tests//double-slash",
        "memory-path-tests/double-slash",
        "path//with////multiple",
        "valid/nested/path",
    ];
    
    for case in test_cases {
        println!("Testing: '{}'", case);
        match validate_relative_folder(case) {
            None => println!("  ✓ VALID"),
            Some(err) => println!("  ✗ REJECTED: {}", err),
        }
        println!("  Segments: {:?}", case.split('/').collect::<Vec<_>>());
        println!();
    }
}
