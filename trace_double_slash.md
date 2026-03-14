# Tracing: How write_memory Handles "memory-path-tests//double-slash"

## Input Scenario:
- folder parameter: "memory-path-tests//double-slash"
- title: "Test Note"
- content: "# Test\n\nContent with [[concept]]"

## Step 1: Validation (access.rs:84-91)
```rust
if let Some(folder) = folder
    && let Some(error) = validate_relative_folder(folder)
{
    return error;  // Return PATH_VALIDATION_ERROR if validation fails
}
```

**validate_relative_folder** (support.rs:147-168):
- Checks if folder is empty ✓ (not empty)
- Checks if it's absolute ✓ (not absolute)
- **KEY:** Splits by "/" and filters empty segments: `.filter(|segment| !segment.is_empty())`
  - Input: "memory-path-tests//double-slash"
  - Split result: ["memory-path-tests", "", "double-slash"]
  - After filter: ["memory-path-tests", "double-slash"] ← Empty segment dropped!
  - No dot-only sequences found ✓
- **RESULT: PASSES VALIDATION** (no error returned)

## Step 2: URI Construction (access.rs:95-99)
```rust
let slug: String = slugify(&title).chars().take(255).collect();
let uri = match folder {
    Some(folder) if !folder.is_empty() => format!("memory://{folder}/{slug}"),
    _ => format!("memory://{slug}"),
};
```

- slug = "test-note" (from slugify("Test Note"))
- folder = "memory-path-tests//double-slash" (passed as-is, NOT normalized)
- **uri = "memory://memory-path-tests//double-slash/test-note"**
  - ⚠️ **DOUBLE SLASHES ARE PRESERVED IN THE URI**

## Step 3: URI to Filesystem Path (markdown/paths.rs:48-60)
```rust
pub fn uri_to_path(uri: &str, base_path: &Path) -> Result<PathBuf> {
    validate_uri(uri, "uri")?;
    let mut path = join_validated_relative_path(base_path, uri.trim_start_matches("memory://"))?;
    // ...append .md if needed
    Ok(path)
}
```

**join_validated_relative_path** (util/paths.rs:78-81):
```rust
pub fn join_validated_relative_path(base_path: &Path, input: &str) -> Result<PathBuf> {
    let relative = validate_relative_path(input)?;  // Calls validate_relative_path
    join_validated_path_buf(base_path, &relative)
}
```

**validate_relative_path** (util/paths.rs:10-28):
```rust
pub fn validate_relative_path(input: &str) -> Result<PathBuf> {
    if Path::new(input).is_absolute() { ... }
    
    let decoded = urlencoding::decode(input)...;
    let normalized = decoded.replace('\\', "/");
    
    // Check for dot sequences by splitting
    for segment in normalized.split('/').filter(|segment| !segment.is_empty()) {
        // ... dot sequence check
    }
    
    Ok(PathBuf::from(normalized.trim_matches('/')))
}
```

- Input: "memory-path-tests//double-slash/test-note"
- Split('/') produces: ["memory-path-tests", "", "double-slash", "test-note"]
- Filter empty: ["memory-path-tests", "double-slash", "test-note"]
- No dots-only found ✓
- **PathBuf::from("memory-path-tests//double-slash/test-note")**
  - ✓ Returns the path WITH the double slashes preserved
  - trim_matches('/') only removes leading/trailing, NOT internal //

**join_validated_path_buf** (util/paths.rs:84-97):
```rust
pub fn join_validated_path_buf(base_path: &Path, relative: &Path) -> Result<PathBuf> {
    let joined = base_path.join(relative);
    // Symlink canonicalization checks...
    Ok(joined)
}
```

- base_path.join(relative) where relative = PathBuf("memory-path-tests//double-slash/test-note")
- PathBuf internally normalizes consecutive slashes in the OS layer

## Step 4: File Creation (markdown/storage.rs:40-87)
```rust
pub fn write_markdown(uri: &str, frontmatter: Option<&Frontmatter>, content: &str, config: &Config) -> Result<()> {
    let path = uri_to_path(uri, &config.memory_path)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, markdown)?;
}
```

- path = /memory_root/memory-path-tests//double-slash/test-note.md
- fs::create_dir_all(parent) → creates directory structure
  - **Rust's fs module normalizes the path before I/O**
  - Double slashes are treated equivalently to single slashes by the OS/filesystem
- fs::write() succeeds

## Result: Double Slashes Are NORMALIZED at Filesystem Level

1. **Validation ACCEPTS** "memory-path-tests//double-slash" (empty segments are filtered)
2. **URI preserves** the double slashes: "memory://memory-path-tests//double-slash/test-note"
3. **Filesystem normalizes** them automatically when writing:
   - /memory_root/memory-path-tests//double-slash/test-note.md
   - Becomes: /memory_root/memory-path-tests/double-slash/test-note.md
   - Created file is identical to what you'd get with single slashes

## Key Findings:

✓ NO sanitization of folder names (only titles are sanitized via sanitize_user_input())
✓ Double slashes are NOT normalized by write_memory
✓ Filesystem normalizes them during file I/O
✓ Reading the file back works correctly (filesystem resolves to the right file)
✓ slugify_title is applied ONLY to titles, NOT folders

