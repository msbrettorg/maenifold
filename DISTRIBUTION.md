# Distribution Strategy

maenifold is distributed through four primary channels for maximum reach:

## 1. NPM Registry (@ma-collective/maenifold)

**Primary distribution method** for users installing via package managers.

### Installation
```bash
npm install -g @ma-collective/maenifold
```

### What's Included
- Self-contained binaries for all platforms (Linux x64, macOS ARM64/x64, Windows x64)
- No .NET runtime required - binaries are self-contained
- Platform detection wrapper script
- Core documentation (README.md, DEVELOPMENT.md, DISTRIBUTION.md)

### Update Process
```bash
npm update -g @ma-collective/maenifold
```

### Publishing
See [NPM_SETUP.md](NPM_SETUP.md) for complete publishing instructions.

## 2. Homebrew Tap (macOS/Linux)

**Primary distribution for macOS and Linux developers.**

### Installation
```bash
brew tap ma-collective/tap
brew install maenifold
```

Or in one command:
```bash
brew install ma-collective/tap/maenifold
```

### What's Included
- Platform-specific binaries (macOS ARM64, macOS x64, Linux x64)
- Automatic PATH setup
- Easy updates via `brew upgrade`

### Publishing
See [distribution/homebrew/README.md](distribution/homebrew/README.md) for tap setup and publishing instructions.

## 3. WinGet (Windows Official)

**Official Microsoft package manager for Windows 10/11.**

### Installation
```bash
winget install maenifold
```

### What's Included
- Self-contained Windows x64 binary
- Automatic PATH setup
- Integration with Windows Package Manager

### Publishing
See [distribution/winget/README.md](distribution/winget/README.md) for manifest submission to Microsoft's repository.

## 4. GitHub Releases

**Universal fallback** for direct downloads, source code, and release notes.

### Release Assets
Each GitHub release should include:
- Source code (automatic from tag)
- Platform-specific binaries:
  - `maenifold-linux-x64.tar.gz`
  - `maenifold-osx-arm64.tar.gz`
  - `maenifold-osx-x64.tar.gz`
  - `maenifold-win-x64.zip`
- Release notes with changelog
- SHA256 checksums for all binaries

### GitHub Release Process

**Use the automated release script:**

```bash
./scripts/release.sh 1.0.0
```

This script will:
- Clean previous builds
- Run tests
- Build all platform binaries
- Package archives (tar.gz for Linux/macOS, zip for Windows)
- Generate SHA256SUMS
- Update Homebrew formula with checksums
- Update WinGet manifests with checksums

**Then create the GitHub release:**

1. **Tag the release:**
   ```bash
   git tag -a v1.0.0 -m "Release v1.0.0"
   git push origin v1.0.0
   ```

2. **Create GitHub Release:**
   - Go to: https://github.com/msbrettorg/maenifold/releases/new
   - Select the tag `v1.0.0`
   - Use template from [distribution/GITHUB_RELEASE_TEMPLATE.md](distribution/GITHUB_RELEASE_TEMPLATE.md)
   - Attach the 5 files:
     - `maenifold-linux-x64.tar.gz`
     - `maenifold-osx-arm64.tar.gz`
     - `maenifold-osx-x64.tar.gz`
     - `maenifold-win-x64.zip`
     - `SHA256SUMS`
   - Publish release

## 5. Direct .NET Integration (Optional)

For projects using the Microsoft Agent Framework:

### Git Submodule
```bash
git submodule add https://github.com/ma-collective/maenifold.git external/maenifold
```

### Project Reference
```xml
<ProjectReference Include="../external/maenifold/src/Maenifold.csproj" />
```

Then use static methods directly:
```csharp
using Maenifold.Tools;

var result = MemoryTools.WriteMemory(
    "API Decision",
    "We chose [[REST]] over [[GraphQL]]...",
    folder: "decisions"
);
```

## Distribution Checklist

When releasing a new version:

### Pre-Release
- [ ] Update CHANGELOG.md with version and changes
- [ ] Update version in package.json
- [ ] Update version in all distribution files (if not using release script)
- [ ] Run full test suite: `dotnet test`
- [ ] All tests passing (0 errors, 0 warnings)

### Build
- [ ] Run release script: `./scripts/release.sh 1.0.0`
- [ ] Verify all 4 platform binaries created
- [ ] Verify SHA256SUMS generated
- [ ] Verify Homebrew formula updated with checksums
- [ ] Verify WinGet manifests updated with checksums

### Release
- [ ] Create git tag: `git tag -a v1.0.0 -m "Release v1.0.0"`
- [ ] Push tag: `git push origin v1.0.0`
- [ ] Create GitHub Release using template
- [ ] Upload 5 files to GitHub release (4 archives + SHA256SUMS)
- [ ] Mark as "Latest release" (if applicable)

### Distribution
- [ ] Publish to NPM: `npm publish --access public`
- [ ] Update Homebrew tap repository with new formula
- [ ] Submit WinGet PR to microsoft/winget-pkgs (for new versions)

### Verification
- [ ] Test NPM: `npm install -g @ma-collective/maenifold && maenifold --tool MemoryStatus --payload '{}'`
- [ ] Test Homebrew: `brew install ma-collective/tap/maenifold && maenifold --tool MemoryStatus --payload '{}'`
- [ ] Test WinGet (after PR merged): `winget install maenifold`
- [ ] Test direct download from GitHub release
- [ ] Verify all checksums match

## Version Numbering

Follow semantic versioning (semver):
- **Major (1.0.0)**: Breaking API changes
- **Minor (0.1.0)**: New features, backward compatible
- **Patch (0.0.1)**: Bug fixes, backward compatible

## Support Matrix

| Platform | Architecture | .NET | Node.js | Status |
|----------|-------------|------|---------|--------|
| Linux | x64 | Self-contained | 18+ | ✅ Supported |
| macOS | ARM64 (M1/M2/M3) | Self-contained | 18+ | ✅ Supported |
| macOS | x64 (Intel) | Self-contained | 18+ | ✅ Supported |
| Windows | x64 | Self-contained | 18+ | ✅ Supported |

All binaries are self-contained - no .NET runtime installation required.
