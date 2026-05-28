use super::{ConsoleProtocolError, WINDOW_MAENIFOLD_INIT_SCRIPT, resolve_console_asset};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn write_fixture(root: &Path, relative_path: &str, contents: &[u8]) {
    let path = root.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create fixture parent");
    }
    fs::write(path, contents).expect("write fixture");
}

#[test]
fn resolve_console_asset_returns_index_html() {
    let root = TempDir::new().expect("create temp dir");
    let expected = b"<html>console</html>";
    write_fixture(root.path(), "index.html", expected);

    let (bytes, mime_type) = resolve_console_asset(root.path(), "index.html").expect("resolve index.html");

    assert_eq!(bytes, expected);
    assert_eq!(mime_type, "text/html");
}

#[test]
fn resolve_console_asset_returns_js_asset() {
    let root = TempDir::new().expect("create temp dir");
    let expected = b"console.log('ready');";
    write_fixture(root.path(), "assets/index-abc.js", expected);

    let (bytes, mime_type) =
        resolve_console_asset(root.path(), "assets/index-abc.js").expect("resolve javascript asset");

    assert_eq!(bytes, expected);
    assert_eq!(mime_type, "application/javascript");
}

#[test]
fn resolve_console_asset_rejects_parent_dir_traversal() {
    let root = TempDir::new().expect("create temp dir");

    let error = resolve_console_asset(root.path(), "../Cargo.toml").expect_err("reject parent traversal");

    assert!(matches!(error, ConsoleProtocolError::PathTraversal));
}

#[test]
fn resolve_console_asset_rejects_embedded_parent_traversal() {
    let root = TempDir::new().expect("create temp dir");

    let error =
        resolve_console_asset(root.path(), "assets/../../etc/passwd").expect_err("reject embedded traversal");

    assert!(matches!(error, ConsoleProtocolError::PathTraversal));
}

#[test]
fn resolve_console_asset_returns_not_found_for_missing_file() {
    let root = TempDir::new().expect("create temp dir");

    let error = resolve_console_asset(root.path(), "missing.html").expect_err("missing asset should fail");

    assert!(matches!(error, ConsoleProtocolError::NotFound));
}

#[test]
fn mime_type_dispatch_covers_required_extensions() {
    let root = TempDir::new().expect("create temp dir");
    let cases = [
        ("index.html", "text/html"),
        ("assets/app.js", "application/javascript"),
        ("assets/app.css", "text/css"),
        ("assets/font.woff2", "font/woff2"),
        ("assets/image.png", "image/png"),
        ("assets/logo.svg", "image/svg+xml"),
        ("assets/data.json", "application/json"),
        ("assets/blob.xyz", "application/octet-stream"),
    ];

    for (relative_path, expected_mime) in cases {
        write_fixture(root.path(), relative_path, b"fixture");
        let (_, mime_type) =
            resolve_console_asset(root.path(), relative_path).expect("fixture asset should resolve");
        assert_eq!(mime_type, expected_mime, "wrong mime for {relative_path}");
    }
}

#[test]
fn window_maenifold_init_script_defines_required_surface() {
    assert!(WINDOW_MAENIFOLD_INIT_SCRIPT.contains("window.maenifold = {"));
    assert!(WINDOW_MAENIFOLD_INIT_SCRIPT.contains("window.ipc.postMessage"));
    assert!(WINDOW_MAENIFOLD_INIT_SCRIPT.contains("invoke(tool, args)"));
    assert!(WINDOW_MAENIFOLD_INIT_SCRIPT.contains("subscribe(channel, cb)"));
    assert!(WINDOW_MAENIFOLD_INIT_SCRIPT.contains("window.__maenifoldDispatch"));
}
