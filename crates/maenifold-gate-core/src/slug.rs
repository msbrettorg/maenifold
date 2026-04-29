pub fn task_title_slug(title: &str) -> String {
    let result = ::slug::slugify(title);
    if result.is_empty() {
        "untitled".to_string()
    } else {
        result
    }
}
