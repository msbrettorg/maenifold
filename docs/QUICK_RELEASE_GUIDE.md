# Quick Release Guide

## For Your First npm Release

### 1. Set up npm (One-time setup)

```bash
# Create account at https://www.npmjs.com/signup
# Enable 2FA in account settings
# Create organization 'ma-collective' at https://www.npmjs.com/org/create

# Generate automation token at https://www.npmjs.com/settings/USERNAME/tokens
# Add token to GitHub: Settings → Secrets → Actions → New secret
#   Name: NPM_TOKEN
#   Value: (paste your token)
```

### 2. Trigger Release

```bash
# Commit all changes
git add .
git commit -m "Release v1.0.0"

# Create and push tag
git tag v1.0.0
git push origin v1.0.0
```

### 3. Wait for Automation

GitHub Actions will:
- ✅ Build all 6 platform binaries
- ✅ Create GitHub Release with archives
- ✅ Publish to npm automatically

Monitor at: https://github.com/msbrettorg/maenifold/actions

### 4. Verify

```bash
# Check npm
npm view @ma-collective/maenifold

# Test install
npm install -g @ma-collective/maenifold
maenifold --version
```

## For Subsequent Releases

```bash
# 1. Update version numbers in:
#    - package.json (2 places: version + optionalDependencies)
#    - src/maenifold.csproj

# 2. Commit and tag
git add .
git commit -m "Bump version to 1.0.1"
git tag v1.0.1
git push origin main --tags

# 3. Wait for automation
```

## Manual Build (Testing Only)

```bash
# Build all platforms locally
npm run build:all

# Create platform packages
./scripts/create-platform-packages.sh

# Test package
npm pack
```

## Troubleshooting

**Publish fails with 403?**
→ Check NPM_TOKEN secret is set in GitHub

**Platform package not found?**
→ Ensure all 6 platform packages published successfully

**Build fails?**
→ Check GitHub Actions logs for error details

---

See [NPM_PUBLISHING.md](./NPM_PUBLISHING.md) for complete documentation.
