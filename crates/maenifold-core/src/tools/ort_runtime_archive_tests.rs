use super::ort_runtime_download::{
    download_runtime_inner, extract_runtime_dylibs, stage_and_extract_checked,
};
use flate2::{Compression, write::GzEncoder};
use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};
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
    // Mach-O 64 LE header with filetype=MH_DYLIB (0x06) at byte offset 12.
    let macho_canonical = &macho64_header(0x06, b"cn");
    let macho_versioned = &macho64_header(0x06, b"vn");
    append_bytes(
        &mut builder,
        "onnxruntime-osx-arm64-1.24.0/lib/libonnxruntime.dylib",
        macho_canonical,
    );
    append_bytes(
        &mut builder,
        "onnxruntime-osx-arm64-1.24.0/lib/libonnxruntime.1.24.0.dylib",
        macho_versioned,
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
    let bad_archive = fixture_corrupt_archive();
    let sha = sha2_hex(&bad_archive);
    let result = stage_and_extract_checked(&bad_archive, &sha, temp.path());
    assert!(result.is_err(), "extraction should fail on corrupt archive");
    let after = fs::read(&dylib_path).expect("dylib still readable");
    assert_eq!(
        after.as_slice(),
        existing_content,
        "existing dylib must be untouched"
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
            std::thread::spawn(move || stage_and_extract_checked(&archive, &sha, &dir))
        })
        .collect();
    let results: Vec<_> = handles
        .into_iter()
        .map(|h| h.join().expect("thread"))
        .collect();
    assert!(
        !results
            .iter()
            .filter(|r| r.is_ok())
            .collect::<Vec<_>>()
            .is_empty()
    );
    let content = fs::read(temp.path().join("libonnxruntime.dylib")).expect("final dylib");
    assert_eq!(content, macho64_header(0x06, b"cn"));
}

/// Proves refuse-if-exists under concurrency via `download_runtime_inner`.
#[test]
fn ort_runtime_download_refuse_if_exists_strict_under_concurrency() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let archive = fixture_archive();
    let sha = sha2_hex(&archive);
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let archive = archive.clone();
            let sha = sha.clone();
            let assets = temp.path().to_path_buf();
            std::thread::spawn(move || {
                download_runtime_inner(&archive, &sha, "libonnxruntime.dylib", &assets)
            })
        })
        .collect();
    let results: Vec<_> = handles
        .into_iter()
        .map(|h| h.join().expect("thread"))
        .collect();
    let successes = results.iter().filter(|r| r.is_ok()).count();
    let failures = results.iter().filter(|r| r.is_err()).count();
    assert_eq!(successes, 1, "exactly one concurrent caller must succeed");
    assert_eq!(failures, 7);
    for result in &results {
        if let Err(msg) = result {
            assert!(msg.contains("already installed"));
        }
    }
    let content = fs::read(temp.path().join("lib/libonnxruntime.dylib")).expect("final dylib");
    assert_eq!(content, macho64_header(0x06, b"cn"));
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
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
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

/// Test (a): synthetic dSYM-collision — the real dylib (MH_DYLIB filetype) wins
/// over a later dSYM entry with Mach-O magic but MH_DSYM filetype.
#[test]
fn ort_runtime_extractor_prefers_real_dylib_over_later_dsym_collision() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let real_magic: Vec<u8> = macho64_header(0x06, b"rl").to_vec(); // MH_DYLIB
    // dSYM payload: valid Mach-O magic but filetype=MH_DSYM (0x0a).
    let dsym_bytes: Vec<u8> = macho64_header(0x0a, b"ds").to_vec();

    let archive = build_dsym_collision_archive(&real_magic, &dsym_bytes);
    let sha = sha2_hex(&archive);

    stage_and_extract_checked(&archive, &sha, temp.path()).expect("extraction must succeed");

    let dylib_path = temp.path().join("libonnxruntime.dylib");
    assert!(dylib_path.is_file(), "real dylib must exist");
    let content = fs::read(&dylib_path).expect("read dylib");
    assert_eq!(
        &content[..4],
        &[0xcf, 0xfa, 0xed, 0xfe],
        "first 4 bytes must be Mach-O magic, not dSYM payload"
    );
    // Byte 12-15 (filetype field) must be MH_DYLIB = 0x06 (LE u32).
    assert_eq!(
        &content[12..16],
        &[0x06, 0x00, 0x00, 0x00],
        "filetype field must be MH_DYLIB (0x06), proving dSYM was rejected"
    );
    // No dSYM files should be present anywhere in target dir.
    let files = list_relative_files(temp.path());
    for f in &files {
        assert!(
            !f.to_string_lossy().contains(".dSYM"),
            "no dSYM files should be extracted, found: {f:?}"
        );
    }
}

/// Mach-O dylib filetype validation is macOS-specific. Production
/// `validate_staged_dylib_magic` is `#[cfg(target_os="macos")]`, so the test must
/// be too — on Windows/Linux the validator is a no-op and extraction succeeds.
#[cfg(target_os = "macos")]
#[test]
fn ort_runtime_extractor_rejects_non_macho_dylib_bytes() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let bad_bytes = b"NOT-A-DYLIB-CONTENT-HERE";

    let archive = build_single_dylib_archive(bad_bytes);
    let sha = sha2_hex(&archive);

    let result = stage_and_extract_checked(&archive, &sha, temp.path());
    assert!(result.is_err(), "non-Mach-O bytes must be rejected");
    let err = result.unwrap_err();
    // "NOT-" = 0x4e 0x4f 0x54 0x2d
    assert!(
        err.contains("0x4e4f542d"),
        "error must contain hex of actual bytes, got: {err}"
    );
    // Final dylib must NOT exist (staging cleaned up by TempDir Drop).
    assert!(
        !temp.path().join("libonnxruntime.dylib").is_file(),
        "rejected dylib must not persist in target dir"
    );
}

/// Test (c): real-tarball smoke — #[ignore]'d; PM runs manually.
#[test]
#[ignore]
fn ort_runtime_extractor_real_microsoft_tarball_smoke() {
    let tarball_path = format!(
        "{}/Downloads/onnxruntime-osx-arm64-1.24.1.tgz",
        std::env::var("HOME").unwrap()
    );
    if !Path::new(&tarball_path).is_file() {
        return;
    }
    let temp = tempfile::TempDir::new().expect("temp dir");
    let archive_bytes = fs::read(&tarball_path).expect("read tarball");
    let real_sha = sha2_hex(&archive_bytes);
    stage_and_extract_checked(&archive_bytes, &real_sha, temp.path())
        .expect("real tarball extraction must succeed");
    let content = fs::read(temp.path().join("libonnxruntime.dylib")).expect("read dylib");
    let magic = u32::from_ne_bytes([content[0], content[1], content[2], content[3]]);
    let valid = [
        0xfeedfacfu32,
        0xcffaedfe,
        0xfeedface,
        0xcefaedfe,
        0xcafebabe,
        0xbebafeca,
    ];
    assert!(
        valid.contains(&magic),
        "must have Mach-O magic, got: 0x{magic:08x}"
    );
    let size = content.len();
    assert!(
        size > 30_000_000 && size < 40_000_000,
        "expected ~35 MB, got {size}"
    );
}

fn build_dsym_collision_archive(real_bytes: &[u8], dsym_bytes: &[u8]) -> Vec<u8> {
    let gz = GzEncoder::new(Vec::new(), Compression::default());
    let mut builder = Builder::new(gz);
    append_bytes(&mut builder, "pkg/lib/libonnxruntime.dylib", real_bytes);
    append_bytes(
        &mut builder,
        "pkg/lib/libonnxruntime.dylib.dSYM/Contents/Resources/DWARF/libonnxruntime.dylib",
        dsym_bytes,
    );
    let gz = builder.into_inner().expect("finish tar");
    gz.finish().expect("finish gzip")
}

#[cfg(target_os = "macos")]
fn build_single_dylib_archive(content: &[u8]) -> Vec<u8> {
    let gz = GzEncoder::new(Vec::new(), Compression::default());
    let mut builder = Builder::new(gz);
    append_bytes(&mut builder, "pkg/lib/libonnxruntime.dylib", content);
    let gz = builder.into_inner().expect("finish tar");
    gz.finish().expect("finish gzip")
}

/// Constructs a synthetic 16-byte Mach-O 64-bit LE header with the given
/// filetype and a 2-byte tag for distinguishing fixtures in assertions.
fn macho64_header(filetype: u32, tag: &[u8; 2]) -> [u8; 16] {
    let mut h = [0u8; 16];
    // Magic: Mach-O 64-bit LE (0xfeedfacf as LE bytes).
    h[0..4].copy_from_slice(&[0xcf, 0xfa, 0xed, 0xfe]);
    // cputype placeholder with tag for fixture identification.
    h[4] = tag[0];
    h[5] = tag[1];
    // filetype at offset 12 as LE u32.
    h[12..16].copy_from_slice(&filetype.to_le_bytes());
    h
}

/// Mach-O dylib filetype validation is macOS-specific (see
/// `ort_runtime_extractor_rejects_non_macho_dylib_bytes`). Gate to macOS only.
#[cfg(target_os = "macos")]
#[test]
fn ort_runtime_extractor_rejects_mh_dsym_filetype_at_lib_passing_path() {
    let t = tempfile::TempDir::new().expect("td");
    let a = build_single_dylib_archive(&macho64_header(0x0a, b"ds"));
    let e =
        stage_and_extract_checked(&a, &sha2_hex(&a), t.path()).expect_err("MH_DSYM must reject");
    assert!(
        e.contains("filetype 0xa"),
        "error must mention filetype 0xa, got: {e}"
    );
    assert!(
        !t.path().join("libonnxruntime.dylib").is_file(),
        "rejected file must not persist"
    );
}
