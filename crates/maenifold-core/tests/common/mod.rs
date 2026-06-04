pub mod gateway_http;

pub fn locate_python3() -> String {
    use std::sync::OnceLock;
    static PYTHON: OnceLock<String> = OnceLock::new();
    PYTHON
        .get_or_init(|| {
            let candidates: &[&str] = if cfg!(windows) {
                &["python3.exe", "python.exe", "python3", "python"]
            } else {
                &["python3", "python"]
            };
            let path = std::env::var("PATH").unwrap_or_default();
            let separator = if cfg!(windows) { ';' } else { ':' };
            for dir in path.split(separator) {
                if dir.is_empty() {
                    continue;
                }
                for candidate in candidates {
                    let path = std::path::Path::new(dir).join(candidate);
                    if path.is_file() {
                        return path.to_string_lossy().to_string();
                    }
                }
            }
            panic!("no python3/python interpreter on PATH");
        })
        .clone()
}
