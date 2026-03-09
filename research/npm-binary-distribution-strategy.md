# npm Binary Distribution Strategy for maenifold-rs

> **Research deliverable** — does not map to a current sprint T-* item. This is forward-looking
> architecture research for future npm packaging work. When a backlog item is created for npm
> distribution, it should reference this document and create proper FR-*/T-* items in PRD/RTM/TODO.

## Executive Summary

Modern Rust CLI tools distribute via npm using a **two-tier architecture**: platform-specific
binary packages (selected via `optionalDependencies` + `os`/`cpu` fields) and a thin JavaScript
dispatcher that resolves and executes the correct binary. For large assets like ONNX models
(86 MB), the industry pattern is **download-on-first-use** from GitHub Releases or a CDN, with
SHA-256 verification and progress feedback. This document recommends a concrete package structure
for `maenifold`, covering the ~10 MB Rust binary, 86 MB ONNX model, ~100-165 KB vec0 SQLite
extensions, and ~784 KB of JSON/YAML config assets.

---

## 1. The optionalDependencies Pattern (How the Industry Does It)

### 1.1 Architecture Overview

```
┌─────────────────────────────────────┐
│  @maenifold/cli  (main package)     │
│  ─────────────────────────────────  │
│  • bin/maenifold  (JS dispatcher)   │
│  • assets/ (JSON/YAML configs)      │
│  • lib/download.js (asset fetcher)  │
│  • package.json with                │
│    optionalDependencies →           │
├─────────────────────────────────────┤
│  @maenifold/cli-darwin-arm64        │  ← os:["darwin"], cpu:["arm64"]
│  @maenifold/cli-darwin-x64          │  ← os:["darwin"], cpu:["x64"]
│  @maenifold/cli-linux-x64          │  ← os:["linux"],  cpu:["x64"]
│  @maenifold/cli-linux-arm64        │  ← os:["linux"],  cpu:["arm64"]
│  @maenifold/cli-win32-x64          │  ← os:["win32"],  cpu:["x64"]
└─────────────────────────────────────┘
```

npm's package manager installs **all** optionalDependencies but silently skips any whose `os`/`cpu`
fields don't match the host. The result: only the correct platform binary lands in `node_modules`.

### 1.2 Real-World Package.json Examples (Verified from Source)

#### esbuild (Go binary, 26 platform packages)
```json
{
  "name": "esbuild",
  "version": "0.27.3",
  "bin": { "esbuild": "bin/esbuild" },
  "scripts": { "postinstall": "node install.js" },
  "optionalDependencies": {
    "@esbuild/darwin-arm64": "0.27.3",
    "@esbuild/darwin-x64": "0.27.3",
    "@esbuild/linux-arm64": "0.27.3",
    "@esbuild/linux-x64": "0.27.3",
    "@esbuild/win32-x64": "0.27.3"
    /* ... 21 more platforms ... */
  }
}
```
**Source**: [github.com/evanw/esbuild/npm/esbuild/package.json](https://github.com/evanw/esbuild/blob/main/npm/esbuild/package.json)

#### Biome (Rust binary, 8 platform packages)
```json
{
  "name": "@biomejs/biome",
  "version": "2.4.6",
  "bin": { "biome": "bin/biome" },
  "files": ["bin/biome", "configuration_schema.json", "README.md"],
  "optionalDependencies": {
    "@biomejs/cli-darwin-arm64": "2.4.6",
    "@biomejs/cli-darwin-x64": "2.4.6",
    "@biomejs/cli-linux-x64": "2.4.6",
    "@biomejs/cli-linux-arm64": "2.4.6",
    "@biomejs/cli-linux-x64-musl": "2.4.6",
    "@biomejs/cli-linux-arm64-musl": "2.4.6",
    "@biomejs/cli-win32-x64": "2.4.6",
    "@biomejs/cli-win32-arm64": "2.4.6"
  }
}
```
**Source**: [github.com/biomejs/biome/packages/@biomejs/biome/package.json](https://github.com/biomejs/biome/blob/main/packages/%40biomejs/biome/package.json)

#### Platform-Specific Package (Biome darwin-arm64)
```json
{
  "name": "@biomejs/cli-darwin-arm64",
  "version": "2.4.6",
  "os": ["darwin"],
  "cpu": ["arm64"]
}
```
Contains only the `biome` binary. The `os` and `cpu` fields cause npm to skip installation on
non-matching hosts.

**Source**: [github.com/biomejs/biome/packages/@biomejs/cli-darwin-arm64/package.json](https://github.com/biomejs/biome/blob/main/packages/%40biomejs/cli-darwin-arm64/package.json)

### 1.3 The JavaScript Dispatcher (bin/ entry point)

Biome's `bin/biome` is a **Node.js script** that resolves and spawns the native binary:

```javascript
#!/usr/bin/env node
const { platform, arch } = process;
const { execSync } = require("child_process");

function isMusl() {
  try {
    const stderr = execSync("ldd --version", { stdio: ['pipe','pipe','pipe'] });
    return stderr.indexOf("musl") > -1;
  } catch (err) {
    return err.stderr && err.stderr.indexOf("musl") > -1;
  }
}

const PLATFORMS = {
  win32:       { x64: "@biomejs/cli-win32-x64/biome.exe",
                 arm64: "@biomejs/cli-win32-arm64/biome.exe" },
  darwin:      { x64: "@biomejs/cli-darwin-x64/biome",
                 arm64: "@biomejs/cli-darwin-arm64/biome" },
  linux:       { x64: "@biomejs/cli-linux-x64/biome",
                 arm64: "@biomejs/cli-linux-arm64/biome" },
  "linux-musl":{ x64: "@biomejs/cli-linux-x64-musl/biome",
                 arm64: "@biomejs/cli-linux-arm64-musl/biome" },
};

const binPath = process.env.BIOME_BINARY ||
  (platform === "linux" && isMusl()
    ? PLATFORMS["linux-musl"]?.[arch]
    : PLATFORMS[platform]?.[arch]);

if (binPath) {
  const result = require("child_process").spawnSync(
    require.resolve(binPath),
    process.argv.slice(2),
    { shell: false, stdio: "inherit" }
  );
  process.exitCode = result.status;
} else {
  console.error("No prebuilt binary for your platform.");
  process.exitCode = 1;
}
```
**Source**: [github.com/biomejs/biome/packages/@biomejs/biome/bin/biome](https://github.com/biomejs/biome/blob/main/packages/%40biomejs/biome/bin/biome) (actual production code, verified)

### 1.4 esbuild's Fallback Strategy

esbuild adds a `postinstall` script (`install.js`) that:
1. **Validates** the installed binary matches the expected version/platform
2. **Optimizes** symlinks to point directly at the native binary (saves 100-200ms per CLI invocation vs JS shim)
3. **Falls back** to downloading directly from npm registry if `--no-optional` was used

This is a defense-in-depth approach. Most users never hit the fallback.

**Source**: [github.com/evanw/esbuild/issues/4288](https://github.com/evanw/esbuild/issues/4288)

---

## 2. On-Demand Asset Downloading

### 2.1 Why Not Bundle Everything?

| Asset | Size | Bundle in npm? |
|-------|------|----------------|
| Rust binary (per-platform) | ~10 MB (release, stripped) | ✅ Yes — in platform-specific package |
| vec0 SQLite extension (per-platform) | 100-165 KB | ✅ Yes — in platform-specific package |
| JSON/YAML configs | ~784 KB | ✅ Yes — in main package |
| tokenizer vocab.txt + config | ~232 KB | ✅ Yes — in main package |
| ONNX model (all-MiniLM-L6-v2) | **86 MB** | ❌ No — download on first use |

The npm registry has a practical package size limit of ~200 MB, but packages >50 MB create poor
install experiences. The ONNX model at 86 MB should be downloaded on demand.

### 2.2 Download-on-First-Use Pattern

The standard pattern, used by Prisma, Playwright, and others:

```
First run of `maenifold`:
  1. Check if model exists at $DATA_DIR/maenifold/models/all-MiniLM-L6-v2.onnx
  2. If missing → download with progress bar
  3. Verify SHA-256 checksum
  4. Cache permanently (survives npm reinstalls)
```

#### Storage Locations (Cross-Platform)

| Platform | Directory | Standard |
|----------|-----------|----------|
| Linux | `$XDG_DATA_HOME/maenifold/` (default: `~/.local/share/maenifold/`) | XDG Base Directory Spec |
| macOS | `~/Library/Application Support/maenifold/` | Apple standard |
| Windows | `%LOCALAPPDATA%\maenifold\` | Windows Known Folders |

The **Rust `directories` crate** (or `dirs-next`) provides these paths natively. On the JS side,
the `env-paths` npm package provides the same.

**Important**: Store downloaded assets **outside** `node_modules` so they survive reinstalls and
are shared across projects.

### 2.3 Download Source: GitHub Releases

GitHub Releases support up to **2 GB per asset** and serve via a global CDN. This is the
recommended approach:

```
Download URL pattern:
https://github.com/ma-collective/maenifold-rs/releases/download/v{VERSION}/all-MiniLM-L6-v2.onnx
```

**Advantages over HuggingFace for this use case**:
- No authentication required for public repos
- Same release tag as the npm package version = version coherence
- GitHub CDN has excellent global performance
- Release assets are immutable once published

**From existing knowledge graph** (memory://build/research/huggingface-hosting-...):
The model is also available from HuggingFace with an immutable commit-pinned URL:
```
https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/c9745ed1d9f207416be6d2e6f8de32d1f16199bf/onnx/model.onnx
```
SHA-256: `6fd5d72fe4589f189f8ebc006442dbb529bb7ce38f8082112682524616046452`

This can serve as a **fallback** download source.

### 2.4 Download Implementation (Rust-Side)

Since maenifold is a Rust binary, the download logic should live in **Rust**, not JavaScript.
The JS dispatcher just launches the binary; the binary handles its own asset management.

```rust
// Pseudocode for asset download check
fn ensure_model(config: &Config) -> Result<PathBuf> {
    let data_dir = dirs::data_dir()
        .ok_or(anyhow!("Cannot determine data directory"))?
        .join("maenifold")
        .join("models");

    let model_path = data_dir.join("all-MiniLM-L6-v2.onnx");

    if model_path.exists() {
        // Verify checksum on first use only (cache the check)
        return Ok(model_path);
    }

    fs::create_dir_all(&data_dir)?;

    eprintln!("Downloading embedding model (86 MB)...");
    let url = format!(
        "https://github.com/ma-collective/maenifold-rs/releases/download/v{}/all-MiniLM-L6-v2.onnx",
        env!("CARGO_PKG_VERSION")
    );

    download_with_progress(&url, &model_path)?;
    verify_sha256(&model_path, EXPECTED_SHA256)?;

    Ok(model_path)
}
```

### 2.5 Progress Feedback

For CLI progress, use the `indicatif` crate in Rust:

```rust
use indicatif::{ProgressBar, ProgressStyle};

let pb = ProgressBar::new(total_bytes);
pb.set_style(ProgressStyle::default_bar()
    .template("{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
    .progress_chars("#>-"));

// In download loop:
pb.inc(chunk.len() as u64);
```

### 2.6 Checksum Verification

```rust
use sha2::{Sha256, Digest};

fn verify_sha256(path: &Path, expected: &str) -> Result<()> {
    let mut hasher = Sha256::new();
    let mut file = File::open(path)?;
    io::copy(&mut file, &mut hasher)?;
    let actual = format!("{:x}", hasher.finalize());
    if actual != expected {
        fs::remove_file(path)?; // Clean up corrupt download
        anyhow::bail!("Checksum mismatch: expected {expected}, got {actual}");
    }
    Ok(())
}
```

---

## 3. Real-World Examples Analysis

### 3.1 Prisma (Rust engines, on-demand download)

**Pattern**: Rust query/migration engines downloaded at `postinstall` from Prisma's CDN.

- `@prisma/engines` package runs a postinstall that detects OS/arch/OpenSSL version
- Downloads ~30-40 MB of engine binaries to `node_modules/.prisma/`
- Supports `binaryTargets` config for cross-compilation scenarios
- Caches across projects via a global engine cache

**Key insight**: Prisma downloads at install time (postinstall), not at runtime. This ensures
offline operation after install but requires network during `npm install`.

**Source**: [deepwiki.com/prisma/prisma/4-engine-and-binary-management](https://deepwiki.com/prisma/prisma/4-engine-and-binary-management)

### 3.2 sqlite-vec (Native SQLite extension, GitHub Releases)

**Pattern**: Pre-built native extensions (vec0.so/dylib/dll) distributed via GitHub Releases.

- Alex Garcia's sqlite-vec uses GitHub Releases to host platform-specific binaries
- The npm package detects OS/arch and downloads the matching `.so`/`.dylib`/`.dll`
- Falls back to WebAssembly build for unsupported platforms
- Each release tags binaries for all supported platforms

**Key insight**: This is the **exact same asset type** maenifold needs for its vec0 extensions.
The extensions are small enough (~100-165 KB) to bundle directly in platform packages.

**Source**: [github.com/asg017/sqlite-vec](https://github.com/asg017/sqlite-vec),
[alexgarcia.xyz/sqlite-vec/installation.html](https://alexgarcia.xyz/sqlite-vec/installation.html)

### 3.3 Biome (Rust binary, optionalDependencies pattern)

**Pattern**: Pure optionalDependencies, no postinstall download, no large assets.

- 8 platform-specific packages under `@biomejs/cli-*`
- Each contains only the compiled Rust binary (~10-15 MB)
- JS dispatcher in `bin/biome` resolves the correct package at runtime
- No large model assets — simplest case

**Key insight**: This is the cleanest model for the binary distribution part. No fallback
download needed because the binary is always in an optionalDependency.

**Source**: Verified from actual source files above.

---

## 4. Recommended Package Structure for maenifold

### 4.1 Package Layout

```
npm/
├── maenifold/                          # Main package (@maenifold/cli or just 'maenifold')
│   ├── package.json
│   ├── bin/
│   │   └── maenifold                   # JS dispatcher (Node.js script)
│   ├── assets/                         # Bundled config assets (~784 KB)
│   │   ├── agents/
│   │   ├── colors/
│   │   ├── perspectives/
│   │   ├── roles/
│   │   ├── usage/
│   │   └── workflows/
│   ├── models/                         # Tokenizer only (~232 KB), NOT the ONNX model
│   │   ├── vocab.txt
│   │   └── tokenizer_config.json
│   └── lib/
│       └── assets.json                 # Manifest of downloadable assets with checksums
│
├── maenifold-darwin-arm64/             # Platform binary + vec0 extension
│   ├── package.json                    # os:["darwin"], cpu:["arm64"]
│   ├── bin/
│   │   └── maenifold                   # Compiled Rust binary (~10 MB stripped)
│   └── native/
│       └── vec0.dylib                  # SQLite vec0 extension (162 KB)
│
├── maenifold-darwin-x64/
│   ├── package.json                    # os:["darwin"], cpu:["x64"]
│   ├── bin/
│   │   └── maenifold
│   └── native/
│       └── vec0.dylib                  # (125 KB)
│
├── maenifold-linux-x64/
│   ├── package.json                    # os:["linux"], cpu:["x64"]
│   ├── bin/
│   │   └── maenifold
│   └── native/
│       └── vec0.so                     # (118 KB)
│
├── maenifold-linux-arm64/
│   ├── package.json                    # os:["linux"], cpu:["arm64"]
│   ├── bin/
│   │   └── maenifold
│   └── native/
│       └── vec0.so                     # (100 KB)
│
└── maenifold-win32-x64/
    ├── package.json                    # os:["win32"], cpu:["x64"]
    ├── bin/
    │   └── maenifold.exe
    └── native/
        └── vec0.dll                    # (165 KB)
```

### 4.2 Main package.json

```json
{
  "name": "maenifold",
  "version": "1.0.0",
  "description": "A knowledge graph and memory system for AI agents",
  "bin": {
    "maenifold": "bin/maenifold"
  },
  "files": [
    "bin/maenifold",
    "assets/**/*",
    "models/vocab.txt",
    "models/tokenizer_config.json",
    "lib/assets.json",
    "README.md",
    "LICENSE"
  ],
  "engines": {
    "node": ">=18"
  },
  "optionalDependencies": {
    "@maenifold/cli-darwin-arm64": "1.0.0",
    "@maenifold/cli-darwin-x64": "1.0.0",
    "@maenifold/cli-linux-x64": "1.0.0",
    "@maenifold/cli-linux-arm64": "1.0.0",
    "@maenifold/cli-win32-x64": "1.0.0"
  },
  "scripts": {
    "postinstall": "node lib/postinstall.js"
  },
  "license": "MIT"
}
```

### 4.3 Platform Package (e.g., darwin-arm64)

```json
{
  "name": "@maenifold/cli-darwin-arm64",
  "version": "1.0.0",
  "description": "macOS ARM64 binary for maenifold",
  "os": ["darwin"],
  "cpu": ["arm64"],
  "files": [
    "bin/maenifold",
    "native/vec0.dylib"
  ],
  "license": "MIT"
}
```

**Note**: No `bin` field in the platform package. The main package's JS dispatcher handles
execution routing.

### 4.4 JavaScript Dispatcher (`bin/maenifold`)

```javascript
#!/usr/bin/env node
const { platform, arch, env } = process;
const { spawnSync } = require("child_process");
const path = require("path");

const PLATFORMS = {
  darwin:  { arm64: "@maenifold/cli-darwin-arm64",
             x64:   "@maenifold/cli-darwin-x64" },
  linux:   { arm64: "@maenifold/cli-linux-arm64",
             x64:   "@maenifold/cli-linux-x64" },
  win32:   { x64:   "@maenifold/cli-win32-x64" },
};

const ext = platform === "win32" ? ".exe" : "";
const pkgName = PLATFORMS[platform]?.[arch];

if (!pkgName) {
  console.error(`Unsupported platform: ${platform}-${arch}`);
  console.error("Install from source: https://github.com/ma-collective/maenifold-rs");
  process.exitCode = 1;
  process.exit();
}

let binPath;
try {
  // Resolve the binary from the installed optional dependency
  const pkgDir = path.dirname(require.resolve(`${pkgName}/package.json`));
  binPath = path.join(pkgDir, "bin", `maenifold${ext}`);
} catch (e) {
  console.error(`Failed to find maenifold binary. Package ${pkgName} not installed.`);
  console.error("Try: npm install maenifold --force");
  process.exitCode = 1;
  process.exit();
}

// Pass through the assets directory location via env var
const mainPkgDir = path.resolve(__dirname, "..");
const assetsDir = path.join(mainPkgDir, "assets");
const modelsDir = path.join(mainPkgDir, "models");

// Resolve vec0 extension path from the platform package
const pkgDir = path.dirname(require.resolve(`${pkgName}/package.json`));
const nativeDir = path.join(pkgDir, "native");

const result = spawnSync(binPath, process.argv.slice(2), {
  shell: false,
  stdio: "inherit",
  env: {
    ...env,
    MAENIFOLD_ASSETS_DIR: assetsDir,
    MAENIFOLD_MODELS_DIR: modelsDir,
    MAENIFOLD_NATIVE_DIR: nativeDir,
  },
});

if (result.error) throw result.error;
process.exitCode = result.status;
```

### 4.5 Asset Manifest (`lib/assets.json`)

```json
{
  "version": "1.0.0",
  "assets": {
    "onnx-model": {
      "filename": "all-MiniLM-L6-v2.onnx",
      "size": 90405214,
      "sha256": "6fd5d72fe4589f189f8ebc006442dbb529bb7ce38f8082112682524616046452",
      "urls": [
        "https://github.com/ma-collective/maenifold-rs/releases/download/v{version}/all-MiniLM-L6-v2.onnx",
        "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/c9745ed1d9f207416be6d2e6f8de32d1f16199bf/onnx/model.onnx"
      ],
      "description": "Sentence embedding model for semantic search"
    }
  }
}
```

### 4.6 On-Demand Download Flow (In Rust Binary)

```
maenifold startup:
  ├─ Check MAENIFOLD_ASSETS_DIR env → resolve bundled config assets
  ├─ Check MAENIFOLD_NATIVE_DIR env → resolve vec0 extension
  ├─ Load vec0 SQLite extension from native dir
  └─ When semantic search is needed:
      ├─ Check MAENIFOLD_DATA_DIR or dirs::data_dir()/maenifold/
      ├─ If model exists → load
      └─ If missing:
          ├─ Print "Downloading embedding model (86 MB)..."
          ├─ Download from GitHub Releases (primary)
          ├─ Fallback: download from HuggingFace
          ├─ Verify SHA-256
          └─ Cache to data dir (persists across reinstalls)
```

### 4.7 Optional Postinstall (lib/postinstall.js)

A minimal postinstall to validate the binary is accessible and optionally pre-download the model:

```javascript
#!/usr/bin/env node
const { platform, arch } = process;
const path = require("path");
const { execSync } = require("child_process");

// Verify binary was installed correctly
const PLATFORMS = {
  darwin:  { arm64: "@maenifold/cli-darwin-arm64",
             x64:   "@maenifold/cli-darwin-x64" },
  linux:   { arm64: "@maenifold/cli-linux-arm64",
             x64:   "@maenifold/cli-linux-x64" },
  win32:   { x64:   "@maenifold/cli-win32-x64" },
};

const pkgName = PLATFORMS[platform]?.[arch];
if (!pkgName) {
  console.warn(`⚠ maenifold: no prebuilt binary for ${platform}-${arch}`);
  process.exit(0); // Don't fail install
}

try {
  require.resolve(`${pkgName}/package.json`);
  console.log(`✓ maenifold: binary installed for ${platform}-${arch}`);
} catch (e) {
  console.warn(`⚠ maenifold: binary package ${pkgName} not found.`);
  console.warn("  Semantic search features may download assets on first use.");
}
```

---

## 5. Size Budget Analysis

### Per-Platform Package Size
| Component | Size |
|-----------|------|
| Rust binary (release, stripped) | ~10 MB |
| vec0 extension | ~100-165 KB |
| package.json + metadata | < 1 KB |
| **Total per platform** | **~10.2 MB** |

### Main Package Size
| Component | Size |
|-----------|------|
| JSON/YAML config assets | ~784 KB |
| vocab.txt + tokenizer_config.json | ~232 KB |
| JS dispatcher + postinstall | ~5 KB |
| assets.json manifest | < 1 KB |
| README + LICENSE | ~10 KB |
| **Total main package** | **~1 MB** |

### User's Install Footprint
| Component | Size | When |
|-----------|------|------|
| Main package | ~1 MB | `npm install` |
| Platform binary + vec0 | ~10.2 MB | `npm install` (automatic) |
| ONNX model | ~86 MB | First use of semantic search |
| **Total at install** | **~11.2 MB** | |
| **Total after first search** | **~97.4 MB** | |

---

## 6. Decision Matrix: Bundle vs. Download

| Asset | Bundle | Download | Recommendation |
|-------|--------|----------|----------------|
| Rust binary | ✅ Fast, reliable | ❌ Fragile on --no-optional | **Bundle** in platform package |
| vec0 extension | ✅ Tiny (165KB), same platform | ❌ Overkill for 165KB | **Bundle** with binary |
| Config JSON/YAML | ✅ Tiny (784KB) | ❌ Overkill | **Bundle** in main package |
| Tokenizer vocab | ✅ Small (232KB) | ❌ Overkill | **Bundle** in main package |
| ONNX model | ❌ 86MB bloats install | ✅ Download only when needed | **Download** on first use |

---

## 7. Key Design Decisions

### 7.1 Use `@scope/` Packages (Recommended: `@maenifold/cli-*`)

Scoped packages prevent name collisions and make the relationship between packages clear.
If publishing to a GitHub Packages registry or private npm registry, scoped packages are required.

### 7.2 Download Logic Lives in Rust, Not JavaScript

Since maenifold is a Rust binary that manages its own configuration and data, the download
logic should be in Rust using `reqwest` + `indicatif` + `sha2`. The JS layer is just a thin
dispatcher. This means:
- The binary works the same whether installed via npm, cargo, or direct download
- No Node.js dependency for the download/verification logic
- Consistent behavior across all distribution channels

### 7.3 Use `dirs::data_dir()` for Model Cache

The Rust `directories` crate provides cross-platform standard directories. The ONNX model
should be stored at:
- Linux: `~/.local/share/maenifold/models/all-MiniLM-L6-v2.onnx`
- macOS: `~/Library/Application Support/maenifold/models/all-MiniLM-L6-v2.onnx`
- Windows: `C:\Users\<user>\AppData\Local\maenifold\models\all-MiniLM-L6-v2.onnx`

### 7.4 GitHub Releases as Primary CDN

Attach the ONNX model to each GitHub Release. The version in the download URL matches the
npm package version, ensuring version coherence. HuggingFace serves as a fallback.

### 7.5 Graceful Degradation Without Model

If the model download fails or the user is offline, maenifold should degrade gracefully:
- Semantic search: unavailable (clear error message)
- Full-text search: still works
- All other tools: unaffected

This is consistent with the existing behavior documented in RTM-X01:
> "No ONNX runtime dependency in maenifold-core (graceful degradation for semantic validation)"

---

## 8. CI/CD Pipeline for Publishing

```yaml
# Simplified GitHub Actions workflow
name: Publish to npm
on:
  release:
    types: [published]

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: macos-latest
            target: aarch64-apple-darwin
            pkg: maenifold-darwin-arm64
          - os: macos-13
            target: x86_64-apple-darwin
            pkg: maenifold-darwin-x64
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            pkg: maenifold-linux-x64
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
            pkg: maenifold-linux-arm64
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            pkg: maenifold-win32-x64
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - name: Build release binary
        run: |
          cargo build --release --target ${{ matrix.target }}
          strip target/${{ matrix.target }}/release/maenifold
      - name: Package for npm
        run: |
          mkdir -p npm/${{ matrix.pkg }}/bin
          cp target/${{ matrix.target }}/release/maenifold npm/${{ matrix.pkg }}/bin/
          cp assets/native/${{ matrix.native_dir }}/* npm/${{ matrix.pkg }}/native/
      - name: Publish platform package
        run: cd npm/${{ matrix.pkg }} && npm publish --access public

  publish-main:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - name: Publish main package
        run: cd npm/maenifold && npm publish --access public
      - name: Upload ONNX model to release
        uses: actions/upload-release-asset@v1
        with:
          upload_url: ${{ github.event.release.upload_url }}
          asset_path: assets/models/all-MiniLM-L6-v2.onnx
          asset_name: all-MiniLM-L6-v2.onnx
```

---

## Sources

1. esbuild package.json — [github.com/evanw/esbuild/npm/esbuild/package.json](https://github.com/evanw/esbuild/blob/main/npm/esbuild/package.json) (verified 2025)
2. esbuild postinstall documentation — [github.com/evanw/esbuild/issues/4288](https://github.com/evanw/esbuild/issues/4288)
3. Biome package.json — [github.com/biomejs/biome/packages/@biomejs/biome/package.json](https://github.com/biomejs/biome/blob/main/packages/%40biomejs/biome/package.json) (verified 2025)
4. Biome bin/biome dispatcher — [github.com/biomejs/biome/packages/@biomejs/biome/bin/biome](https://github.com/biomejs/biome/blob/main/packages/%40biomejs/biome/bin/biome) (verified, actual code)
5. Biome platform package — [github.com/biomejs/biome/packages/@biomejs/cli-darwin-arm64/package.json](https://github.com/biomejs/biome/blob/main/packages/%40biomejs/cli-darwin-arm64/package.json) (verified)
6. oxlint package.json — [github.com/oxc-project/oxc/npm/oxlint/package.json](https://github.com/oxc-project/oxc/blob/main/npm/oxlint/package.json) (verified 2025, uses napi-rs)
7. Prisma engine management — [deepwiki.com/prisma/prisma/4-engine-and-binary-management](https://deepwiki.com/prisma/prisma/4-engine-and-binary-management)
8. sqlite-vec distribution — [github.com/asg017/sqlite-vec](https://github.com/asg017/sqlite-vec), [alexgarcia.xyz/sqlite-vec/installation.html](https://alexgarcia.xyz/sqlite-vec/installation.html)
9. napi-rs documentation — [napi.rs/docs/introduction/getting-started](https://napi.rs/docs/introduction/getting-started)
10. HuggingFace model URLs — memory://build/research/huggingface-hosting-for-allminilml6v2-onnx-model-download-urls-rate-limits-and-net-packaging-patterns (knowledge graph, 2026-02-24)
11. GitHub Release assets limits — [docs.github.com/articles/distributing-large-binaries](https://docs.github.com/articles/distributing-large-binaries)
12. `directories` Rust crate — [codeberg.org/dirs/directories-rs](https://codeberg.org/dirs/directories-rs)
13. `env-paths` npm package — [npmjs.com/package/env-paths](https://www.npmjs.com/package/env-paths)

## Limitations

- **Release binary size**: The debug binary is 48 MB; release stripped size is estimated at ~10 MB
  but needs verification with an actual `cargo build --release` + `strip` cycle.
- **musl Linux**: Not yet tested. May need a `maenifold-linux-x64-musl` variant for Alpine/Docker.
- **Windows ARM64**: No vec0.dll for Windows ARM64 in current assets. May need to add or skip.
- **ONNX Runtime bundling**: The `ort` crate with `download-binaries` feature downloads ONNX Runtime
  shared libraries at build time. These need to be included in the platform packages as well.
  The exact size impact needs measurement.
- **Offline-first environments**: The on-demand download pattern requires internet on first use.
  A `maenifold setup` command could pre-download all assets explicitly.
