# GitHub Pages Official Requirements Research

**Research Date**: October 28, 2025
**Researcher Role**: Research Agent 1 - GitHub Pages Documentation
**Session**: session-1761655960213
**Status**: Complete with Exact Citations

---

## 1. Core Definition and Static File Support

### GitHub Pages Accepts Plain HTML/CSS/JavaScript

**EXACT QUOTE**: "GitHub Pages is a static site hosting service that takes HTML, CSS, and JavaScript files straight from a repository on GitHub, optionally runs the files through a build process, and publishes a website."

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/what-is-github-pages
**Accessed**: 2025-10-28

**Key Implication**: Plain static HTML/CSS/JS sites are fully supported. No build process required.

---

## 2. Repository Structure Requirements

### User/Organization Site Naming

**EXACT QUOTE**: "Sites associated with a user or organization account [...] Must be stored in a repository named `<owner>.github.io`, where `<owner>` is the personal or organization account name"

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/what-is-github-pages
**Accessed**: 2025-10-28

**Additional Note on Casing**:
User/org sites must follow strict naming: "Any uppercase letters in the name must be lowercased."

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/creating-a-github-pages-site
**Accessed**: 2025-10-28

### Project Site Repository Structure

**EXACT QUOTE**: "Project sites [...] Stored in a folder within the repository that contains the project's code"

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/what-is-github-pages
**Accessed**: 2025-10-28

### Entry File Location

**EXACT QUOTE**: "GitHub Pages searches for an `index.html, index.md, or README.md file as the entry file` at the top level of your publishing source directory."

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/creating-a-github-pages-site
**Accessed**: 2025-10-28

**Additional Requirement for /docs Folder**:
"If your publishing source is a branch and folder, the entry file must be at the top level of the source folder on the source branch. For example, files in the `/docs` folder on the `main` branch would need their entry file positioned there."

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/creating-a-github-pages-site
**Accessed**: 2025-10-28

---

## 3. Supported Deployment Methods

### Method 1: Branch-Based Publishing

**EXACT QUOTE**: "You can configure your GitHub Pages site to publish when changes are pushed to a specific branch"

**Source**: https://docs.github.com/en/pages
**Accessed**: 2025-10-28

### Supported Branches

**FINDING**: "Any branch in your repository can serve as a publishing source—there are no restrictions on which branches you can use."

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site
**Accessed**: 2025-10-28

### Folder Structure Options

**EXACT QUOTE**: "Two folder options are available: 1. **Root directory** (`/`) - publishes from the repository root; 2. **Docs folder** (`/docs`) - publishes from a dedicated `/docs` directory on the source branch"

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site
**Accessed**: 2025-10-28

### Method 2: GitHub Actions Workflow

**EXACT QUOTE**: "You can write a GitHub Actions workflow to publish your site."

**Source**: https://docs.github.com/en/pages
**Accessed**: 2025-10-28

**Current Recommendation**:
"GitHub Actions is now the recommended deployment approach over the `github-pages` gem"

**Source**: https://docs.github.com/en/pages/setting-up-a-github-pages-site-with-jekyll/about-github-pages-and-jekyll
**Accessed**: 2025-10-28

### GitHub Actions Workflow Components

Three key actions are required:

1. **configure-pages**: "This action gathers website metadata and supports deployment from any static site generator"

2. **upload-pages-artifact**: "The artifact must be a compressed gzip archive with a single tar file under 10GB, containing no symbolic or hard links."

3. **deploy-pages**: Requires "a minimum of `pages: write` and `id-token: write` permissions."

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages
**Accessed**: 2025-10-28

### Publishing Behavior

**EXACT QUOTE**: "Whenever changes are pushed to the source branch, the changes in the source folder will be published to your GitHub Pages site."

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site
**Accessed**: 2025-10-28

### Critical Limitation: Symbolic Links

**EXACT QUOTE**: "If your repository contains symbolic links, you will need to publish your site using a GitHub Actions workflow"

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site
**Accessed**: 2025-10-28

### Critical Limitation: GITHUB_TOKEN

**FINDING**: "Commits pushed by workflows using `GITHUB_TOKEN` don't trigger GitHub Pages builds"

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site
**Accessed**: 2025-10-28

**Implication**: Custom workflows require additional authentication setup to trigger Pages builds.

---

## 4. Technical Limits and Constraints

### Repository Size Limits

**EXACT QUOTE**: "Source repositories: 1 GB recommended limit; Published sites: maximum 1 GB"

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/github-pages-limits
**Accessed**: 2025-10-28

### Build Limits

**EXACT QUOTE**: "Deployment timeout: 10 minutes maximum; Build limit: 10 builds per hour (doesn't apply with custom GitHub Actions workflows)"

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/github-pages-limits
**Accessed**: 2025-10-28

### Bandwidth Limits

**EXACT QUOTE**: "soft bandwidth limit of 100 GB per month"

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/github-pages-limits
**Accessed**: 2025-10-28

### Site Uniqueness per Account

**EXACT QUOTE**: "You can only create one user or organization site for each account on GitHub"

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/github-pages-limits
**Accessed**: 2025-10-28

**BUT**: "Maximum of one pages site per repository" for project sites, so multiple project repos can have their own project sites.

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/what-is-github-pages
**Accessed**: 2025-10-28

### Rate Limiting

**FINDING**: "Rate limits may apply to maintain service quality, returning HTTP 429 status codes when triggered"

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/github-pages-limits
**Accessed**: 2025-10-28

### Publication Delay

**EXACT QUOTE**: "It can take up to 10 minutes for changes to your site to publish after you push the changes to GitHub."

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/creating-a-github-pages-site
**Accessed**: 2025-10-28

### Deployment Artifact Size (GitHub Actions)

**EXACT QUOTE**: "The artifact must be a compressed gzip archive with a single tar file under 10GB"

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages
**Accessed**: 2025-10-28

---

## 5. Jekyll Support and Configuration

### Jekyll as Supported Generator

**EXACT QUOTE**: "Jekyll is a static site generator with built-in support for GitHub Pages. It transforms Markdown and HTML files into complete static websites using your chosen layouts and supports Markdown plus Liquid templating."

**Source**: https://docs.github.com/en/pages/setting-up-a-github-pages-site-with-jekyll/about-github-pages-and-jekyll
**Accessed**: 2025-10-28

### Fixed Jekyll Settings (Cannot Change)

**EXACT QUOTE**: "GitHub Pages enforces certain Jekyll configurations that cannot be changed: `lsi: false`; `safe: true`; `highlighter: rouge`; `kramdown` with `mathjax` math engine and `rouge` syntax highlighter"

**Source**: https://docs.github.com/en/pages/setting-up-a-github-pages-site-with-jekyll/about-github-pages-and-jekyll
**Accessed**: 2025-10-28

### Built-in Plugins (Cannot Be Disabled)

**EXACT QUOTE**: "GitHub Pages automatically enables nine plugins: jekyll-coffeescript; jekyll-default-layout; jekyll-gist; jekyll-github-metadata; jekyll-optional-front-matter; jekyll-paginate; jekyll-readme-index; jekyll-titles-from-headings; jekyll-relative-links"

**Source**: https://docs.github.com/en/pages/setting-up-a-github-pages-site-with-jekyll/about-github-pages-and-jekyll
**Accessed**: 2025-10-28

### Jekyll Files Automatically Excluded

**EXACT QUOTE**: "Jekyll automatically excludes files in `/node_modules` or `/vendor` folders, those starting with `_`, `.`, or `#`, and those ending with `~`."

**Source**: https://docs.github.com/en/pages/setting-up-a-github-pages-site-with-jekyll/about-github-pages-and-jekyll
**Accessed**: 2025-10-28

### Windows Platform Warning

**EXACT QUOTE**: "Jekyll is not officially supported for Windows"

**Source**: https://docs.github.com/en/pages/setting-up-a-github-pages-site-with-jekyll/about-github-pages-and-jekyll
**Accessed**: 2025-10-28

---

## 6. Custom Domain Configuration

### Supported Domain Types

**EXACT QUOTE**: "GitHub Pages supports two domain categories: 1. **Subdomains** - configured via CNAME records (www subdomains like `www.example.com`; Custom subdomains like `blog.example.com`); 2. **Apex domains** - configured via A, ALIAS, or ANAME records (Root domains without subdomains like `example.com`)"

**Source**: https://docs.github.com/en/pages/configuring-a-custom-domain-for-your-github-pages-site/about-custom-domains-and-github-pages
**Accessed**: 2025-10-28

### Subdomain Stability Recommendation

**EXACT QUOTE**: "www subdomains are the most stable type of custom domain because www subdomains are not affected by changes to the IP addresses of GitHub's servers."

**Source**: https://docs.github.com/en/pages/configuring-a-custom-domain-for-your-github-pages-site/about-custom-domains-and-github-pages
**Accessed**: 2025-10-28

### Automatic Domain Redirects

**EXACT QUOTE**: "When both apex and www domains are properly configured through your DNS provider, GitHub Pages automatically establishes redirects between them. For instance, if you set `www.example.com` as your primary domain, `example.com` will redirect to it automatically."

**Source**: https://docs.github.com/en/pages/configuring-a-custom-domain-for-your-github-pages-site/about-custom-domains-and-github-pages
**Accessed**: 2025-10-28

### Domain Sharing Across Sites

**EXACT QUOTE**: "Custom domains set on user or organization sites automatically apply to associated project sites, though individual repositories can override this with their own domains."

**Source**: https://docs.github.com/en/pages/configuring-a-custom-domain-for-your-github-pages-site/about-custom-domains-and-github-pages
**Accessed**: 2025-10-28

---

## 7. HTTPS and Security

### Automatic HTTPS Provisioning

**EXACT QUOTE**: "Sites created after June 15, 2016, using `github.io` domains are served over HTTPS automatically. All GitHub Pages sites support HTTPS, including custom domains."

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/securing-your-github-pages-site-with-https
**Accessed**: 2025-10-28

### HTTPS Enforcement

**EXACT QUOTE**: "Users with admin permissions can enable HTTPS enforcement through repository Settings > Pages, which transparently redirect all HTTP requests to HTTPS"

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/securing-your-github-pages-site-with-https
**Accessed**: 2025-10-28

### TLS Certificate Provider

**EXACT QUOTE**: "GitHub uses Let's Encrypt for TLS certificate generation."

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/securing-your-github-pages-site-with-https
**Accessed**: 2025-10-28

### Certificate Provisioning Process

**EXACT QUOTE**: "When you configure a custom domain: 1. An automatic DNS check begins; 2. GitHub requests a certificate from Let's Encrypt upon successful verification; 3. The certificate is automatically uploaded to TLS termination servers; 4. A checkmark appears once complete"

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/securing-your-github-pages-site-with-https
**Accessed**: 2025-10-28

### DNS Requirements for Certificate

**EXACT QUOTE**: "For successful certificate generation, avoid extra A, AAAA, ALIAS, ANAME records with the @ host, or CNAME records pointing to www or custom subdomains. Properly configured DNS pointing to GitHub's IP addresses is essential."

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/securing-your-github-pages-site-with-https
**Accessed**: 2025-10-28

### Domain Name Length Restriction

**EXACT QUOTE**: "Domain names must be less than 64 characters long for a certificate to be successfully created" (per RFC3280)

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/securing-your-github-pages-site-with-https
**Accessed**: 2025-10-28

### Mixed Content Warning

**FINDING**: "Sites must reference all assets (CSS, JavaScript, images) over HTTPS, not HTTP, to avoid security warnings and loading failures."

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/securing-your-github-pages-site-with-https
**Accessed**: 2025-10-28

### Domain Takeover Risk

**EXACT QUOTE**: "If your Pages site is disabled but retains custom domain DNS records, the domain faces takeover risks. GitHub recommends verifying your custom domain before adding it to repositories to prevent unauthorized use by other users."

**Source**: https://docs.github.com/en/pages/configuring-a-custom-domain-for-your-github-pages-site/about-custom-domains-and-github-pages
**Accessed**: 2025-10-28

---

## 8. Public Accessibility and Privacy

### Public Visibility (Important)

**EXACT QUOTE**: "GitHub Pages sites are publicly available on the internet, even if the repository for the site is private"

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/creating-a-github-pages-site
**Accessed**: 2025-10-28

### IP Address Logging

**EXACT QUOTE**: "When a GitHub Pages site is visited, the visitor's IP address is logged and stored for security purposes, regardless of whether the visitor has signed into GitHub or not."

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/what-is-github-pages
**Accessed**: 2025-10-28

---

## 9. Plan Availability

### Free Tier Support

**EXACT QUOTE**: "GitHub Pages is available in public repositories with GitHub Free and GitHub Free for organizations, and in public and private repositories with GitHub Pro, GitHub Team, GitHub Enterprise Cloud, and GitHub Enterprise Server."

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/what-is-github-pages
**Accessed**: 2025-10-28

---

## 10. Prohibited Uses

### Explicit Restrictions

**EXACT QUOTE**: "GitHub Pages prohibits commercial use, e-commerce operations, SaaS offerings, and sensitive transactions. The service also restricts content violating GitHub's Terms of Service, including get-rich-quick schemes, obscene material, and violent content."

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/github-pages-limits
**Accessed**: 2025-10-28

---

## 11. Custom 404 Page Support

### Supported File Types

**FINDING**: "Custom 404 pages are supported using either `404.html` or `404.md` as the filename"

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/creating-a-custom-404-page-for-your-github-pages-site
**Accessed**: 2025-10-28

### Markdown Requirement

**EXACT QUOTE**: "If you choose the `.md` format, include this at the top: `--- permalink: /404.html ---`"

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/creating-a-custom-404-page-for-your-github-pages-site
**Accessed**: 2025-10-28

---

## 12. Site Types and Default URLs

### Site Type Comparison

| Property | User and organization sites | Project sites |
|----------|--------------------------|---------------|
| **Source files** | Repository named `<owner>.github.io` | Folder within project repository |
| **Limits** | One site per account | One site per repository |
| **Default URL** | `https://<owner>.github.io` | `https://<owner>.github.io/<repositoryname>` |

**Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/what-is-github-pages
**Accessed**: 2025-10-28

---

## Summary: Key Technical Facts for PRD Validation

1. **Plain HTML/CSS/JS**: ✅ Fully supported without build process
2. **Build Process**: ✅ Optional - can deploy pre-built artifacts
3. **Custom Generators**: ✅ Supported via GitHub Actions
4. **Repository Size**: ✅ 1 GB recommended source, 1 GB max published
5. **Build Time**: ✅ 10 minute timeout for branch-based publishing
6. **Build Frequency**: ✅ 10 builds per hour for branch-based (unlimited with GitHub Actions)
7. **Bandwidth**: ✅ 100 GB/month soft limit
8. **Custom Domains**: ✅ Full support (CNAME for subdomains, A/ALIAS/ANAME for apex)
9. **HTTPS**: ✅ Automatic with Let's Encrypt
10. **Jekyll**: ✅ Supported but with fixed configuration constraints
11. **Deployment Methods**: ✅ Branch-based OR GitHub Actions
12. **Artifact Size (GHA)**: ✅ Under 10 GB gzip archive

---

## Next Steps for Research Team

- **Agent 2**: Research [[Next.js]] static export official documentation
- **Agent 3**: Research [[GitHub-Actions]] deployment patterns for static sites
- **Agent 4**: Research [[Tailwind-CSS]] + [[Next.js]] integration requirements

All claims in [[PRD]] should be cross-referenced against these official quotes before finalizing.
