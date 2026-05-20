use super::ort_runtime_download::{
    download_runtime_inner, extract_runtime_dylibs, stage_and_extract_checked,
};
use flate2::{Compression, write::GzEncoder};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use tar::{Builder, Header};

#[test]
fn ort_runtime_extractor_keeps_only_dylibs() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let archive = fixture_archive();

    extract_runtime_dylibs(&archive, temp.path()).expect("extract dylibs");

    assert!(temp.path().join("libonnxruntime.dylib").is_file());
    assert!(temp.path().join("libonnxruntime.1.24.0.dylib").is_file());
    assert!(!temp.path().join("README.md").exists());
    assert!(!temp.path().join("LICENSE").exists());
    assert!(!temp.path().join("onnxruntime_c_api.h").exists());
    assert_eq!(
        list_relative_files(temp.path()),
        vec![
            PathBuf::from("libonnxruntime.1.24.0.dylib"),
            PathBuf::from("libonnxruntime.dylib"),
        ]
    );
}

#[test]
fn ort_runtime_download_contract_cleans_partial_archive_on_sha_mismatch() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let archive = fixture_archive();

    let error = stage_and_extract_checked(&archive, "00", temp.path()).expect_err("sha mismatch");

    assert!(error.contains("SHA-256 verification failed"));
    assert!(!temp.path().join(".ort-runtime-download.tar.gz").exists());
    assert_eq!(list_relative_files(temp.path()), Vec::<PathBuf>::new());
}

fn fixture_archive() -> Vec<u8> {
    let gz = GzEncoder::new(Vec::new(), Compression::default());
    let mut builder = Builder::new(gz);
    append_bytes(
        &mut builder,
        "onnxruntime-osx-arm64-1.24.0/lib/libonnxruntime.dylib",
        b"canonical dylib",
    );
    append_bytes(
        &mut builder,
        "onnxruntime-osx-arm64-1.24.0/lib/libonnxruntime.1.24.0.dylib",
        b"versioned dylib",
    );
    append_bytes(
        &mut builder,
        "onnxruntime-osx-arm64-1.24.0/include/onnxruntime_c_api.h",
        b"header",
    );
    append_bytes(
        &mut builder,
        "onnxruntime-osx-arm64-1.24.0/README.md",
        b"readme",
    );
    append_bytes(
        &mut builder,
        "onnxruntime-osx-arm64-1.24.0/LICENSE",
        b"license",
    );
    let gz = builder.into_inner().expect("finish tar");
    gz.finish().expect("finish gzip")
}

fn append_bytes(builder: &mut Builder<GzEncoder<Vec<u8>>>, path: &str, bytes: &[u8]) {
    let mut header = Header::new_gnu();
    header.set_path(path).expect("set path");
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append(&header, Cursor::new(bytes))
        .expect("append fixture");
}

#[test]
fn ort_runtime_download_does_not_truncate_existing_dylib_on_extraction_failure() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let existing_content = b"existing-good-dylib";
    let dylib_path = temp.path().join("libonnxruntime.dylib");
    fs::write(&dylib_path, existing_content).expect("pre-create dylib");

    // Build a malformed archive that will pass sha256 but fail during
    // tar entry iteration (truncated gzip stream).
    let bad_archive = fixture_corrupt_archive();
    let sha = sha2_hex(&bad_archive);

    let result = stage_and_extract_checked(&bad_archive, &sha, temp.path());

    assert!(result.is_err(), "extraction should fail on corrupt archive");
    let after = fs::read(&dylib_path).expect("dylib still readable");
    assert_eq!(
        after.as_slice(),
        existing_content,
        "existing dylib must be untouched after failed extraction"
    );
}

#[test]
fn ort_runtime_download_concurrent_invocations_do_not_corrupt() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let archive = fixture_archive();
    let sha = sha2_hex(&archive);

    let handles: Vec<_> = (0..4)
        .map(|_| {
            let archive = archive.clone();
            let sha = sha.clone();
            let dir = temp.path().to_path_buf();
            std::thread::spawn(move || {
                stage_and_extract_checked(&archive, &sha, &dir)
            })
        })
        .collect();

    let results: Vec<_> = handles.into_iter().map(|h| h.join().expect("thread")).collect();
    let successes: Vec<_> = results.iter().filter(|r| r.is_ok()).collect();
    assert!(
        !successes.is_empty(),
        "at least one concurrent call must succeed"
    );

    let content = fs::read(temp.path().join("libonnxruntime.dylib")).expect("final dylib");
    assert_eq!(
        content,
        b"canonical dylib",
        "final dylib content must be valid fixture data"
    );
}

/// Proves the strict refuse-if-exists contract under concurrency via
/// the public-API-level `download_runtime_inner`. Exactly one caller
/// succeeds; all others get the "already installed" error.
#[test]
fn ort_runtime_download_refuse_if_exists_strict_under_concurrency() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let archive = fixture_archive();
    let sha = sha2_hex(&archive);
    let dylib_filename = "libonnxruntime.dylib";

    let handles: Vec<_> = (0..8)
        .map(|_| {
            let archive = archive.clone();
            let sha = sha.clone();
            let assets = temp.path().to_path_buf();
            let filename = dylib_filename.to_string();
            std::thread::spawn(move || {
                download_runtime_inner(&archive, &sha, &filename, &assets)
            })
        })
        .collect();

    let results: Vec<_> = handles.into_iter().map(|h| h.join().expect("thread")).collect();
    let successes: Vec<_> = results.iter().filter(|r| r.is_ok()).collect();
    let failures: Vec<_> = results.iter().filter(|r| r.is_err()).collect();

    assert_eq!(
        successes.len(),
        1,
        "exactly one concurrent caller must succeed, got {}",
        successes.len()
    );
    assert_eq!(
        failures.len(),
        7,
        "all other callers must be refused, got {} failures",
        failures.len()
    );

    for result in &results {
        if let Err(msg) = result {
            assert!(
                msg.contains("already installed"),
                "refuse error must say 'already installed', got: {msg}"
            );
        }
    }

    let content = fs::read(temp.path().join("lib/libonnxruntime.dylib"))
        .expect("final dylib must exist");
    assert_eq!(
        content,
        b"canonical dylib",
        "final dylib content must match fixture"
    );
}

fn fixture_corrupt_archive() -> Vec<u8> {
    let mut archive = fixture_archive();
    // Truncate the valid gzip stream — sha will match the truncated bytes,
    // but tar iteration will fail mid-stream.
    archive.truncate(archive.len() / 2);
    archive
}

fn sha2_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

fn list_relative_files(root: &Path) -> Vec<PathBuf> {
    let mut paths = fs::read_dir(root)
        .expect("read dir")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| path.is_file())
        .filter_map(|path| path.strip_prefix(root).ok().map(Path::to_path_buf))
        .collect::<Vec<_>>();
    paths.sort();
    paths
}
