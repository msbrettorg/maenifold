# PRD Scope Gap Analysis: Maenifold Documentation Site
**Date**: 2025-10-28
**Reviewer**: Claude (Engineering)
**Focus**: Missing requirements, edge cases, and overlooked features

---

## Executive Summary

The PRD is **75% complete** with solid foundations but has **significant gaps** in:
1. **Error handling & resilience** (no 404 strategy, build failure recovery)
2. **Site health/observability** (no monitoring, error tracking, link validation)
3. **Content maintenance processes** (no update workflow, versioning strategy)
4. **Edge cases** (orphaned pages, content conflicts, asset delivery failures)
5. **Testing & validation** (no acceptance test plan, deployment verification)

**Critical Priority**: 3 gaps that block approval
**Important Priority**: 8 gaps that should be addressed
**Nice-to-Have Priority**: 6 gaps that can defer

---

## CRITICAL GAPS (Must Address Before Approval)

### 1. No Build Failure Recovery Plan
**Impact**: High | **Likelihood**: Medium | **Current Status**: Unspecified

**What's Missing**:
- What happens if GitHub Actions build fails? (e.g., content extraction script breaks)
- No rollback strategy if deployment breaks site
- No mention of build logs retention or debugging aids
- No recovery procedure for partial deployments

**Questions for PM**:
- Should we maintain a previous "last known good" version?
- How long to retain failed build artifacts for debugging?
- Who gets notified on build failure?
- Should we have a manual deployment rollback mechanism?

**Recommendation**: Add to PRD:
```markdown
### Build Failure Recovery
- GitHub Actions keeps last 90 days of build artifacts
- Failed builds trigger GitHub email notification
- Manual rollback: Re-run deploy action with previous commit SHA
- Maintain git tags for each successful production deployment
```

---

### 2. No 404 Error Handling Strategy
**Impact**: High | **Likelihood**: High | **Current Status**: Missing entirely

**What's Missing**:
- No custom 404 page mentioned
- No strategy for renamed/moved content
- No mention of GitHub Pages 404 behavior
- No redirects for deprecated URLs

**GitHub Pages Reality Check**:
- GitHub Pages serves `/404.html` automatically (if exists)
- Next.js static export creates `404.html` - should verify this works
- No server-side redirects possible on GitHub Pages

**Questions for PM**:
- Should we create a custom 404 page with navigation back?
- Do we need redirect mapping for any legacy URLs?
- Should search index include "did you mean" suggestions?

**Recommendation**: Add to PRD:
```markdown
### 404 Error Handling
- Create custom `404.tsx` page in `app/` directory
  - Display friendly message: "Page not found"
  - Show search box for users to find content
  - Link to site index/search
  - GitHub Pages automatically serves when route not found
- No server redirects possible (static site limitation)
- Broken links discovered during testing must be fixed before deployment
```

---

### 3. No Link Validation & Health Check Process
**Impact**: Critical | **Likelihood**: High | **Current Status**: Missing entirely

**What's Missing**:
- No link validation mentioned in testing phase
- No strategy for detecting broken links between pages
- No external link checking (e.g., to GitHub source files)
- No continuous health monitoring post-deployment
- No process for maintaining links as source files change

**Questions for PM**:
- Should we validate all internal links before deploying?
- Do we check external links to GitHub source repos?
- What happens when source `.md` files move/get renamed?
- Should we have a scheduled link audit post-launch?

**Recommendation**: Add to PRD:
```markdown
### Link Validation & Health Checks
**Pre-Deployment**:
- Lychee or similar tool validates all links in `out/` directory
- Internal links must return 200
- External links (GitHub) checked with timeout handling
- Build fails if critical links broken (configurable threshold)

**Post-Deployment**:
- Weekly GitHub Actions job runs link validation on live site
- Slack/email notification if links broken
- Report sent to maintainer for remediation

**Source File Changes**:
- If source files move/rename, update hardcoded links in documentation
- Add lint rule to catch broken markdown links in content
```

---

## IMPORTANT GAPS (Should Address)

### 4. No Content Update/Maintenance Workflow
**Impact**: Medium | **Likelihood**: High | **Current Status**: Completely missing

**What's Missing**:
- How do we add new tools/workflows to the site?
- What's the process when tool documentation changes?
- Who maintains the site after launch?
- No version control for content changes
- No approval process for documentation updates

**Questions for PM**:
- When new tools are added to `src/assets/usage/tools/`, how does site get updated?
- Is site rebuilt on every `maenifold` repo push, or on schedule?
- Who owns content accuracy? (Contributor, PM, Tech Lead?)
- Should changes to source content trigger automatic site rebuild?

**Recommendation**: Add to PRD:
```markdown
## Content Maintenance Process

### Adding New Tools
1. New tool markdown added to `/src/assets/usage/tools/`
2. Site rebuild triggered (manual or automatic on merge to main)
3. New tool page auto-generated from markdown
4. Tool browser page automatically includes new card
5. Search index rebuilt to include new content

### Updating Tool Documentation
1. Tool markdown edited in source repository
2. Changes pushed to main
3. Site rebuild triggered
4. Updated page deployed within 5 minutes

### Content Ownership
- Source of Truth: `/src/assets/usage/tools/`, `/src/assets/workflows/`, etc.
- Site Pages: Auto-generated from source (no manual editing)
- Content accuracy: Verified by tool/workflow creator before merge
- Site maintenance: PM or designated technical lead

### Approval Process
- Changes to core docs/philosophy: Require PM approval (code review)
- Tool docs: Assumed approved by tool author (merge to main = publish)
- Workflow docs: Approved by workflow author
```

---

### 5. No Content Versioning or Changelog
**Impact**: Medium | **Likelihood**: Medium | **Current Status**: Not mentioned

**What's Missing**:
- Site version tracking (e.g., v1.0, v1.1)
- Changelog for site documentation changes
- Ability to track when pages were last updated
- No "Last updated" dates on pages
- No deprecation path for old content

**Questions for PM**:
- Should pages display "Last updated: YYYY-MM-DD"?
- Do we need a CHANGELOG for the site itself?
- Should we version-pin the site (e.g., maintain /v1.0/ routes)?
- Can we deprecate old documentation pages?

**Recommendation**: Add to PRD:
```markdown
### Content Versioning
- Site versioning: Single version for v1.0 (no /v1.0/ routes)
- Page-level: Display git commit date and last author
- Future: If v2.0 needed, create separate versioned deployment
- No changelog needed for v1.0 (first launch)

### Page Metadata
- Every page displays: "Last updated: Oct 28, 2025"
- Pulled from file git history or frontmatter
- Enable users to know content freshness
```

---

### 6. No Search Result Relevance or Ranking
**Impact**: Medium | **Likelihood**: Medium | **Current Status**: Uses Pagefind (not specified how)

**What's Missing**:
- No specification of how search ranks results
- No mention of search result preview/excerpt
- No handling of typos or "did you mean"
- No analytics on what users search for (expected per Ma Protocol, but not explicitly stated)

**Questions for PM**:
- Should search results show page excerpts/context?
- Should we highlight search terms in results?
- How many results per page?
- Pagefind ranking: alphabetical? by frequency? by heading level?

**Recommendation**: Add to PRD:
```markdown
### Search Implementation Details
- Tool: Pagefind (static, client-side indexing)
- Build: Search index generated during static export
- Results: Title + 150-char excerpt showing query context
- Highlighting: Search term bolded in results
- Ranking: Pagefind default (word frequency + heading hierarchy)
- Pagination: Show 10 results per page
- No typo tolerance or fuzzy matching (keep it simple)
```

---

### 7. No Image/Asset Delivery or Optimization Strategy
**Impact**: Medium | **Likelihood**: Medium | **Current Status**: Partially specified

**What's Missing**:
- What image formats are supported? (SVG, JPG, PNG, WebP?)
- No image optimization process (is graph.jpeg large?)
- No responsive image strategy for different screen sizes
- No fallback if images fail to load
- No alt text strategy (WCAG requirement but not detailed)

**GitHub Pages Reality Check**:
- 1GB total repo size limit (must check image file sizes)
- No CDN, images served from repo (may be slow)
- No automatic format conversion

**Questions for PM**:
- Should we convert graph.jpeg to modern formats (WebP)?
- What's the max acceptable image file size?
- Do we need responsive images (srcset)?
- Should we embed SVGs or use `<img>`?

**Recommendation**: Add to PRD:
```markdown
### Image & Asset Management
- Formats: SVG (preferred for logos), JPG (optimized <500KB), PNG (optimized <300KB)
- Locations: `/public/assets/branding/`, `/public/demo-artifacts/`
- Optimization: Use image compression tool before commit
  - Logo: Keep as SVG (scalable)
  - Graph: Convert to WebP with JPG fallback
  - Previews: Compress to max 100KB each
- Responsive: Use `<picture>` tag for mobile fallbacks
- Alt text: Required for all images (WCAG AA)
- Failure handling: Provide text fallback if images don't load
```

---

### 8. No Mobile-Specific Testing Plan
**Impact**: Medium | **Likelihood**: High | **Current Status**: Mentioned but not detailed

**What's Missing**:
- No specific mobile screen sizes to test
- No mention of touch target sizes (WCAG requires 44x44px minimum)
- No hamburger menu detailed design
- No testing of dark mode on mobile (brightness/contrast)
- No network throttling tests for slow connections

**Questions for PM**:
- Minimum screen size support? (iPhone SE: 375px, iPad mini: 768px)
- Should navigation menu collapse at specific breakpoint?
- Touch target sizes: 44x44px minimum?
- Test on real devices or just responsive emulation?

**Recommendation**: Add to PRD:
```markdown
### Mobile Testing Requirements
- Screen sizes: 375px (iPhone SE), 768px (tablet), 1024px, 1440px+
- Touch targets: Minimum 44x44px for all clickable elements
- Navigation: Hamburger menu below 768px breakpoint
- Code blocks: Horizontal scroll or collapse/expand toggle
- Images: Responsive with lazy loading for performance
- Dark mode: Test contrast on various brightness settings
- Network: Simulate 4G throttling to verify performance
```

---

### 9. No Accessibility Audit or WCAG Details
**Impact**: Medium | **Likelihood**: High | **Current Status**: Mentioned vaguely as "WCAG AA"

**What's Missing**:
- No specific WCAG AA success criteria list
- No mention of keyboard navigation testing (Tab, Enter, Escape)
- No focus indicator design specified
- No color contrast ratios provided
- No screen reader testing plan
- No skiplinks for keyboard navigation

**Questions for PM**:
- Which WCAG AA criteria are critical for this site?
- Should we include skip-to-content link?
- Focus indicator color/style?
- Heading hierarchy validation?

**Recommendation**: Add to PRD:
```markdown
### Accessibility Requirements (WCAG AA)

**Keyboard Navigation**:
- All interactive elements reachable via Tab key
- Focus order logical and visible
- Focus indicator: 3px outline, high contrast color
- Escape key closes menus/modals
- Enter/Space activates buttons

**Screen Readers**:
- Semantic HTML: Use <button>, <nav>, <main>, <article>
- ARIA labels for icon buttons
- Table markup for tool/workflow browsers
- Skip link: "Skip to main content"

**Color & Contrast**:
- Foreground/background contrast ratio ≥ 4.5:1 (AA standard)
- Don't rely on color alone to convey information
- Test with WAVE, Axe, or Lighthouse audit

**Testing**:
- Browser: Chrome + NVDA (free screen reader)
- Automated: Lighthouse accessibility audit
- Manual: Keyboard-only navigation test
```

---

### 10. No Performance Monitoring or Observability
**Impact**: Medium | **Likelihood**: Medium | **Current Status**: Partially specified

**What's Missing**:
- No strategy for measuring site performance post-launch
- "Lighthouse > 90" is a goal but no testing process
- No Core Web Vitals monitoring (LCP, FID, CLS)
- No error tracking (404s, missing content, build failures)
- No user experience metrics (but expected per Ma Protocol)

**Questions for PM**:
- How do we know if site is broken post-deployment? (Error logs?)
- Should we monitor Core Web Vitals?
- Where do we check Lighthouse scores? (CI, manual, scheduled?)
- GitHub Pages provides no analytics - acceptable?

**Recommendation**: Add to PRD:
```markdown
### Performance & Monitoring

**Build-Time Checks**:
- Lighthouse audit runs on every build (must achieve >90)
- Link validation checks all internal links
- Build fails if Lighthouse < 90 or critical links broken

**Post-Deployment**:
- GitHub Pages access logs available in settings
- No external analytics per Ma Protocol
- Manual monthly check: Test on real devices, Chrome DevTools
- Baseline: FCP <1.5s, LCP <2.5s, CLS <0.1

**Error Tracking**:
- 404s: Monitor in GitHub Pages logs
- Missing assets: GitHub Actions build would catch
- Broken links: Weekly automated validation
```

---

## NICE-TO-HAVE GAPS (Can Defer)

### 11. No Breadcrumb Navigation Specification
**Impact**: Low | **Likelihood**: Low | **Current Status**: Mentioned vaguely

PRD says "breadcrumbs for deep pages" but:
- No specification of which pages show breadcrumbs
- No visual design provided
- No behavior on root pages (should they show?)

**Recommendation**: Defer to implementation, but document pattern.

---

### 12. No Table of Contents for Long Pages
**Impact**: Low | **Likelihood**: Medium | **Current Status**: Mentioned but not detailed

PRD specifies "Table of contents for long pages" but:
- No definition of "long" (how many headings?)
- No styling/placement specified
- No auto-generation process

**Recommendation**: Generate TOC from markdown heading hierarchy, place at top-right or sticky sidebar.

---

### 13. No Copy Button Implementation Detail
**Impact**: Low | **Likelihood**: Low | **Current Status**: Mentioned but not detailed

PRD says "copy button for code blocks" but:
- No specification of button placement/styling
- No handling of different code languages
- No feedback on successful copy (toast notification?)

**Recommendation**: Use simple "Copy" button, show "Copied!" feedback for 2 seconds.

---

### 14. No Favicon or App Icon Strategy
**Impact**: Low | **Likelihood**: High | **Current Status**: Not mentioned

**What's Missing**:
- No favicon specified (usually in `public/favicon.ico`)
- No Apple icon for bookmarking
- No manifest.json for PWA

**Recommendation**: Add to PRD:
```markdown
### Branding Assets (Low Priority)
- Favicon: Generate from maenifold-logo.svg (32x32)
- Apple touch icon: 180x180 for bookmarking on iOS
- Manifest.json: Optional, can add if PWA features desired
```

---

### 15. No robots.txt or Sitemap Strategy
**Impact**: Low | **Likelihood**: Low | **Current Status**: Not mentioned

**What's Missing**:
- No robots.txt to control search engine crawling
- No sitemap.xml (helps search engines index)
- Not critical for GitHub Pages but good practice

**Recommendation**: Add optional sitemap.xml generation with `next-sitemap` package.

---

### 16. No RSS Feed Implementation
**Impact**: Low | **Likelihood**: Low | **Current Status**: Mentioned in philosophy but not spec'd

PRD mentions Ma Protocol and no telemetry, but:
- No mention of RSS feed for content updates
- "Could add later" - mark as future enhancement

**Recommendation**: Defer to v1.1 if requested.

---

## MISSING FEATURES TO CHECK

### Feature: Code Copy Button
**Status**: ✓ Mentioned, but needs implementation detail
**Risk**: Low - straightforward implementation

### Feature: Syntax Highlighting Languages
**Status**: ✓ Mentioned (Shiki) but no language coverage specified
**Risk**: Low - Shiki supports 200+ languages by default
**Recommendation**: Specify in implementation which languages to highlight (JS, TS, YAML, JSON, Markdown, etc.)

### Feature: Search Result Preview/Highlighting
**Status**: ✓ Identified as gap (#6)

### Feature: Print Styles
**Status**: ✗ Not mentioned
**Risk**: Low - can use CSS media query `@media print`
**Recommendation**: Add print stylesheet for better readability when printing
```css
@media print {
  nav, footer { display: none; }
  main { max-width: 100%; }
}
```

### Feature: Dark Mode Preference Sync
**Status**: ✓ Mentioned (localStorage) but not detailed
**Risk**: Low - standard implementation
**Recommendation**: Check browser prefers-color-scheme, fall back to localStorage

### Feature: Responsive Images with srcset
**Status**: ✗ Not mentioned
**Risk**: Low - only needed if multiple image sizes
**Recommendation**: Not critical for static site, can defer

---

## EDGE CASES NOT ADDRESSED

### Edge Case 1: What if tool markdown has broken frontmatter?
**Current Plan**: Unknown
**Risk**: Build could fail silently or content could be lost
**Recommendation**: Add validation in content extraction script:
```markdown
- Parse tool markdown files with front-matter validator
- Log errors for invalid files
- Build fails if any tool file is malformed
```

### Edge Case 2: What if workflow JSON is missing required fields?
**Current Plan**: Unknown
**Risk**: Pages render with missing data
**Recommendation**: Add schema validation:
```markdown
- Validate each workflow JSON against schema
- Required fields: name, description, triggers, steps
- Build fails if validation fails
```

### Edge Case 3: What if image file is renamed/moved?
**Current Plan**: Unknown
**Risk**: Broken image references
**Recommendation**: Use relative paths and validate during build:
```markdown
- Image references use relative paths
- Build validates all image files exist
- Build fails if any image missing
```

### Edge Case 4: What if GitHub Pages is down?
**Current Plan**: Unknown (external service)
**Risk**: Site unavailable
**Recommendation**: Document in README as external dependency:
```markdown
### Dependencies
- GitHub Pages: Hosting platform (external service)
- GitHub Actions: Build automation (external service)
```

### Edge Case 5: What if code examples are very long?
**Current Plan**: Unknown
**Risk**: Code blocks could break layout
**Recommendation**: Add CSS for code block scrolling:
```css
pre {
  max-height: 400px;
  overflow-y: auto;
  horizontal-scroll if needed
}
```

---

## OPEN QUESTIONS FOR PM (CONSOLIDATED)

### Critical Questions (Require Answers)
1. **Build failure recovery**: What's our rollback strategy if site breaks?
2. **404 handling**: Custom 404 page with search integration?
3. **Link validation**: Automated link checking pre-deployment?
4. **Content updates**: How do we add new tools/workflows to the site?
5. **Maintenance ownership**: Who maintains the site after launch?

### Important Questions (Strongly Recommended to Answer)
6. **Mobile testing**: What devices/screen sizes are required for testing?
7. **Accessibility scope**: Which WCAG AA criteria are mandatory?
8. **Image optimization**: Should we convert images to WebP?
9. **Search preview**: Should results show page excerpts?
10. **Performance monitoring**: How do we verify performance post-launch?

### Nice-to-Have Questions (Can Address in Implementation)
11. **Breadcrumb design**: File path style or navigation style?
12. **Copy button styling**: Toast notification on copy?
13. **Print styles**: Should site be printable?
14. **Favicon**: Generate from logo or use different image?
15. **Sitemap**: Auto-generate sitemap.xml for SEO?

---

## RECOMMENDED PRD ADDITIONS

### Addition 1: Build & Deployment Resilience Section
```markdown
## Build Resilience & Recovery

### Build Failure Handling
- Failed builds: GitHub Actions notifies via email
- Cause analysis: Check build logs in Actions tab
- Rollback: Re-run deploy on previous commit
- Production tags: Tag each successful deployment with `site-v1.0.0` format

### Deployment Verification
- Pre-deploy: Verify all critical links in `out/` directory
- Post-deploy: Test main pages on GitHub Pages URL
- Smoke test: Verify home, docs, tools, workflows pages load
- Check dark mode toggle works
```

### Addition 2: Content Maintenance & Governance Section
```markdown
## Site Maintenance & Content Governance

### Adding New Tools
1. Contributor creates tool markdown in `/src/assets/usage/tools/[tool-name].md`
2. PR review: Tech lead verifies documentation accuracy
3. Merge to main: Triggers automatic site rebuild
4. New page live within 5 minutes

### Updating Existing Content
- Changes to source files automatically update site pages
- Changes to core philosophy docs: Require PM approval
- Changes to tool docs: Assumed approved by tool author

### Content Accuracy
- Source of truth: `/src/assets/usage/tools/`, etc.
- Site pages: Auto-generated (no manual editing)
- Outdated content: Mark with deprecation notice and timestamp
```

### Addition 3: Error Handling & Recovery Section
```markdown
## Error Handling & Site Health

### 404 Page
- Custom 404.tsx provides friendly messaging
- Includes search box for users to find content
- Links to site index and documentation browser

### Broken Links
- Internal links validated during build
- External links checked with timeout handling
- Build fails if critical links broken
- Manual link audit monthly or after major updates

### Image Failures
- Provide text fallback if images don't load
- Images compress to <500KB (within repo limits)
- Use lazy loading to defer non-critical images
```

### Addition 4: Accessibility & Testing Section
```markdown
## Accessibility & Quality Assurance

### WCAG AA Compliance
- Keyboard navigation: All elements reachable via Tab
- Focus indicators: Visible 3px outline
- Color contrast: ≥4.5:1 for all text
- Semantic HTML: Proper heading hierarchy
- Screen reader: Compatible with NVDA/VoiceOver
- Skip links: "Skip to main content" link at top

### Mobile Testing (375px - 1440px+)
- Touch targets: 44x44px minimum
- Navigation: Hamburger menu <768px
- Code blocks: Scroll or collapse on mobile
- Images: Responsive with srcset

### Performance Testing
- Lighthouse: >90 on every build
- Core Web Vitals: LCP <2.5s, FID <100ms, CLS <0.1
- Network simulation: 4G throttling test
```

---

## Summary by Priority

| Priority | Count | Examples |
|----------|-------|----------|
| **CRITICAL** | 3 | Build failure recovery, 404 handling, link validation |
| **IMPORTANT** | 8 | Content maintenance, versioning, search details, images, mobile testing, accessibility, monitoring, responsive design |
| **NICE-TO-HAVE** | 6 | Breadcrumbs, TOC, copy buttons, favicon, robots.txt, RSS |
| **EDGE CASES** | 5 | Broken frontmatter, missing JSON fields, missing images, long code blocks |

---

## Recommended Next Steps

### Immediately (Before Implementation)
1. **PM approval**: Get answers to Critical and Important questions
2. **Add to PRD**: Incorporate recommended additions into final PRD
3. **Clarify scope**: Document Phase 1 build failure recovery approach

### During Implementation
1. **Phase 1**: Add 404.tsx, build error logging, link validation
2. **Phase 2**: Document content update process in README
3. **Phase 7**: Implement Lighthouse audit, accessibility testing

### Post-Launch
1. **Week 1**: Manual smoke test on real devices
2. **Week 1**: Run accessibility audit with WAVE
3. **Monthly**: Validate all links, review build logs
4. **Quarterly**: Full Lighthouse & Core Web Vitals audit

---

## Conclusion

The PRD provides a **solid foundation** (75% complete) with clear success criteria and technical specifications. However, it needs **refinement in operational resilience, content governance, and testing strategy** before it can be considered "production-ready."

**Key insight**: The PRD is excellent at *what* to build but light on *how to maintain* and *what if it breaks*. Adding the three critical sections (resilience, governance, testing) will convert this from a build specification into a sustainable operations plan.

**Confidence in Success**: Medium-High
- If CRITICAL gaps addressed: High confidence
- Without addressing critical gaps: Risk of launch-day surprises (broken links, no rollback strategy)

---

**Review Status**: Complete
**Generated**: 2025-10-28
**Reviewer**: Claude (Engineering)
