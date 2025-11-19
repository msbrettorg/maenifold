# Changelog

All notable changes to maenifold MCP Server will be documented in this file.

## [1.0.2] - 2025-11-19

### Fixed
- **CRITICAL**: Removed broken optionalDependencies that prevented installation
  - v1.0.0 and v1.0.1 referenced non-existent platform packages
  - Now ships all binaries in single package with automatic platform detection
- Fixed VS Code install URL scheme on website (vscode:mcp/install)
- Fixed release archive filenames to include version number (e.g., maenifold-1.0.2-linux-x64.tar.gz)
- Added mandatory release validation (pre-commit + GitHub Actions)
  - Blocks commits with optionalDependencies
  - Validates VS Code URL schemes
  - Verifies package.json structure
- Removed automatic npm publishing from release workflow
  - npm publishing moved to separate manual workflow (publish-npm.yml)
  - Prevents accidental npm publishes on GitHub releases
- Fixed Git LFS checkout in release workflow
  - ONNX model file (90MB) was being checked out as LFS pointer (133 bytes)
  - Added `lfs: true` to checkout action to fetch actual model files
  - Previous releases (v1.0.2 initial) were non-functional due to missing model

### Changed
- Distribution model: single package with bundled binaries for all platforms
- Wrapper script automatically selects correct binary based on OS/arch
- **Platform Support**: Temporarily removed linux-arm64 and win-arm64 builds
  - Missing sqlite-vec native libraries (vec0) for these platforms
  - See [sqlite-vec#231](https://github.com/asg017/sqlite-vec/issues/231)
- Updated CLAUDE.md asset counts (32 workflows, 16 roles)

### Architecture
- No separate platform packages (maenifold-win32-x64, etc.)
- All binaries included in bin/ directory structure
- Cleaner, more reliable installation process
- SHA256 checksums now generated for all release archives
- README and LICENSE included in release archives
