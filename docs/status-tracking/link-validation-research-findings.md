# Official Link Validation Tools and Methods for Static Websites

**Research Date:** 2025-10-28
**Research Scope:** Official documentation only (no third-party tools)

---

## Executive Summary

This document presents findings from researching official link validation tools and methods from three authoritative sources: GitHub, Next.js/Vercel, and W3C. The research focused exclusively on officially maintained tools and documentation.

**Key Findings:**
1. **GitHub Actions (actions/*)**: ❌ No official link validation action exists
2. **Next.js**: ✅ Built-in Statically Typed Links (typedRoutes) feature exists
3. **W3C**: ✅ Official W3C Link Checker exists with web and CLI interfaces

---

## 1. Official GitHub Actions for Link Checking

### Finding: NO OFFICIAL LINK CHECKER EXISTS

**Research Method:**
- Searched GitHub official actions organization: https://github.com/actions
- Queried GitHub API for all repositories in the `actions/*` namespace
- Reviewed GitHub Enterprise Server documentation for bundled official actions

**Evidence:**
The `actions/*` namespace contains 80+ repositories including:
- `actions/checkout`
- `actions/setup-node`
- `actions/setup-python`
- `actions/setup-go`
- `actions/upload-artifact`
- `actions/download-artifact`
- `actions/cache`
- `actions/github-script`
- `actions/labeler`
- `actions/stale`

**None of these official actions provide link validation functionality.**

**Third-Party Alternatives (Not Official):**
While GitHub Marketplace contains many community-maintained link checkers (e.g., `lycheeverse/lychee-action`, `gaurav-nelson/github-action-markdown-link-check`, `peter-evans/link-checker`), these are NOT official GitHub actions and fall outside the scope of this research.

**Conclusion:**
GitHub does not provide an official link validation action. Projects requiring link checking must either:
- Use third-party community actions (not officially supported)
- Implement custom validation scripts
- Use external validation tools

**Source:**
- Official Actions Organization: https://github.com/actions
- GitHub API: https://api.github.com/orgs/actions/repos

---

## 2. Next.js Built-in Link Validation

### Finding: STATICALLY TYPED LINKS FEATURE EXISTS

Next.js provides built-in link validation through its **Statically Typed Links** feature (also called **Typed Routes**), which validates route references at build time using TypeScript.

### How It Works

When enabled, Next.js generates a `.d.ts` file in `.next/types` containing type definitions for all valid routes in your application. The TypeScript compiler validates `href` props in `next/link` and navigation method calls against these generated types.

### Configuration

Enable in your Next.js configuration:

```typescript
// next.config.ts
import type { NextConfig } from 'next'

const nextConfig: NextConfig = {
  typedRoutes: true,
}

export default nextConfig
```

### Requirements

- TypeScript must be enabled in the project
- The `typedRoutes` option must be set to `true`
- The generated types must be included in `tsconfig.json` (handled automatically by `create-next-app`)

### Usage Examples

#### Valid Route References
```typescript
import Link from 'next/link'

// TypeScript validates this route exists
<Link href="/about" />

// Dynamic routes are supported
<Link href={`/blog/${slug}`} />
```

#### Type Errors Caught
```typescript
// Error: Route does not exist
<Link href="/aboot" />  // TypeScript error: typo detected

// Error: Invalid route format
<Link href="https://external.com" />  // TypeScript error: external URLs not allowed
```

#### Navigation Methods
```typescript
import { useRouter } from 'next/navigation'

const router = useRouter()

// All validated at compile time
router.push('/about')
router.replace('/contact')
router.prefetch('/products')
```

#### Custom Components with Type Safety
```typescript
import type { Route } from 'next'

function CustomLink<T extends string>({
  href
}: {
  href: Route<T> | URL
}) {
  return <Link href={href}>Click here</Link>
}
```

### Additional Runtime Validation

Next.js also provides runtime error messages for invalid link usage:

1. **Invalid href with anchor child** - Error when using `<a>` as a child of `<Link>`
2. **Invalid href passed to router** - Error when using external URLs or `mailto:` links with `next/link`
3. **No HTML link for pages** - ESLint rule preventing `<a>` elements for internal navigation

### Limitations

- Only validates internal application routes
- Does not check if external links are valid/reachable
- Non-literal string hrefs require explicit type casting:
  ```typescript
  <Link href={('/blog/' + slug) as Route} />
  ```

### Source Documentation

- **Official Next.js TypeScript Documentation**: https://nextjs.org/docs/app/building-your-application/configuring/typescript
- **Error Messages Reference**: https://nextjs.org/docs/messages/
- **Specific Error Pages**:
  - Invalid href: https://nextjs.org/docs/messages/invalid-href-passed
  - Invalid anchor usage: https://nextjs.org/docs/messages/invalid-new-link-with-extra-anchor
  - No HTML link: https://nextjs.org/docs/messages/no-html-link-for-pages

---

## 3. W3C Link Validation

### Finding: OFFICIAL W3C LINK CHECKER EXISTS

The W3C (World Wide Web Consortium) provides an official link validation tool with both web-based and command-line interfaces.

### Web Interface

**URL:** https://validator.w3.org/checklink

**Description:**
The W3C Link Checker validates links and anchors in web pages or full websites by examining issues in links, anchors, and referenced objects.

**Features:**
- Validates HTML, XHTML documents, and CSS style sheets
- Extracts and verifies all anchors and links
- Checks that no anchor is defined twice
- Verifies all links are dereferenceable (including fragments)
- Warns about HTTP redirects (3xx status codes)
- Detects broken links (4xx, 5xx status codes)

**Configuration Options:**
- **Summary mode**: Display only essential findings
- **Redirect handling**: Show all redirects or only directory redirects
- **Header control**: Optionally exclude Accept-Language and Referer headers
- **Recursive checking**: Enable site-wide scanning with configurable depth limits
- **Cookie preferences**: Save configuration choices locally

**Usage:**
1. Navigate to https://validator.w3.org/checklink
2. Enter the URL of the page or site to check
3. Configure options as needed
4. Submit for validation
5. Review results showing broken links, redirects, and anchor issues

### Command-Line Interface

**Installation:**

The W3C Link Checker is a Perl-based tool available via CPAN:

```bash
sudo perl -MCPAN -e 'install W3C::LinkChecker'
```

(Omit `sudo` if installing from an administrator account)

**Usage:**

```bash
checklink --help  # Display command-line options
perldoc checklink # View comprehensive documentation
man checklink     # View man page (Unix systems)
```

**Dependencies:**

Required:
- `W3C-LinkChecker` (core tool)
- `HTML-Parser` (version 3.20+)
- `libwww-perl` (version 5.802+)
- `URI` (version 1.31+)
- `Time-HiRes`
- `CSS-DOM` (version 0.09+)

Optional (recommended):
- `TermReadKey` (for password input in CLI)
- `Net-IP` (for restricting access to private addresses)

**Configuration:**

Optional configuration file locations:
- `/etc/w3c/checklink.conf`
- Custom path via `W3C_CHECKLINK_CFG` environment variable

**Advanced Features:**
- Respects proxy settings through environment variables
- Honors `robots.txt` exclusion rules
- User agent: `W3C-checklink`
- Supports HTTP basic authentication
- Requires explicit SSL/TLS support via libwww-perl for `https` links

### Best Practices

The W3C recommends:
1. Validate markup first using the W3C Markup Validator: https://validator.w3.org/
2. Validate CSS using the W3C CSS Validator: https://jigsaw.w3.org/css-validator/
3. Then run link validation for optimal results

### Source Documentation

- **W3C Link Checker Web Interface**: https://validator.w3.org/checklink
- **Command-Line Documentation**: https://dev.w3.org/perl/modules/W3C/LinkChecker/docs/checklink
- **W3C Validators and Tools Page**: https://www.w3.org/developers/tools/
- **About W3C Link Checker**: https://validator.w3.org/about.html
- **Version**: 5.0.0 (©1999-2025 W3C)

---

## Summary of Official Tools

| Source | Tool/Feature | Status | Type | Validation Scope |
|--------|--------------|--------|------|------------------|
| **GitHub Actions** | Link checker in `actions/*` namespace | ❌ Does not exist | N/A | N/A |
| **Next.js** | Statically Typed Links (`typedRoutes`) | ✅ Exists | Build-time TypeScript validation | Internal application routes only |
| **W3C** | W3C Link Checker | ✅ Exists | Web-based & CLI tool | All links (internal & external) |

---

## Recommendations

### For Next.js Projects:
1. **Enable `typedRoutes`** for compile-time validation of internal route references
2. **Use W3C Link Checker** (web or CLI) for comprehensive validation of all links, including external URLs
3. **Implement manual testing** for critical external links
4. **Consider third-party GitHub Actions** for automated CI/CD link checking (not officially supported)

### For Static Sites (General):
1. **Use W3C Link Checker** as the only official validation tool available
2. **Validate markup first** using W3C Markup Validator before link checking
3. **No official GitHub Action exists** - must use community actions or custom scripts
4. **Manual validation required** if official tools only policy is enforced

### For Production Systems:
If a strict "official tools only" policy is in place:
- **Next.js projects**: Use `typedRoutes` + W3C Link Checker
- **Other frameworks**: W3C Link Checker only (web interface or CLI)
- **CI/CD pipelines**: Custom scripts calling W3C Link Checker CLI or manual validation

---

## Research Methodology

This research was conducted on 2025-10-28 using the following approach:

1. **GitHub Actions Research:**
   - Searched GitHub official actions organization
   - Queried GitHub REST API for complete repository list
   - Reviewed GitHub Enterprise documentation
   - Searched GitHub Marketplace to confirm no official actions exist

2. **Next.js Research:**
   - Searched official Next.js documentation at nextjs.org
   - Reviewed TypeScript configuration documentation
   - Examined error message documentation
   - Extracted configuration examples from official docs

3. **W3C Research:**
   - Searched W3C official validators page
   - Retrieved W3C Link Checker documentation
   - Reviewed command-line usage documentation
   - Examined installation and configuration guides

**Research Scope Limitation:**
This research **excluded** third-party, community-maintained, or unofficial tools, regardless of popularity or quality. Only tools and features officially maintained by GitHub, Vercel/Next.js, and W3C were documented.

---

## Conclusion

**Official link validation tools are LIMITED:**

- ❌ **GitHub provides NO official link validation action**
- ✅ **Next.js provides Statically Typed Links for internal route validation**
- ✅ **W3C provides a comprehensive Link Checker (web + CLI)**

**For comprehensive link validation:**
Projects must combine Next.js's `typedRoutes` (for internal links) with W3C's Link Checker (for all links including external) OR implement manual validation processes OR use non-official third-party tools.

**If strict "official tools only" policy applies:**
Manual link validation is required for GitHub Actions workflows, as no official action exists for this purpose.
