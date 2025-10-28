# Link Validation Methods Research - 2025-10-28

## Task

Research official documentation for link validation tools and methods for static websites.

**Requirements:**
- Look for official GitHub Actions for link checking (actions/*)
- Check Next.js documentation for link validation features
- Find W3C or web standards for link validation
- ONLY include officially documented tools/methods
- If no official tools exist, document that manual validation is required

**Output Required:**
Create a markdown document with:
1. Official link validation GitHub Actions (if they exist)
2. Next.js built-in link validation (if it exists)
3. W3C validator information (if applicable)
4. Exact configuration examples
5. Source URLs
6. If none exist: explicit statement that no official tools found

Write findings to: /Users/brett/src/ma-collective/maenifold/docs/status-tracking/link-validation-methods.md

Ma Protocol: No assumptions. If official tools don't exist, say so explicitly.

## Task Restated

I need to research and document ONLY officially documented link validation tools and methods from:
1. GitHub's official Actions (specifically under the `actions/*` namespace)
2. Next.js official documentation
3. W3C or web standards bodies

**Assumptions:**
- "Official" means maintained by the organizations themselves (GitHub, Vercel/Next.js team, W3C)
- I will NOT include third-party community tools, even if popular
- If no official tools exist in any category, I must explicitly state this
- I will provide exact URLs to official documentation sources
- Configuration examples must come from official documentation only

**What I will do:**
1. Search GitHub's official documentation for link validation Actions
2. Search Next.js official documentation for built-in link validation features
3. Search W3C documentation for link validation standards/validators
4. Document findings with exact source URLs
5. Provide configuration examples only from official sources
6. Explicitly state where no official tools exist

**What I will NOT do:**
- Include third-party or community-maintained tools
- Make assumptions about tool existence
- Provide configuration examples without official source documentation

## Plan

1. Search for official GitHub Actions for link validation
2. Search Next.js documentation for link validation features
3. Search W3C documentation for link validation standards
4. Compile findings into structured markdown
5. Update this document with final results

## Status

✅ **RESEARCH COMPLETED**

Research completed on all three areas:
1. ✅ GitHub Actions official actions/* namespace - CONFIRMED: No official link checker exists
2. ✅ Next.js documentation - CONFIRMED: Built-in Statically Typed Links feature exists
3. ✅ W3C documentation - CONFIRMED: Official W3C Link Checker exists

✅ **DOCUMENT CREATED**

Final comprehensive findings document created at:
`/Users/brett/src/ma-collective/maenifold/docs/status-tracking/link-validation-research-findings.md`

## Key Findings Summary

### 1. GitHub Actions (actions/* namespace)
**Result:** ❌ NO OFFICIAL LINK CHECKER EXISTS

- Reviewed all 80+ repositories in the official `actions/*` namespace
- Confirmed via GitHub API and official documentation
- No link validation functionality in official actions
- Third-party community actions exist but are outside research scope

**Evidence:**
- GitHub Actions Org: https://github.com/actions
- API Query: https://api.github.com/orgs/actions/repos
- Official actions include: checkout, setup-node, setup-python, cache, upload-artifact, etc.
- None provide link validation

### 2. Next.js Built-in Features
**Result:** ✅ STATICALLY TYPED LINKS (typedRoutes) EXISTS

- Official feature for compile-time route validation
- Uses TypeScript to validate internal route references
- Validates `next/link` href props and router navigation calls
- Configuration: `typedRoutes: true` in next.config.ts

**Limitations:**
- Only validates internal application routes
- Does not check external URLs
- Requires TypeScript enabled

**Source:** https://nextjs.org/docs/app/building-your-application/configuring/typescript

### 3. W3C Link Validation
**Result:** ✅ W3C LINK CHECKER EXISTS

- Official W3C tool for comprehensive link validation
- Available as web interface AND command-line tool
- Validates all links (internal and external)
- Checks HTML, XHTML, CSS documents

**Access:**
- Web: https://validator.w3.org/checklink
- CLI: `perl -MCPAN -e 'install W3C::LinkChecker'`
- Documentation: https://dev.w3.org/perl/modules/W3C/LinkChecker/docs/checklink

**Features:**
- Detects broken links (4xx, 5xx)
- Warns about redirects (3xx)
- Validates anchors
- Recursive site checking
- Configurable depth and scope

## Final Recommendations

**For Next.js projects:**
- Enable `typedRoutes` for internal route validation
- Use W3C Link Checker for external link validation
- Consider third-party GitHub Actions for CI/CD (not officially supported)

**For static sites (general):**
- W3C Link Checker is the only official validation tool
- No official GitHub Action exists
- Manual validation or third-party tools required for automation

**For strict "official tools only" policies:**
- Next.js: Use `typedRoutes` + W3C Link Checker
- Other frameworks: W3C Link Checker only
- GitHub Actions: NO official option - manual validation required

## Task Completion

✅ All requirements met:
1. ✅ Researched official GitHub Actions (none found)
2. ✅ Researched Next.js documentation (typedRoutes found)
3. ✅ Researched W3C standards (Link Checker found)
4. ✅ Documented exact configuration examples
5. ✅ Provided source URLs for all findings
6. ✅ Explicitly stated where no official tools exist
7. ✅ Created comprehensive markdown document

**Output location:** `/Users/brett/src/ma-collective/maenifold/docs/status-tracking/link-validation-research-findings.md`
