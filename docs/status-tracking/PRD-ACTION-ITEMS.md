# PRD Technical Feasibility: Action Items

**Date**: 2025-10-28
**Document**: Critical issues and immediate action items from feasibility review
**Priority**: BEFORE PHASE 1 APPROVAL

---

## 🚨 BLOCKING ISSUES (MUST RESOLVE)

### Issue 1: Content Files Not Found
**Severity**: 🔴 CRITICAL - Blocks Phase 2
**Discovered**: Asset paths in PRD don't match repository structure

**What PRD Claims**:
- `/Users/brett/src/ma-collective/maenifold/src/assets/usage/tools/*.md` (26 files)
- `/Users/brett/src/ma-collective/maenifold/src/assets/workflows/*.json` (28 files)
- `/Users/brett/src/ma-collective/maenifold/src/assets/roles/*.json` (7 files)
- etc.

**What We Found**:
- Actual path: `/Users/brett/src/ma-collective/maenifold/bin/osx-arm64/assets/workflows/*.json`
- These are compiled binary assets, not source files
- `/src/assets/` directory does NOT exist

**Action Required**:
```
[ ] 1. Locate source files for tools (26 markdown files)
[ ] 2. Locate source files for workflows (28 JSON files)
[ ] 3. Locate source files for roles (7 JSON files)
[ ] 4. Locate source files for colors (7 JSON files)
[ ] 5. Locate source files for perspectives (12 JSON files)
[ ] 6. Locate demo artifacts (15 markdown files)
[ ] 7. Document actual file locations
[ ] 8. Design content extraction/loading strategy
[ ] 9. Create extraction script if files are in binaries
[ ] 10. Test content loading with sample data
```

**Timeline**: 1-3 hours (research + scripting)

**Assigned To**: Product Manager (requires file location knowledge)

---

### Issue 2: Repository Size Exceeds Limits
**Severity**: 🔴 CRITICAL - Blocks Phase 1 approval
**Discovered**: Total repo 3.7GB, GitHub Pages limit 1GB for published site

**What PRD Says**:
- "Repository size: 1 GB recommended limit"
- "Published sites: maximum 1 GB" ✓ (Correct)

**What We Found**:
- `/Users/brett/src/ma-collective/maenifold` = **3.7GB total**
- Compiled binaries in `/bin/` = significant portion
- Unknown: Built site output size (could be huge)

**Action Required**:
```
[ ] 1. Create .gitignore entry to exclude /bin/ folder
[ ] 2. Create .gitignore entry to exclude /src/ (C# source)
[ ] 3. Create .gitignore entry to exclude node_modules/
[ ] 4. Create .gitignore entry to exclude large demo files
[ ] 5. Run "next build" with test site to measure output size
[ ] 6. Measure actual published site size (should be < 1GB)
[ ] 7. If > 1GB: Plan exclusions (demo artifacts, etc.)
[ ] 8. Test build size in GitHub Actions runner
[ ] 9. Document size management strategy
[ ] 10. Set up GitHub Actions cleanup step (remove node_modules after build)
```

**Timeline**: 1-2 hours (setup + testing)

**Assigned To**: Implementation lead (Phase 1)

---

## ⚠️ MEDIUM-RISK ISSUES (TEST BEFORE PHASE 7)

### Issue 3: Pagefind Compatibility Unverified
**Severity**: 🟡 MEDIUM - Blocks Phase 7 if failed
**Discovered**: Pagefind + Next.js 16 combination not verified

**What We Know**:
- Pagefind is good for static sites ✓
- Next.js 16 is new, released Oct 2024
- No official Next.js 16 + Pagefind documentation found

**What Could Go Wrong**:
- Build fails in Phase 7
- Search index doesn't work
- Build time exceeds 10-minute timeout
- basePath configuration breaks search

**Action Required**:
```
[ ] 1. Create test Next.js 16 project (locally)
[ ] 2. Add Pagefind to test project
[ ] 3. Create 20+ test pages with varied content
[ ] 4. Run "next build" and measure time
[ ] 5. Verify search index generation completes
[ ] 6. Test search functionality in built site
[ ] 7. Test with basePath configuration set
[ ] 8. Verify build time < 10 minutes
[ ] 9. Document any issues/workarounds found
[ ] 10. If fails: Document backup solution (JSON search)
```

**Timeline**: 2-3 hours (test + documentation)

**Assigned To**: Phase 7 implementation lead

**Fallback Plan** (if Pagefind fails):
- Implement pre-built JSON search index
- Simple in-page search with browser Ctrl+F
- Consider external search service

---

### Issue 4: Dynamic Routes Build Time Unknown
**Severity**: 🟡 MEDIUM - Affects Phase 3-6 implementation
**Discovered**: Building 54 dynamic pages (26 tools + 28 workflows) could exceed 10-min limit

**What We Know**:
- Each dynamic route requires server-side rendering at build time
- Must call `generateStaticParams()` for all 54 pages
- Content loading (fs reads) happens at build time
- Pagefind index generation adds time

**Action Required**:
```
[ ] 1. Create test site with 54 dynamic pages
[ ] 2. Populate with realistic content (tool docs, workflow steps)
[ ] 3. Measure "next build" time locally
[ ] 4. Test build on slower machine (simulate GitHub Actions)
[ ] 5. If > 10 minutes: Optimize content loading
[ ] 6. If > 10 minutes: Consider parallel builds
[ ] 7. Document baseline build time
[ ] 8. Create monitoring for production builds
```

**Timeline**: 2-3 hours (benchmarking)

**Assigned To**: Phase 3 implementation lead

---

### Issue 5: basePath Interaction with Images
**Severity**: 🟡 MEDIUM - Affects Phase 2 (images)
**Discovered**: PRD uses `unoptimized: true` but image paths with basePath untested

**What PRD Says**:
- "When using next/image component, you need to add basePath in front of src" ✓ (Correct)

**What's Untested**:
- SVG logo rendering with basePath
- JPEG graph rendering with basePath
- Relative image paths in markdown
- Image loading in dynamic routes

**Action Required**:
```
[ ] 1. Create test pages with images (SVG + JPEG)
[ ] 2. Use both <Image> component and <img> tags
[ ] 3. Test with basePath configured
[ ] 4. Test relative paths (./assets/images/test.jpg)
[ ] 5. Test absolute paths (/assets/branding/logo.svg)
[ ] 6. Verify images load correctly in built site
[ ] 7. Test on GitHub Pages with basePath
[ ] 8. Document image path rules for developers
```

**Timeline**: 1-2 hours (testing)

**Assigned To**: Phase 1 implementation lead

---

## ✅ LOW-RISK ISSUES (FYI, NO ACTION NEEDED YET)

### Issue 6: Dark Mode Hydration Edge Case
**Severity**: 🟢 LOW - Nice-to-have, not blocking
**Status**: Common pattern, well-documented

**Note**: Requires client-side script in `<head>` to prevent theme flicker. Standard implementation.

---

### Issue 7: Code Syntax Highlighting Build Impact
**Severity**: 🟢 LOW - Acceptable performance hit
**Status**: Shiki will add 30-60 seconds to build. Within tolerance.

---

## 📋 PRE-PHASE-1 CHECKLIST

**Before Phase 1 Approval, Complete**:

```
BLOCKING ISSUES:
[ ] Content file locations verified
[ ] .gitignore configured for repo cleanup
[ ] Test build run locally, output measured

SETUP:
[ ] GitHub repository configured
[ ] Next.js project initialized with TypeScript
[ ] Tailwind CSS dependencies installed
[ ] ESLint/Prettier configured
[ ] Git branch strategy decided (feature branches vs. main)
[ ] GitHub Actions runner size understood (affects build time)

TESTING:
[ ] Pagefind test project completed (if not done)
[ ] Build time baseline established
[ ] Size baseline established
[ ] Local development environment working

DOCUMENTATION:
[ ] Content extraction strategy documented
[ ] Build process documented
[ ] Image path rules documented
[ ] basePath configuration documented
```

---

## 📋 PHASE-1-ACCEPTANCE CHECKLIST

**Before Phase 2 Approval**:

```
BUILD:
[ ] Next.js build succeeds locally
[ ] Build completes on GitHub Actions
[ ] Build time < 10 minutes
[ ] Build output size measured (< 1GB)

FUNCTIONALITY:
[ ] Navigation links work (no 404s)
[ ] Dark mode toggle visible
[ ] Dark mode persists across page reloads
[ ] Header/footer on all pages
[ ] Responsive design tested (mobile, tablet, desktop)
[ ] No TypeScript errors
[ ] No console errors/warnings

CONTENT:
[ ] Content loading strategy implemented
[ ] Sample tool page renders
[ ] Sample workflow page renders
[ ] Sample cognitive asset page renders
[ ] Image loading works (logo, graph)

DEPLOYMENT:
[ ] GitHub Actions workflow passes
[ ] Test deployment to GitHub Pages successful
```

---

## 📋 PHASE-7-ACCEPTANCE CHECKLIST (Search)

**Before Phase 8 Approval**:

```
PAGEFIND:
[ ] Pagefind installed and configured
[ ] Search index builds successfully
[ ] Build time still < 10 minutes with Pagefind
[ ] Search UI visible and functional
[ ] Search returns correct results
[ ] All 100+ pages indexed
[ ] Search works on mobile

LIGHTHOUSE:
[ ] Lighthouse score > 90
[ ] First Contentful Paint < 1.5s
[ ] Time to Interactive < 3s
[ ] Cumulative Layout Shift < 0.1

FUNCTIONALITY:
[ ] All user stories pass acceptance criteria
[ ] No broken links (404 audit)
[ ] All images load correctly
[ ] Dark mode works with search
[ ] Breadcrumbs functional
[ ] Copy buttons on code blocks work
```

---

## 🗓️ ESTIMATED TIMELINE

| Blocker | Est. Time | Owner | Deadline |
|---------|-----------|-------|----------|
| Content discovery | 1-3 hrs | PM | Before Phase 1 |
| Repo cleanup + gitignore | 1-2 hrs | Phase 1 lead | Phase 1 |
| Pagefind testing | 2-3 hrs | Phase 7 lead | Before Phase 7 |
| Build benchmarking | 2-3 hrs | Phase 3 lead | Phase 3 |
| Image path testing | 1-2 hrs | Phase 2 lead | Phase 2 |

**Total Pre-Phase-1 Work**: 2-5 hours (before Phase 1 approval)

---

## 🎯 SUCCESS CRITERIA

**Phase 1 Approval**: Build works, navigation functional, size < 1GB
**Phase 2+ Approval**: Content discovered and loading successfully
**Phase 7 Approval**: Search works, Lighthouse > 90
**Phase 8 Approval**: All user stories pass, PM approval obtained

---

## 📞 ESCALATION CONTACTS

If blocking issues discovered:
1. **Content discovery stuck**: Contact PM for file locations
2. **Build exceeds size limit**: Contact architect for exclusion strategy
3. **Pagefind fails**: Alert PM, prepare JSON search alternative
4. **Build time issues**: Contact implementation lead for optimization

---

**Document**: PRD Technical Feasibility - Action Items
**Created**: 2025-10-28
**Status**: Ready for PM and Phase 1 lead assignment
**Next Step**: Assign blockers to owners, begin Phase 1 when approved
