# Distribution Strategy

maenifold is distributed through four primary channels:

## 1. GitHub Releases (Primary)

**Universal distribution** with platform-specific binaries and installers.

### Release Assets
Each GitHub release includes:
- Source code (automatic from tag)
- Platform-specific binaries:
  - `maenifold-osx-arm64.zip`
  - `maenifold-osx-x64.zip`
  - `maenifold-linux-x64.zip`
  - `maenifold-linux-arm64.zip`
  - `maenifold-win-x64.zip`
- Release notes with changelog

### Manual Installation
```bash
# macOS/Linux
curl -LO https://github.com/msbrettorg/maenifold/releases/latest/download/maenifold-osx-arm64.zip
unzip maenifold-osx-arm64.zip
sudo mv maenifold /usr/local/bin/
```

## 2. Homebrew (macOS/Linux)

**Recommended for macOS and Linux developers.**

### Installation
```bash
brew tap msbrettorg/tap
brew install maenifold
```

Or in one command:
```bash
brew install msbrettorg/tap/maenifold
```

### What's Included
- Platform-specific binaries (macOS ARM64, macOS x64, Linux x64, Linux ARM64)
- Automatic PATH setup
- Easy updates via `brew upgrade maenifold`

## 3. Windows MSI Installer

**Recommended for Windows users.**

### Installation
Download the `.msi` file from GitHub Releases and run the installer.

### What's Included
- Self-contained Windows x64 binary
- Automatic PATH setup (system-wide)
- Clean uninstall (removes PATH entry)
- Installs to `C:\Program Files\MSBrett\Maenifold\`

## 4. .NET Tool (NuGet)

**For .NET developers who prefer dotnet tool management.**

### Installation
```bash
dotnet tool install --global Maenifold
```

### Update
```bash
dotnet tool update --global Maenifold
```

### Requirements
- .NET 9.0 SDK or later

## Distribution Checklist

When releasing a new version:

### Pre-Release
- [ ] Update CHANGELOG.md with version and changes
- [ ] Update version in `src/Maenifold.csproj`
- [ ] Run full test suite: `dotnet test`
- [ ] All tests passing

### Release
- [ ] Merge PR to main
- [ ] Create git tag: `git tag -a v1.0.0 -m "Release v1.0.0"`
- [ ] Push tag: `git push origin v1.0.0`
- [ ] GitHub Actions builds and publishes release automatically

### Verification
- [ ] Verify GitHub Release created with 6 artifacts (5 archives + MSI)
- [ ] Test Homebrew: `brew upgrade maenifold && maenifold --help`
- [ ] Test MSI installer on Windows
- [ ] Test direct download from GitHub release

## Version Numbering

Follow semantic versioning (semver):
- **Major (1.0.0)**: Breaking API changes
- **Minor (0.1.0)**: New features, backward compatible
- **Patch (0.0.1)**: Bug fixes, backward compatible

## Support Matrix

| Platform | Architecture | Status |
|----------|-------------|--------|
| Linux | x64 | Supported |
| Linux | ARM64 | Supported |
| macOS | ARM64 (M1/M2/M3) | Supported |
| macOS | x64 (Intel) | Supported |
| Windows | x64 | Supported |

All binaries are self-contained - no .NET runtime installation required (except for dotnet tool install method).
