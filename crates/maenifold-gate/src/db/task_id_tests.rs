use super::extract_task_id;

#[test]
fn extract_id_from_standard_name() {
    assert_eq!(extract_task_id("T-55.1-foo.md"), "T-55.1");
}

#[test]
fn extract_id_from_dotless_name() {
    assert_eq!(extract_task_id("T-55-foo.md"), "T-55");
}

#[test]
fn extract_id_from_non_task_name() {
    assert_eq!(extract_task_id("some-file.md"), "some-file");
}

#[test]
fn extract_id_empty_string() {
    assert_eq!(extract_task_id(""), "");
}

#[test]
fn extract_id_dot_md_only() {
    assert_eq!(extract_task_id(".md"), ".md");
}

#[test]
fn extract_id_no_extension() {
    assert_eq!(extract_task_id("T-55.1-foo"), "T-55.1");
}

#[test]
fn extract_id_triple_version() {
    assert_eq!(extract_task_id("T-1.2.3-foo.md"), "T-1.2.3");
}

#[test]
fn extract_task_id_trailing_alpha() {
    assert_eq!(extract_task_id("T-44.5b-foo.md"), "T-44.5b");
    assert_eq!(extract_task_id("T-44.5bc-foo.md"), "T-44.5bc");
    assert_eq!(extract_task_id("T-44.5-foo.md"), "T-44.5");
}
