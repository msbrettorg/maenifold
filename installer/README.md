# Maenifold Windows MSI Installer

WiX v5 configuration for building a Windows MSI installer.

## Features

- Installs `maenifold.exe` to `C:\Program Files\Maenifold\`
- Adds install directory to system PATH
- Cleanly removes PATH entry on uninstall
- Supports upgrades (replaces older versions)

## Prerequisites

- [WiX Toolset v5](https://wixtoolset.org/docs/intro/)
- .NET SDK 8.0+

## Building the MSI

1. First, publish the application for Windows x64:

```bash
dotnet publish src/Maenifold.csproj -c Release -r win-x64 --self-contained -o publish/win-x64
```

2. Build the MSI installer:

```bash
# Install WiX tools if not already installed
dotnet tool install --global wix

# Build the MSI
wix build installer/maenifold.wxs -o dist/maenifold.msi
```

## Configuration

| Setting | Value |
|---------|-------|
| Install Location | `C:\Program Files\Maenifold\` |
| UpgradeCode | `F95FE8AD-AD08-4BB2-A193-BB59347F47D5` |
| Scope | Per-machine (requires admin) |

## Upgrade Behavior

The `MajorUpgrade` element ensures:
- Installing a newer version automatically removes the older version
- Installing an older version over a newer one is blocked with an error message

## PATH Integration

The installer adds the installation directory to the system PATH environment variable:
- `Permanent="no"` ensures the entry is removed on uninstall
- `Part="last"` appends to existing PATH rather than replacing
- `System="yes"` modifies system PATH (not user PATH)

A system restart or new terminal session may be required for PATH changes to take effect.
