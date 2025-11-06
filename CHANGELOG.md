# Changelog

All notable changes to maenifold MCP Server will be documented in this file.

## [1.0.2] - 2025-11-06

### Fixed
- **CRITICAL**: Removed broken optionalDependencies that prevented installation
  - v1.0.0 and v1.0.1 referenced non-existent platform packages
  - Now ships all binaries in single package with automatic platform detection
- Fixed VS Code install URL scheme on website (vscode:mcp/install)
- Added mandatory release validation (pre-commit + GitHub Actions)
  - Blocks commits with optionalDependencies
  - Validates VS Code URL schemes
  - Verifies package.json structure

### Changed
- Distribution model: single package with bundled binaries for all platforms
- Wrapper script automatically selects correct binary based on OS/arch

### Architecture
- No separate platform packages (maenifold-win32-x64, etc.)
- All binaries included in bin/ directory structure
- Cleaner, more reliable installation process
