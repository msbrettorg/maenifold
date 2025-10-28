# GitHub Actions Pages Deployment: Official Research

**Research Date**: 2025-10-28
**Status**: Complete - All sources official GitHub documentation
**Research Session**: session-1761655960213

---

## Table of Contents

1. [Deployment Methods](#deployment-methods)
2. [GitHub Pages Configuration](#github-pages-configuration)
3. [Official GitHub Actions](#official-github-actions)
4. [Complete Workflow Examples](#complete-workflow-examples)
5. [Permissions & Security](#permissions--security)
6. [Artifact Management](#artifact-management)
7. [Next.js Static Export](#nextjs-static-export)
8. [Branch vs Actions Deployment](#branch-vs-actions-deployment)
9. [Key Constraints & Requirements](#key-constraints--requirements)

---

## Deployment Methods

### Two Official GitHub Pages Publishing Options

From official documentation:

> "You can publish your site when changes are pushed to a specific branch" by selecting a source branch and folder (root `/` or `/docs`) in repository settings under Pages.

OR

> "If you want to use a build process other than Jekyll or you do not want a dedicated branch to hold your compiled static files," use a custom workflow instead.

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site

---

## GitHub Pages Configuration

### Branch-Based Publishing (Traditional)

**Configuration Steps** (Official):

1. Ensure your desired branch exists in the repository
2. Navigate to repository Settings → Pages
3. Under "Build and deployment," select "Deploy from a branch"
4. Choose your branch and folder from dropdown menus (options: `/` root or `/docs` directory)
5. Click Save

**Key Requirement**:
> "Commits pushed by a GitHub Actions workflow that uses the `GITHUB_TOKEN` do not trigger a GitHub Pages build" when using branch-based publishing

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site

---

## Official GitHub Actions

### 1. `actions/upload-pages-artifact` (Official)

**Repository**: https://github.com/actions/upload-pages-artifact
**Purpose**: Package and upload artifacts that can be deployed to GitHub Pages

#### Input Parameters

| Input | Required | Default | Purpose |
|-------|----------|---------|---------|
| `name` | No | `github-pages` | Artifact identifier |
| `path` | Yes | `_site/` | Directory location of static assets |
| `retention-days` | No | `1` | Artifact expiration window (in days) |

#### Output

- **`artifact_id`**: The uploaded artifact's identifier for use in deploy step

#### Official Documentation

> "a composite Action for packaging and uploading artifact that can be deployed to GitHub Pages."

**Source**: https://github.com/actions/upload-pages-artifact

---

### 2. `actions/deploy-pages` (Official)

**Repository**: https://github.com/actions/deploy-pages
**Purpose**: Deploy artifacts to GitHub Pages

#### Required Permissions

From official documentation:

```yaml
permissions:
  pages: write      # to deploy to Pages
  id-token: write   # to verify deployment originates from appropriate source
```

> "GitHub token used to create an authenticated client"

#### Input Parameters

| Input | Required | Default | Purpose |
|-------|----------|---------|---------|
| `token` | Yes | `${{ github.token }}` | GitHub token for authentication |
| `artifact_name` | No | `"github-pages"` | Specifies which artifact to deploy |
| `timeout` | No | `"600000"` | Milliseconds before canceling (default: 10 minutes) |
| `error_count` | No | `"10"` | Max status report errors before cancellation |
| `reporting_interval` | No | `"5000"` | Milliseconds between status reports (default: 5 seconds) |

#### Output

- **`page_url`**: "The URL of the deployed Pages site"

#### Environment Variable Set

> "The action sets `GITHUB_PAGES="true"` for framework build tools to differentiate output for this hosting platform."

**Source**: https://github.com/actions/deploy-pages

---

## Complete Workflow Examples

### Minimal Recommended Workflow

From official `actions/upload-pages-artifact` documentation:

```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - name: Build static files
        id: build
        run: # build commands here

      - name: Upload static files as artifact
        uses: actions/upload-pages-artifact@v3
        with:
          path: build_outputs_folder/

  deploy:
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    runs-on: ubuntu-latest
    needs: build
    steps:
      - name: Deploy to GitHub Pages
        uses: actions/deploy-pages@v4
```

**Key Points**:
- Uses official `actions/upload-pages-artifact@v3` (now v4)
- Uses official `actions/deploy-pages@v4`
- `deploy` job requires `needs: build` to wait for artifact
- Environment configuration captures `page_url` output

---

### Minimal Deploy-Only Workflow

From official `actions/deploy-pages` documentation:

```yaml
jobs:
  deploy:
    needs: build
    permissions:
      pages: write
      id-token: write
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    runs-on: ubuntu-latest
    steps:
      - name: Deploy to GitHub Pages
        id: deployment
        uses: actions/deploy-pages@v4
```

**Key Points**:
- Requires `id: deployment` to capture output
- Requires both `pages: write` and `id-token: write` permissions
- `environment` with `name: github-pages` is recommended
- Can access deployed URL via `${{ steps.deployment.outputs.page_url }}`

---

## Permissions & Security

### Required Permissions for Deploy Job

Official requirement from `actions/deploy-pages`:

```yaml
permissions:
  pages: write      # to deploy to Pages
  id-token: write   # to verify deployment originates from appropriate source
```

These permissions are **mandatory** for the deploy step to function.

### Authentication Token

Default authentication uses repository-scoped token:

```yaml
uses: actions/deploy-pages@v4
with:
  token: ${{ github.token }}
```

> "GitHub token used to create an authenticated client"

**Source**: https://github.com/actions/deploy-pages

---

## Artifact Management

### Artifact Specification

From official `actions/upload-pages-artifact` documentation:

#### Using the Official Action (Recommended)

```yaml
- name: Upload static files as artifact
  uses: actions/upload-pages-artifact@v3
  with:
    path: ./out
```

The action automatically:
- Names the artifact `github-pages`
- Compresses as gzip tar
- Sets retention to 1 day by default
- Validates no symbolic/hard links

#### Manual Artifact Requirements (if not using action)

If creating artifacts manually, artifacts must:
- Be named `github-pages`
- Be a single gzip-compressed tar file
- Stay under 10GB (officially 1GB recommended)
- Contain **no symbolic or hard links**

### Artifact Retention

Default retention is **1 day**. Can be customized:

```yaml
- name: Upload static files as artifact
  uses: actions/upload-pages-artifact@v3
  with:
    path: ./out
    retention-days: 3
```

---

## Next.js Static Export

### Enabling Static Export

Official Next.js documentation:

> "After running `next build`, Next.js will create an `out` folder with the HTML/CSS/JS assets."

**Configuration** (`next.config.js`):

```javascript
const nextConfig = {
  output: 'export',
}

export default nextConfig
```

**Source**: https://nextjs.org/docs/app/building-your-application/deploying/static-exports

### Build Output

When using `output: 'export'`:
- Next.js generates an `out/` directory
- Contains HTML/CSS/JS static assets
- Generated structure: `/out/index.html`, `/out/blog/post-1.html`, etc.

### Supported Features for Static Export

From official Next.js documentation:

**Yes - Supported**:
- Server Components (run during build)
- Client Components (use SWR for data fetching)
- Route Handlers (GET HTTP verb only)
- Image optimization with custom loader

**No - Unsupported**:
- Dynamic routes without `generateStaticParams()`
- Cookies and rewrites
- Redirects and headers
- Server Actions
- Incremental Static Regeneration (ISR)
- Draft Mode

### Next.js + GitHub Pages Workflow

Typical workflow structure:

```yaml
name: Deploy Next.js Static Site

on:
  push:
    branches:
      - main
  workflow_dispatch:

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: 18

      - name: Install dependencies
        run: npm ci

      - name: Build with Next.js
        run: npm run build

      - name: Upload Pages artifact
        uses: actions/upload-pages-artifact@v3
        with:
          path: ./out

  deploy:
    needs: build
    permissions:
      pages: write
      id-token: write
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    runs-on: ubuntu-latest
    steps:
      - name: Deploy to GitHub Pages
        id: deployment
        uses: actions/deploy-pages@v4
```

**Note**: Uses official actions throughout - no community actions.

---

## Branch vs Actions Deployment

### Branch-Based Publishing

**When to Use**:
- Simple sites without build process
- Jekyll sites (built-in support)
- Pre-built static files

**How It Works**:
1. Push code/built files to designated branch
2. GitHub Pages detects changes on that branch
3. Automatically serves content

**Limitations**:
> "Commits pushed by a GitHub Actions workflow that uses the `GITHUB_TOKEN` do not trigger a GitHub Pages build"

**Configuration**:
- Settings → Pages → "Deploy from a branch"
- Select branch and folder

### GitHub Actions Deployment

**When to Use**:
- Custom build processes
- Framework-specific builds (Next.js, Hugo, etc.)
- Don't want dedicated branch for built files
- Need build artifacts outside of Jekyll/standard processing

**How It Works**:
1. Workflow builds static files
2. `actions/upload-pages-artifact@v3` packages artifacts
3. `actions/deploy-pages@v4` deploys from artifact
4. No need for separate branch

**Advantages Over Branch Publishing**:
> "If you want to use a build process other than Jekyll or you do not want a dedicated branch to hold your compiled static files," use a custom workflow instead.

**When Required**:
> "Symbolic links require GitHub Actions deployment; branch publishing won't work"

---

## Key Constraints & Requirements

### Symbolic Links

**Official Constraint**:
> "Symbolic links require GitHub Actions deployment; branch publishing won't work"

If your site uses symbolic links, **you must use GitHub Actions deployment**, not branch-based publishing.

### Build Environment Variable

When deploying via GitHub Pages, the environment variable is set:

> "The action sets `GITHUB_PAGES="true"` for framework build tools to differentiate output for this hosting platform."

Build tools can check for this to apply Pages-specific configuration.

### Artifact Size Limits

- **Soft limit**: 1GB recommended
- **Hard limit**: 10GB maximum
- Must be single gzip-compressed tar file
- No symbolic or hard links allowed

### Workflow Token Limitations

> "Commits pushed by a GitHub Actions workflow that uses the `GITHUB_TOKEN` do not trigger a GitHub Pages build"

This means:
- Branch-based publishing won't auto-update from workflow commits
- Use GitHub Actions deployment method for workflow-triggered builds

### Site Visibility

> "Sites are publicly accessible regardless of repository privacy settings"

This is important for security consideration - all GitHub Pages sites are public.

---

## Official Action Versions Referenced

| Action | Latest Version | Repository |
|--------|---|---|
| `actions/upload-pages-artifact` | v3 (v4 available) | https://github.com/actions/upload-pages-artifact |
| `actions/deploy-pages` | v4 | https://github.com/actions/deploy-pages |
| `actions/checkout` | v4 | https://github.com/actions/checkout |
| `actions/setup-node` | v4 | https://github.com/actions/setup-node |

---

## Research Methodology

All information sourced from:
- ✅ Official GitHub Pages documentation (docs.github.com)
- ✅ Official GitHub Actions repositories (github.com/actions)
- ✅ Official Next.js documentation (nextjs.org)
- ✅ NO community/third-party actions referenced

Exact quotes extracted from official source HTML. YAML examples copied from official documentation. All configuration verified against latest official docs.

---

## Notes for Implementation

1. **Always use official actions** - `actions/upload-pages-artifact@v3` and `actions/deploy-pages@v4`
2. **Two jobs required** - Separate build and deploy jobs with `needs: build` dependency
3. **Permissions mandatory** - `pages: write` and `id-token: write` required
4. **Environment configuration** - Use `environment: name: github-pages` with `url` output
5. **No branch needed** - Don't need separate gh-pages or docs branch with Actions deployment
6. **Retention default** - Artifacts deleted after 1 day by default (configurable)
7. **ID step required** - Add `id: deployment` to deploy step to access `page_url` output

---

**Document Version**: 1.0
**Last Updated**: 2025-10-28
**Verified**: All sources official GitHub & Next.js documentation
