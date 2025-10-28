# Product Requirements Document: Maenifold GitHub Pages Site

**Date**: 2025-10-28
**Product Manager**: Claude (Blue Hat)
**Status**: Draft for Approval

---

## Executive Summary

Build a **Simple, Lovable, Complete** GitHub Pages documentation site for maenifold that:
- Contains ALL existing content from source files (zero fabrication)
- Works reliably on GitHub Pages without constant rework
- Provides full navigation between all sections
- Reflects the sophisticated, technical nature of the product
- Is maintainable and won't require architectural changes

This is a **complete v1.0**, not an MVP.

---

## Problem Statement

Previous attempts at the maenifold site failed because:
1. Partial implementations shipped without complete navigation
2. Markdown/layout approaches that don't work on GitHub Pages
3. Need for constant rework due to poor architecture decisions
4. Violations of SLC principle (shipping incomplete features)

**User Pain**: "tired of re-doing this each time things need formatting"

---

## Success Criteria

### Definition of "Complete"
✅ A visitor can discover and navigate to ALL content
✅ Every piece of existing source content is accessible
✅ Site functions as a cohesive whole, not disconnected pages
✅ Dark mode works (branding requirement)
✅ Builds successfully for GitHub Pages static export
✅ No broken links or missing pages
✅ Mobile responsive

### Definition of "Simple"
✅ Clean, uncluttered design
✅ Fast page loads (minimal JavaScript)
✅ Clear information hierarchy
✅ Easy to maintain and update

### Definition of "Lovable"
✅ Beautiful typography and spacing
✅ Smooth dark mode that matches branding
✅ Syntax-highlighted code examples
✅ Reflects the technical sophistication of maenifold

---

## Content Inventory (What EXISTS)

### Source Files Verified
- `/Users/brett/src/ma-collective/maenifold/README.md` (229 lines)
- `/Users/brett/src/ma-collective/maenifold/docs/README.md` (520 lines)
- `/Users/brett/src/ma-collective/maenifold/src/assets/usage/tools/*.md` (26 files)
- `/Users/brett/src/ma-collective/maenifold/src/assets/workflows/*.json` (28 files)
- `/Users/brett/src/ma-collective/maenifold/src/assets/roles/*.json` (7 files)
- `/Users/brett/src/ma-collective/maenifold/src/assets/colors/*.json` (7 files)
- `/Users/brett/src/ma-collective/maenifold/src/assets/perspectives/*.json` (12 files)
- `/Users/brett/src/ma-collective/maenifold/assets/demo-artifacts/**/*.md` (15 files)
- `/Users/brett/src/ma-collective/maenifold/assets/branding/` (2 assets: logo.svg, graph.jpeg)

**Total Pages**: ~71 pages of content minimum

---

## Site Structure (Information Architecture)

```
maenifold.com/
├── / (Home/Landing)
│   └── Hero + What maenifold does + Demo story
│
├── /start (Quick Start)
│   └── Install + MCP config + CLI examples
│
├── /docs/
│   ├── architecture (Cognitive Stack)
│   ├── philosophy (Ma Protocol)
│   └── technical-specs
│
├── /tools/
│   ├── index (Tool browser - all 26 tools)
│   ├── sequential-thinking
│   ├── workflow
│   ├── write-memory
│   └── ... (23 more tool pages)
│
├── /workflows/
│   ├── index (Workflow browser - all 28 workflows)
│   ├── agentic-dev
│   ├── design-thinking
│   ├── pm-lite
│   └── ... (25 more workflow pages)
│
├── /cognitive-assets/
│   ├── index (Overview of 54 frameworks)
│   ├── roles/ (7 role pages)
│   ├── colors/ (7 color pages)
│   └── perspectives/ (12 perspective pages)
│
└── /demo/
    ├── index (Demo overview + timeline)
    ├── part1-pm-lite/
    └── part2-agentic-slc/
```

---

## What We WILL Build

### Core Pages (Must Have)
1. **Landing Page** (`/`)
   - Hero section with logo
   - "What maenifold does" narrative
   - Demo story (25 agents, 68 minutes, real bug found)
   - Call-to-action to Quick Start
   - Visual: graph.jpeg

2. **Quick Start** (`/start`)
   - Install command
   - MCP configuration (JSON examples)
   - CLI examples
   - Link to tools documentation

3. **Architecture** (`/docs/architecture`)
   - Cognitive Stack diagram
   - Memory Layer explanation
   - Graph Layer explanation
   - Reasoning Layer explanation

4. **Technical Specs** (`/docs/technical-specs`)
   - Language, database, vector dimensions
   - Search algorithm details
   - Scale testing results
   - MCP compliance

5. **Philosophy** (`/docs/philosophy`)
   - Ma Protocol principles
   - "What We Don't Do" manifesto
   - Design philosophy

### Tool Documentation (Must Have)
6. **Tool Browser** (`/tools/`)
   - Searchable/filterable list of all 26 tools
   - Card layout with descriptions
   - Links to individual tool pages

7. **Individual Tool Pages** (`/tools/[tool-name]`)
   - One page per tool (26 total)
   - Exact content from `/src/assets/usage/tools/*.md`
   - Parameter documentation
   - Usage examples
   - Code syntax highlighting

### Workflow Documentation (Must Have)
8. **Workflow Browser** (`/workflows/`)
   - Searchable/filterable list of all 28 workflows
   - Card layout with emoji, name, description
   - Links to individual workflow pages

9. **Individual Workflow Pages** (`/workflows/[workflow-name]`)
   - One page per workflow (28 total)
   - Content extracted from JSON files
   - Steps, triggers, description
   - Usage examples

### Cognitive Assets (Must Have)
10. **Cognitive Assets Overview** (`/cognitive-assets/`)
    - Explanation of 54 frameworks
    - Links to Roles, Colors, Perspectives sections

11. **Role Pages** (`/cognitive-assets/roles/[role-name]`)
    - 7 individual role pages
    - Content from JSON: personality, principles, approach

12. **Color Pages** (`/cognitive-assets/colors/[color-name]`)
    - 7 individual color pages (De Bono's Six Hats + Gray)
    - Content from JSON: thinking mode, when to use

13. **Perspective Pages** (`/cognitive-assets/perspectives/[perspective-name]`)
    - 12 individual perspective pages
    - Content from JSON: linguistic patterns, cognitive styles

### Demo Showcase (Must Have)
14. **Demo Overview** (`/demo/`)
    - Timeline: 68-minute orchestration
    - 25 agents across 2 phases
    - Key results: 85% success, critical bug found

15. **Demo Artifact Pages** (`/demo/[artifact-name]`)
    - 15 markdown files from demo-artifacts
    - Part 1: PM-lite protocol
    - Part 2: Agentic-SLC workflow
    - Full orchestration sessions, test reports

### Navigation & Layout (Must Have)
16. **Header Navigation**
    - Logo (clickable → home)
    - Main nav: Start, Docs, Tools, Workflows, Cognitive Assets, Demo
    - Dark mode toggle
    - GitHub link

17. **Footer**
    - Learn more links
    - License: MIT
    - Social links (GitHub)
    - Version badge

### Technical Implementation (Must Have)
19. **Next.js Configuration**
    - Static export (`output: "export"`)

      **Official configuration from Next.js docs:** "To enable a static export, change the output mode inside `next.config.js`:"

      ```javascript
      const nextConfig = {
        output: 'export',
      }
      module.exports = nextConfig
      ```

      **Source:** [Next.js Static Export Official Documentation](https://nextjs.org/docs/app/building-your-application/deploying/static-exports) - Accessed 2025-10-28

    - Image optimization disabled

      **Official configuration from Next.js docs:** "When `unoptimized` is set to `true`, the source image will be served as-is from the `src` instead of changing quality, size, or format."

      ```javascript
      module.exports = {
        images: {
          unoptimized: true,
        },
      }
      ```

      **Source:** [Next.js Image Component Documentation](https://nextjs.org/docs/app/api-reference/components/image) - Accessed 2025-10-28

    - basePath configuration for GitHub Pages

      **Official configuration from Next.js docs:** "To deploy a Next.js application under a sub-path of a domain you can use the `basePath` config option."

      **Source:** [Next.js basePath Configuration](https://nextjs.org/docs/app/api-reference/config/next-config-js/basePath) - Accessed 2025-10-28

    - Dark mode (Tailwind dark: class)
    - TypeScript strict mode
    - Responsive design (mobile-first)

20. **Styling**
    - Tailwind CSS 4.0 with official installation

      **Official installation from Tailwind docs:**

      ```bash
      npm install tailwindcss @tailwindcss/postcss postcss
      ```

      **PostCSS Configuration (`postcss.config.mjs`):**

      ```javascript
      export default {
        plugins: {
          '@tailwindcss/postcss': {},
        },
      }
      ```

      **Global CSS Import (`app/globals.css`):**

      ```css
      @import 'tailwindcss';
      ```

      **Source:** [Tailwind CSS + Next.js Guide](https://tailwindcss.com/docs/guides/nextjs) - Accessed 2025-10-28

    - Dark theme matching branding (dark backgrounds, blue accents)

      **Official dark mode support from Tailwind docs:** "For user-controlled dark mode switching, update `app/globals.css`:"

      ```css
      @import "tailwindcss";

      @custom-variant dark (&:where(.dark, .dark *));
      ```

      **Apply dark class to HTML element and use in components:**

      ```jsx
      <div className="bg-white dark:bg-slate-900">
        <h1 className="text-slate-900 dark:text-white">
          Content
        </h1>
      </div>
      ```

      **Source:** [Tailwind CSS Dark Mode Documentation](https://tailwindcss.com/docs/dark-mode) - Accessed 2025-10-28

    - Syntax highlighting (Shiki)
    - Monospace fonts for code
    - Clean typography (Inter or similar)

21. **Assets**
    - Copy logo.svg to public/
    - Copy graph.jpeg to public/
    - Proper image optimization

22. **GitHub Pages Deployment**
    - GitHub Actions workflow with official actions

      **Official GitHub Actions for Pages deployment:**

      ```yaml
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

      **Source:** [GitHub Actions Pages Deployment - Official](https://github.com/actions/deploy-pages) - Accessed 2025-10-28

    - Builds on push to main
    - Deploys via GitHub Actions (artifact-based)

      **Official requirement:** "If you want to use a build process other than Jekyll or you do not want a dedicated branch to hold your compiled static files, use a custom workflow instead."

      **Source:** [GitHub Pages - Configuring Publishing Source](https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site) - Accessed 2025-10-28

    - .nojekyll file to disable Jekyll processing

      **From official Next.js GitHub Pages template:** "You may need to place a `.nojekyll` file in the `/public` directory to disable GitHub Pages from trying to create a Jekyll website."

      **Source:** [Next.js GitHub Pages Deployment Template](https://github.com/nextjs/deploy-github-pages) - Accessed 2025-10-28

    - Custom domain support (if configured)

23. **404 Error Handling**
    - Automatic 404.html generation

      **Official statement from Next.js docs:** "When using static exports in Next.js, a 404 error page is automatically generated. After running `next build` with `output: 'export'` configured, Next.js generates an `/out/404.html` file alongside your other static assets."

      **Source:** [Next.js Static Export Documentation](https://nextjs.org/docs/app/building-your-application/deploying/static-exports) - Accessed 2025-10-28

    - Custom 404 page creation

      **Official implementation from Next.js docs:** "To create a custom 404 'Page Not Found' error page, create a file named `pages/404.js`. This file is statically generated at build time."

      ```typescript
      // pages/404.js
      export default function Custom404() {
        return (
          <div>
            <h1>404 - Page Not Found</h1>
            <p>The page you're looking for doesn't exist.</p>
          </div>
        )
      }
      ```

      **Source:** [Next.js Custom Error Pages](https://nextjs.org/docs/pages/building-your-application/routing/custom-error) - Accessed 2025-10-28

    - Can use getStaticProps for build-time data fetching in 404 page
    - GitHub Pages automatically serves 404.html for non-existent routes

24. **Build Failure Recovery**
    - GitHub Actions failure handling with official actions

      **Official failure detection from GitHub docs:** "A job or step is considered successful or failed based on its conclusion. The `conclusion` property can be `success`, `failure`, `cancelled`, `skipped`, or `timed_out`."

      **Source:** [GitHub Actions Workflow Syntax](https://docs.github.com/en/actions/writing-workflows/workflow-syntax-for-github-actions) - Accessed 2025-10-28

    - Step-level failure handling

      ```yaml
      - name: Build with Next.js
        id: build
        run: npm run build
        continue-on-error: false
        timeout-minutes: 10
      ```

    - Conditional job execution based on failure

      ```yaml
      - name: Notify on failure
        if: failure()
        run: echo "Build failed - check logs"
      ```

    - Email notifications

      **Official notification support:** "You can configure notifications for workflow run failures. By default, you'll receive email notifications for workflow failures on workflows you triggered or own."

      **Source:** [GitHub Actions Notifications](https://docs.github.com/en/actions/monitoring-and-troubleshooting-workflows/notifications-for-workflow-runs) - Accessed 2025-10-28

    - Manual re-run capability

      **Official re-run documentation:** "You can re-run a workflow run, all failed jobs in a workflow run, or specific jobs in a workflow run up to 30 days after its initial run."

      Command: `gh run rerun <run-id> --failed`

      **Source:** [GitHub CLI Manual](https://docs.github.com/en/actions/managing-workflow-runs/re-running-workflows-and-jobs) - Accessed 2025-10-28

25. **Link Validation**
    - Internal link validation with Next.js typedRoutes

      **Official feature from Next.js docs:** "Next.js can automatically generate types for links in your application, adding type safety to prevent typos and other errors when navigating between pages."

      Configuration in `next.config.ts`:
      ```typescript
      import type { NextConfig } from 'next'

      const nextConfig: NextConfig = {
        typedRoutes: true,
      }

      export default nextConfig
      ```

      **Source:** [Next.js TypeScript Configuration](https://nextjs.org/docs/app/building-your-application/configuring/typescript) - Accessed 2025-10-28

    - External link validation with W3C Link Checker

      **Official W3C tool:** The W3C Link Checker is the official validation tool from the World Wide Web Consortium.

      - **Web Interface:** https://validator.w3.org/checklink
      - **Command Line:** Install via CPAN: `perl -MCPAN -e 'install W3C::LinkChecker'`
      - Validates all links (internal and external), anchors, and redirects
      - Supports HTML, XHTML, and CSS documents

      **Source:** [W3C Link Checker](https://validator.w3.org/checklink) - Accessed 2025-10-28

    - No official GitHub Action exists for link validation (manual process required)

26. **Content Maintenance Workflow**
    - Source of truth: All content in `/Users/brett/src/ma-collective/maenifold/` repository
    - Update process:
      1. Modify markdown files in source repository
      2. Commit changes to main branch
      3. GitHub Actions automatically rebuilds and deploys site
    - No manual site updates required (build is fully automated)
    - Content sync: Site rebuilds on every push to main branch

27. **Content Versioning**
    - Add "Last Updated" dates to each page
    - Use Git commit timestamp via build-time data fetching
    - Implementation: Query file modification time during build
    - Display format: "Last updated: January 28, 2025"
    - Location: Footer of each content page
    - No manual date tracking required (automated from Git history)

28. **Image Optimization Strategy**
    - Static images only (no runtime optimization)

      **Required configuration:** As documented in Technical Implementation section, `images: { unoptimized: true }` is required for static export.

    - Image formats:
      - SVG for logos and icons (vector graphics, small file size)
      - WebP for photos and screenshots (modern format, smaller than JPEG)
      - JPEG fallback for older browser compatibility (if needed)
    - Manual optimization before commit:
      - Compress images using tools like ImageOptim or Squoosh
      - Target: < 200KB per image
      - Dimensions: Serve at display size (no oversized images)
    - All images in `/public/assets/` directory
    - No lazy loading required (static site, fast loads)

29. **Mobile Testing Plan**
    - Responsive breakpoints:
      - Mobile: 320px - 768px
      - Tablet: 768px - 1024px
      - Desktop: 1024px+
    - Testing devices (via browser DevTools):
      - iPhone SE (375x667)
      - iPhone 14 Pro (393x852)
      - iPad Air (820x1180)
      - Galaxy S21 (360x800)
    - Test scenarios:
      - Navigation menu (hamburger on mobile)
      - Code blocks (horizontal scroll)
      - Tables (responsive layout)
      - Images (proper scaling)
      - Touch targets (min 44x44px)
    - Testing tools:
      - Chrome DevTools device emulation
      - Safari Responsive Design Mode
      - Real device testing on iPhone and Android
    - Lighthouse mobile audit score target: > 90

30. **Accessibility Audit Process**
    - WCAG 2.1 AA compliance minimum
    - Automated testing:
      - Lighthouse accessibility audit (target: 100)
      - axe DevTools browser extension
      - WAVE Web Accessibility Evaluation Tool
    - Manual testing checklist:
      - ✅ Semantic HTML (proper heading hierarchy h1→h2→h3)
      - ✅ Alt text for all images
      - ✅ Keyboard navigation (tab through all interactive elements)
      - ✅ Focus indicators visible
      - ✅ Color contrast ratios (4.5:1 for normal text, 3:1 for large text)
      - ✅ Screen reader testing (VoiceOver on macOS, NVDA on Windows)
      - ✅ No keyboard traps
      - ✅ Form labels and ARIA attributes where needed
    - Tools:
      - **axe DevTools:** Browser extension for automated a11y scanning
      - **WAVE:** https://wave.webaim.org/
      - **WebAIM Contrast Checker:** https://webaim.org/resources/contrastchecker/
    - Fix all issues before deployment

31. **Performance Monitoring**
    - Pre-deployment monitoring:
      - Lighthouse CI in GitHub Actions
      - Performance budget: FCP < 1.5s, TTI < 3s, LCP < 2.5s
      - Fail build if Lighthouse score < 90
    - Post-deployment verification:
      - Manual Lighthouse audit on production URL
      - PageSpeed Insights: https://pagespeed.web.dev/
      - WebPageTest: https://www.webpagetest.org/
    - Metrics to track:
      - First Contentful Paint (FCP)
      - Largest Contentful Paint (LCP)
      - Time to Interactive (TTI)
      - Cumulative Layout Shift (CLS)
      - Total page size
      - Number of requests
    - No runtime monitoring (static site, no analytics per Ma Protocol)
    - Performance regression checks on every PR

32. **Content Update Process**
    - Update frequency: As needed (no fixed schedule)
    - Update workflow:
      1. Identify content to update in source repository
      2. Edit markdown/JSON files locally
      3. Test locally with `npm run dev`
      4. Commit with descriptive message
      5. Push to main branch
      6. GitHub Actions rebuilds site automatically
      7. Verify deployment successful (check GitHub Actions logs)
      8. Verify content live on production URL
    - Breaking changes: Test in local environment before pushing
    - Rollback: Revert Git commit and push (triggers automatic rebuild)
    - No manual FTP, no manual file copies, no manual deployments

---

## What We WON'T Build

### Features Explicitly Out of Scope
❌ **Interactive graph visualization** (3D force-directed graph)
   - Reason: Not in source content, adds complexity, requires React Three Fiber
   - Future enhancement if requested

❌ **API documentation**
   - Reason: Maenifold is an MCP server/CLI tool, not an API
   - Existing tool docs cover the interface

❌ **Video tutorials or demos**
   - Reason: No video content exists in source files
   - Would require production work beyond site scope

❌ **User authentication or accounts**
   - Reason: Static documentation site, no dynamic features needed

❌ **Analytics or tracking**
   - Reason: Ma Protocol principle - no telemetry, not even anonymous

❌ **Newsletter signup or email capture**
   - Reason: Not requested, adds complexity

❌ **Blog or news section**
   - Reason: No blog content exists in source

❌ **Community forum or comments**
   - Reason: Static site, no backend

❌ **Versioned documentation**
   - Reason: Single version for v1.0, can add later if needed

❌ **Playground or interactive code examples**
   - Reason: Maenifold is a local MCP tool, web playground doesn't apply

❌ **Custom animated backgrounds or effects**
   - Reason: Keep it simple, fast loading

❌ **Social media embeds or widgets**
   - Reason: Unnecessary complexity

❌ **Cookie consent banners**
   - Reason: No cookies, no tracking (Ma Protocol)

❌ **Multi-language support (i18n)**
   - Reason: English only for v1.0

❌ **Site-wide search functionality**
   - Reason: Pagefind + Next.js 16 integration is NOT officially documented
   - Next.js provides NO official recommendations for static site search
   - Ma Protocol: "Either it works and is documented or we don't use it at all"
   - Users can use browser's native find (Cmd/Ctrl+F) or GitHub search

### Content We WON'T Fabricate
❌ **User testimonials** - None exist
❌ **Case studies** - None documented
❌ **Comparison charts** - Not in source
❌ **Pricing pages** - Open source, free
❌ **FAQ section** - Can add later from actual questions
❌ **Tutorials beyond Quick Start** - Stick to source content
❌ **Release notes** - Not in scope for v1.0

---

## User Stories & Acceptance Criteria

### As a Developer Evaluating Maenifold
**I want** to understand what maenifold does and why I should use it
**So that** I can decide if it solves my problem

**Acceptance Criteria:**
- Landing page clearly explains the value proposition
- Demo story shows real-world results (25 agents, 85% success, real bug found)
- Quick Start gets me to "hello world" in < 5 minutes
- Can see the cognitive stack architecture

### As a Developer Implementing Maenifold
**I want** to find documentation for specific tools and workflows
**So that** I can integrate them into my agent system

**Acceptance Criteria:**
- Can browse all 26 tools in one place
- Each tool page has complete parameter documentation
- Code examples are syntax-highlighted
- Navigation is intuitive (header always visible)

### As a Researcher Understanding AI Orchestration
**I want** to read the demo artifacts and orchestration sessions
**So that** I can learn from real multi-agent implementations

**Acceptance Criteria:**
- Demo section shows timeline and results
- All 15 artifact documents are accessible
- Can read full orchestration sessions
- Understand how PM-lite protocol works

### As a User on Mobile
**I want** the site to work on my phone
**So that** I can reference docs while working

**Acceptance Criteria:**
- All pages responsive (mobile-first design)
- Navigation works on small screens (hamburger menu)
- Code blocks scroll horizontally if needed
- Dark mode works on mobile

### As a Site Visitor
**I want** dark mode that matches the maenifold brand
**So that** the experience is consistent and easy on my eyes

**Acceptance Criteria:**
- Dark mode toggle in header
- Preference persists across pages (localStorage)
- All pages support dark mode
- Colors match branding (dark backgrounds, blue accents)
- Code blocks have dark theme syntax highlighting

---

## Design Requirements

### Visual Design
- **Color Palette**:
  - Light mode: White backgrounds, dark text, blue accents
  - Dark mode: Slate-950 backgrounds, white text, blue-400 accents
  - Code blocks: Slate-100 (light) / Slate-900 (dark)

- **Typography**:
  - Headings: Sans-serif (Inter, system-ui)
  - Body: Sans-serif (Inter, system-ui)
  - Code: Monospace (SF Mono, Menlo, monospace)

- **Spacing**:
  - Consistent 8px grid
  - Generous padding/margins (not cramped)
  - Max-width: 1200px for readability

- **Components**:
  - Card-based layouts for browsers (tools, workflows)
  - Breadcrumbs for deep pages
  - Table of contents for long pages
  - Copy button for code blocks

### Branding Assets
- Logo: `/assets/branding/maenifold-logo.svg`
  - Animated neural network
  - Use in header (32px height)

- Graph Visualization: `/assets/branding/graph.jpeg`
  - Hero section on landing page
  - Demonstrates knowledge graph concept

### Accessibility
- WCAG AA minimum compliance
- Semantic HTML (proper heading hierarchy)
- Alt text for images
- Keyboard navigation
- Focus indicators
- Color contrast requirements met

---

## Technical Architecture

### Technology Stack
- **Framework**: Next.js 16 with App Router

  **Official statement:** "Server Components consumed inside the `app` directory will run during the build, similar to traditional static-site generation."

  **Source:** [Next.js Static Exports Documentation](https://nextjs.org/docs/app/building-your-application/deploying/static-exports) - Accessed 2025-10-28

- **Language**: TypeScript (strict mode)
- **Styling**: Tailwind CSS 4.0

  **Official framework recommendation:** "Tailwind CSS is a utility-first CSS framework that provides low-level utility classes to build custom designs."

  **Source:** [Next.js Styling Documentation](https://nextjs.org/docs/app/building-your-application/styling/tailwind-css) - Accessed 2025-10-28

- **Syntax Highlighting**: Shiki (server-side)
- **Build**: Static export for GitHub Pages

  **Official GitHub Pages support:** "GitHub Pages is a static site hosting service that takes HTML, CSS, and JavaScript files straight from a repository on GitHub, optionally runs the files through a build process, and publishes a website."

  **Source:** [GitHub Pages Official Documentation](https://docs.github.com/en/pages/getting-started-with-github-pages/what-is-github-pages) - Accessed 2025-10-28

- **Deployment**: GitHub Actions

### File Structure
```
site/
├── app/
│   ├── layout.tsx (root layout with nav/footer)
│   ├── page.tsx (landing page)
│   ├── start/page.tsx
│   ├── docs/
│   │   ├── architecture/page.tsx
│   │   ├── philosophy/page.tsx
│   │   └── technical-specs/page.tsx
│   ├── tools/
│   │   ├── page.tsx (browser)
│   │   └── [slug]/page.tsx (dynamic routes)
│   ├── workflows/
│   │   ├── page.tsx (browser)
│   │   └── [slug]/page.tsx (dynamic routes)
│   ├── cognitive-assets/
│   │   ├── page.tsx
│   │   ├── roles/[slug]/page.tsx
│   │   ├── colors/[slug]/page.tsx
│   │   └── perspectives/[slug]/page.tsx
│   └── demo/
│       ├── page.tsx
│       └── [slug]/page.tsx
├── components/
│   ├── Header.tsx
│   ├── Footer.tsx
│   ├── Navigation.tsx
│   ├── DarkModeToggle.tsx
│   ├── ToolCard.tsx
│   ├── WorkflowCard.tsx
│   └── CodeBlock.tsx
├── lib/
│   ├── content.ts (content loading utilities)
│   └── utils.ts
├── public/
│   ├── assets/
│   │   └── branding/ (logo, graph)
│   └── demo-artifacts/
├── content/ (or process from src/assets/)
│   ├── tools/
│   ├── workflows/
│   └── cognitive-assets/
└── next.config.ts
```

### Build Process
1. Extract content from source files (tools, workflows, roles, etc.)
2. Transform JSON to MDX/TypeScript for Next.js consumption
3. Generate static pages (SSG)
4. Export to `out/` directory
5. Deploy `out/` to GitHub Pages

### Performance Requirements
- First Contentful Paint < 1.5s
- Time to Interactive < 3s
- Lighthouse score > 90
- No JavaScript required for basic navigation/reading

---

## Implementation Phases

### Phase 1: Foundation (Agent Wave 1)
**Deliverable**: Working site structure with navigation
- Initialize Next.js project with TypeScript + Tailwind
- Create layout, header, footer components
- Implement dark mode toggle
- Build navigation structure
- Verify GitHub Pages static export works

**Acceptance Criteria**:
- Site builds without errors
- Navigation links all exist (even if pages empty)
- Dark mode works
- Header/footer on all pages

### Phase 2: Core Content (Agent Wave 2)
**Deliverable**: Landing, Quick Start, Docs pages
- Landing page with hero + demo story
- Quick Start page
- Architecture page
- Technical Specs page
- Philosophy page

**Acceptance Criteria**:
- All content copied exactly from source (README, docs/README)
- Images display correctly
- Code blocks syntax-highlighted
- Links between core pages work

### Phase 3: Tool Documentation (Agent Wave 3)
**Deliverable**: All 26 tool pages + browser
- Tool browser page with search/filter
- Generate all 26 tool pages from source markdown
- Consistent styling and navigation

**Acceptance Criteria**:
- All 26 tools accessible
- Browser shows all tools
- Search/filter works
- Code examples properly formatted

### Phase 4: Workflow Documentation (Agent Wave 4)
**Deliverable**: All 28 workflow pages + browser
- Workflow browser page
- Generate all 28 workflow pages from JSON
- Extract steps, triggers, descriptions

**Acceptance Criteria**:
- All 28 workflows accessible
- Browser shows all workflows with emojis
- Content extracted correctly from JSON
- Navigation works

### Phase 5: Cognitive Assets (Agent Wave 5)
**Deliverable**: Roles, Colors, Perspectives pages
- Cognitive assets overview page
- 7 role pages
- 7 color pages
- 12 perspective pages

**Acceptance Criteria**:
- All 26 asset pages created
- Content extracted from JSON
- Organized into logical sections

### Phase 6: Demo Showcase (Agent Wave 6)
**Deliverable**: Demo overview + artifact pages
- Demo overview with timeline
- 15 demo artifact pages
- Part 1 and Part 2 organization

**Acceptance Criteria**:
- All demo content accessible
- Timeline visualization
- Artifact pages readable

### Phase 7: Polish & Testing (Agent Wave 7)
**Deliverable**: Complete, tested site
- Add breadcrumbs
- Add copy buttons to code blocks
- Mobile testing
- Accessibility audit
- Performance optimization
- Link validation

**Acceptance Criteria**:
- Mobile responsive (tested on multiple sizes)
- Lighthouse score > 90
- No broken links (validated with Next.js typedRoutes and W3C Link Checker)
- All acceptance criteria from user stories met

### Phase 8: Deployment (Agent Wave 8)
**Deliverable**: Live site on GitHub Pages
- GitHub Actions workflow
- Test deployment
- Custom domain (if configured)
- Verify all pages accessible

**Acceptance Criteria**:
- Site live at GitHub Pages URL
- All pages load correctly
- No 404s
- Dark mode persists

---

## Risks & Mitigations

### Risk: Content extraction complexity
**Impact**: High
**Likelihood**: Medium
**Mitigation**: Build extraction scripts early, test with one tool/workflow first

### Risk: Next.js static export limitations
**Impact**: High
**Likelihood**: Low
**Mitigation**: Test static export in Phase 1, verify all routes work

### Risk: Dark mode state persistence
**Impact**: Medium
**Likelihood**: Low
**Mitigation**: Use localStorage, test early

### Risk: Mobile responsiveness issues
**Impact**: Medium
**Likelihood**: Medium
**Mitigation**: Mobile-first design, test continuously on small screens

### Risk: Scope creep (adding features not in PRD)
**Impact**: High
**Likelihood**: High
**Mitigation**: THIS PRD. Strict adherence to WILL/WON'T lists. PM approval required for any additions.

---

## Definition of Done

A complete site means:

✅ **All pages exist and are accessible**
   - 1 landing, 1 quick start, 3 docs, 26 tools, 28 workflows, 26 cognitive assets, 15 demo pages
   - Total: 100+ pages minimum

✅ **Navigation works everywhere**
   - Header on every page
   - Footer on every page
   - Breadcrumbs on deep pages
   - No dead links

✅ **Dark mode works**
   - Toggle in header
   - Preference persists
   - All pages support both modes
   - Matches branding

✅ **Mobile responsive**
   - Works on phone screens
   - Navigation adapts
   - Content readable

✅ **Performance passes**
   - Lighthouse > 90
   - Fast page loads

✅ **Content accuracy**
   - Zero fabrication
   - All source content present
   - No hallucinated features

✅ **Deployment successful**
   - Live on GitHub Pages
   - All pages accessible
   - No 404s

✅ **User acceptance**
   - PM (user) approves
   - Meets SLC criteria
   - Won't need rework

---

## Open Questions for Product Manager (User)

1. **Custom domain**: Do you want to configure a custom domain (e.g., maenifold.dev) or use github.io?

2. **GitHub Actions**: Should deployment be automatic on push to main, or manual?

3. **Workflow JSON display**: Should we show raw JSON for workflows or just human-readable content?

4. **Demo artifact organization**: Should Part 1 and Part 2 be separate sections or tabs?

5. **Code copy buttons**: Do you want copy-to-clipboard buttons on all code blocks?

6. **Breadcrumbs**: Show file path style breadcrumbs on all pages, or just deep pages?

---

## References

All technical claims in this PRD are validated against official documentation from the following sources:

### 1. GitHub Pages Official Requirements
- **Document**: github-pages-official-requirements.md
- **Primary Source**: https://docs.github.com/en/pages/getting-started-with-github-pages/what-is-github-pages
- **Date Accessed**: 2025-10-28
- **Key Topics**: Repository limits (1GB), build timeout (10 minutes), bandwidth limits (100GB/month), artifact size limits (10GB max)

**Relevant Quotes Cited in PRD**:
- "GitHub Pages is a static site hosting service that takes HTML, CSS, and JavaScript files straight from a repository on GitHub"
- Repository size: 1 GB recommended limit; published sites: maximum 1 GB
- Build limit: 10 minute deployment timeout
- Soft bandwidth limit of 100 GB per month
- Artifact requirement: compressed gzip archive under 10GB

### 2. Next.js Static Export Official Documentation
- **Document**: nextjs-static-export-official-docs.md
- **Primary Source**: https://nextjs.org/docs/app/building-your-application/deploying/static-exports
- **Date Accessed**: 2025-10-28
- **Key Topics**: Static export configuration, image optimization, basePath requirement, unsupported features

**Relevant Quotes Cited in PRD**:
- `output: 'export'` configuration syntax
- `images: { unoptimized: true }` for static exports
- `basePath` configuration for subdirectory deployment
- `.nojekyll` file for disabling Jekyll processing

### 3. GitHub Actions Pages Deployment Official Documentation
- **Document**: github-actions-pages-deployment.md
- **Primary Sources**:
  - https://github.com/actions/deploy-pages
  - https://github.com/actions/upload-pages-artifact
  - https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages
- **Date Accessed**: 2025-10-28
- **Key Topics**: Official GitHub Actions, workflow structure, permissions, artifact management

**Relevant Quotes Cited in PRD**:
- `actions/upload-pages-artifact@v3` for packaging artifacts
- `actions/deploy-pages@v4` for deployment
- Required permissions: `pages: write` and `id-token: write`
- Complete workflow YAML structure

### 4. Tailwind CSS + Next.js Official Integration
- **Document**: tailwind-nextjs-official-integration.md
- **Primary Sources**:
  - https://nextjs.org/docs/app/building-your-application/styling/tailwind-css
  - https://tailwindcss.com/docs/guides/nextjs
  - https://tailwindcss.com/docs/dark-mode
- **Date Accessed**: 2025-10-28
- **Key Topics**: Installation, configuration, dark mode implementation, static export compatibility

**Relevant Quotes Cited in PRD**:
- Tailwind CSS installation command: `npm install tailwindcss @tailwindcss/postcss postcss`
- PostCSS configuration: `postcss.config.mjs` structure
- Global CSS import: `@import 'tailwindcss';`
- Dark mode with CSS class: `@custom-variant dark (&:where(.dark, .dark *))`
- Compatible with static exports (CSS is compiled at build time)

### 5. Next.js 404 Error Page Handling in Static Export Mode
- **Document**: nextjs-404-handling-static-export.md
- **Primary Sources**:
  - https://nextjs.org/docs/app/building-your-application/deploying/static-exports
  - https://nextjs.org/docs/pages/building-your-application/routing/custom-error
  - https://github.com/vercel/next.js/blob/canary/docs/02-pages/03-building-your-application/01-routing/08-custom-error.mdx
- **Date Accessed**: 2025-10-28
- **Key Topics**: Automatic 404.html generation, custom 404 page creation, static generation, Pages Router implementation

**Relevant Quotes Cited in PRD**:
- "When using static exports in Next.js, a 404 error page is automatically generated"
- "To create a custom 404 'Page Not Found' error page, create a file named `pages/404.js`. This file is statically generated at build time"
- Can use `getStaticProps` for build-time data fetching in 404 pages

### 6. GitHub Actions Build Failure Handling
- **Document**: github-actions-failure-handling.md
- **Primary Sources**:
  - https://docs.github.com/en/actions/writing-workflows/workflow-syntax-for-github-actions
  - https://docs.github.com/en/actions/monitoring-and-troubleshooting-workflows/notifications-for-workflow-runs
  - https://docs.github.com/en/actions/managing-workflow-runs/re-running-workflows-and-jobs
- **Date Accessed**: 2025-10-28
- **Key Topics**: Failure detection, step-level error handling, conditional execution, email notifications, manual re-runs

**Relevant Quotes Cited in PRD**:
- "A job or step is considered successful or failed based on its conclusion"
- "You can configure notifications for workflow run failures. By default, you'll receive email notifications"
- "You can re-run a workflow run, all failed jobs in a workflow run, or specific jobs in a workflow run up to 30 days after its initial run"
- Command: `gh run rerun <run-id> --failed`

### 7. Link Validation Methods for Static Sites
- **Document**: link-validation-research-findings.md
- **Primary Sources**:
  - https://nextjs.org/docs/app/building-your-application/configuring/typescript (typedRoutes)
  - https://validator.w3.org/checklink (W3C Link Checker)
  - https://github.com/actions (verified no official link validation action exists)
- **Date Accessed**: 2025-10-28
- **Key Topics**: Next.js typedRoutes for internal links, W3C Link Checker for external validation, absence of official GitHub Actions

**Relevant Quotes Cited in PRD**:
- "Next.js can automatically generate types for links in your application, adding type safety to prevent typos and other errors when navigating between pages"
- Configuration: `typedRoutes: true` in `next.config.ts`
- W3C Link Checker is official W3C tool available via web interface and command line
- NO official GitHub Action exists for link validation (manual process required)

### 8. Pagefind + Next.js Integration Research (Result: NOT DOCUMENTED)
- **Document**: pagefind-nextjs-integration.md
- **Primary Source**: https://pagefind.app/docs/
- **Date Accessed**: 2025-10-28
- **Key Finding**: Pagefind integration with Next.js 16 is NOT officially documented
- **Result**: Search functionality REMOVED from PRD per Ma Protocol ("Either it works and is documented or we don't use it at all")

### 9. Next.js Official Search Guidance (Result: DOES NOT EXIST)
- **Document**: nextjs-official-search-guidance.md
- **Primary Source**: https://nextjs.org/docs/app/guides/static-exports
- **Date Accessed**: 2025-10-28
- **Key Finding**: Next.js provides NO official recommendations for static site search
- **Result**: Search functionality REMOVED from PRD

---

## Citation Methodology

This PRD follows the [[Ma-Protocol]] principle: **Every technical claim requires proof.**

All claims are backed by:
1. **Exact verbatim quotes** from official documentation
2. **Source URLs** for independent verification
3. **Dates accessed** for temporal tracking
4. **No fabrication** - only documented official information

Technical claims without citations represent areas requiring further research or vendor confirmation.

---

## Approval Required

This PRD requires approval from the Product Manager (user) before implementation begins.

**Sign-off required on:**
- [ ] Site structure (Information Architecture)
- [ ] WILL build list (scope)
- [ ] WON'T build list (out of scope)
- [ ] Definition of Done
- [ ] Technical architecture (Next.js + Tailwind + Pagefind)

**Once approved, this PRD is the contract.** No scope changes without PM approval.

---

**Version**: 1.0
**Last Updated**: 2025-10-28
**Status**: Awaiting Approval
