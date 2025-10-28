# Agent Dispatch Plan: Landing Page Redesign
## "The Living Manifold" Implementation Strategy

**Sprint ID**: `sprint-20251028-living-manifold`
**Planning Session**: `session-1761686526050`
**Total Agents**: 15 coding-agents across 5 waves
**Estimated Duration**: 9-14 hours

---

## Wave Execution Summary

| Wave | Phase | Agents | Execution | Duration | Dependencies |
|------|-------|--------|-----------|----------|--------------|
| 1 | Foundation | 3 | Parallel | 45-60 min | None |
| 2 | Component Library | 3 | Parallel | 60-75 min | Wave 1 |
| 3 | Integration | 3 | Sequential | 45-75 min | Wave 2 |
| 4 | Polish | 3 | Parallel | 60-75 min | Wave 3 |
| 5 | Testing | 3 | Parallel | 75-90 min | Wave 4 |

**Total**: 285-375 minutes (4.75-6.25 hours) + ~60 min PM verification = 5.75-7.25 hours

---

## Wave 1: Foundation (Parallel - 3 Agents)

### Agent 1: NetworkBackground Component
**RTM**: REQ-1.1
**Session**: Create new Wave 1 session
**Duration**: 20-25 minutes

**Tasks**:
1. Create `site/app/components/NetworkBackground.tsx`
2. Generate SVG structure with viewBox="0 0 1920 1080"
3. Generate 20-30 circular nodes with randomized positions
4. Create connection lines between nearby nodes (distance threshold ~300px)
5. Apply animation CSS classes (from Agent 3's keyframes)
6. Implement responsive node count (20-30 desktop, 10-15 mobile)
7. Add accessibility attributes (role="presentation", aria-hidden="true")
8. Export as: `export function NetworkBackground({ className }: NetworkBackgroundProps)`

**Deliverable**: `site/app/components/NetworkBackground.tsx` (~100 lines)

**Next.js MCP**: Use for runtime validation after component creation

**Acceptance**:
- Component renders without errors
- SVG structure correct
- TypeScript compilation passes
- Animations apply correctly

---

### Agent 2: Tailwind Config Extension
**RTM**: REQ-1.3
**Session**: Same as Agent 1
**Duration**: 10-15 minutes

**Tasks**:
1. Query Next.js MCP for current Tailwind config structure
2. Read `site/tailwind.config.ts`
3. Add Deep Knowledge colors to `theme.extend.colors`:
   ```typescript
   colors: {
     primary: '#0ea5e9',
     secondary: '#a855f7',
     accent: '#06b6d4',
   }
   ```
4. Verify build succeeds (dev server restart if needed)
5. Test colors available as utilities (e.g., `bg-primary`, `text-secondary`)

**Deliverable**: `site/tailwind.config.ts` (~20 lines added)

**Next.js MCP**: Query current config before modifying

**Acceptance**:
- Build succeeds
- Colors available as Tailwind utilities
- No TypeScript errors

---

### Agent 3: Animation Keyframes
**RTM**: REQ-1.2
**Session**: Same as Agent 1
**Duration**: 15-20 minutes

**Tasks**:
1. Read `site/app/globals.css`
2. Add @keyframes float:
   ```css
   @keyframes float {
     0%, 100% { transform: translate3d(0, 0, 0); }
     50% { transform: translate3d(var(--float-x), var(--float-y), 0); }
   }
   ```
   Duration: 30-60s, easing: ease-in-out
3. Add @keyframes pulse:
   ```css
   @keyframes pulse {
     0%, 100% { opacity: 0.3; }
     50% { opacity: 0.8; }
   }
   ```
   Duration: 3-5s, iteration: infinite
4. Add @keyframes gradient-shift (for future shimmer effect):
   ```css
   @keyframes gradient-shift {
     0%, 100% { background-position: 0% 50%; }
     50% { background-position: 100% 50%; }
   }
   ```
5. Enhance prefers-reduced-motion media query:
   ```css
   @media (prefers-reduced-motion: reduce) {
     *, *::before, *::after {
       animation-duration: 0.01ms !important;
       animation-iteration-count: 1 !important;
       transition-duration: 0.01ms !important;
     }
   }
   ```

**Deliverable**: `site/app/globals.css` (~50 lines added)

**Next.js MCP**: Not needed for CSS-only changes

**Acceptance**:
- Animations defined correctly
- GPU acceleration used (transform, opacity)
- Reduced motion media query works
- No CSS syntax errors

---

## PM Verification After Wave 1

**Checklist**:
- [ ] NetworkBackground renders without console errors
- [ ] SVG structure correct (inspect with DevTools)
- [ ] Tailwind colors available (test in browser console: `getComputedStyle`)
- [ ] Animation keyframes defined (DevTools Animations panel)
- [ ] REQ-1.4: Performance profiling (60fps, <5% CPU)
  - Open DevTools → Performance
  - Record 30s idle
  - Verify FPS ≥60, CPU <5%
- [ ] REQ-1.5: Reduced motion testing
  - Enable OS reduced motion setting
  - Verify animations disabled/minimized
- [ ] Mark REQ-1.1, 1.2, 1.3, 1.4, 1.5 complete in RTM.md
- [ ] Git commit: `feat(foundation): Add NetworkBackground, animations, colors`

**Escape Hatch**: If any test fails, STOP and fix before Wave 2

---

## Wave 2: Component Library (Parallel - 3 Agents)

### Agent 4: GlassCard Component
**RTM**: REQ-2.1, 2.2, 2.3
**Session**: Create new Wave 2 session
**Duration**: 20-25 minutes

**Tasks**:
1. Create `site/app/components/GlassCard.tsx` with TypeScript interface:
   ```typescript
   interface GlassCardProps {
     children: React.ReactNode;
     className?: string;
     enableTilt?: boolean;
   }
   ```
2. Implement glassmorphic base styles:
   - Background: `rgba(255, 255, 255, 0.1)` dark / `rgba(0, 0, 0, 0.05)` light
   - Backdrop-filter: `blur(12px) saturate(180%)`
   - Border: `1px solid rgba(255, 255, 255, 0.2)`
   - Border-radius: 16px
   - Box-shadow: `0 8px 32px 0 rgba(0, 0, 0, 0.37)`
3. Add `@supports (backdrop-filter: blur())` fallback to solid background
4. Add hover state:
   - Transform: `translateY(-8px) scale(1.02)`
   - Enhanced shadow: `0 16px 48px 0 rgba(0, 0, 0, 0.45)`
   - Transition: `300ms cubic-bezier(0.4, 0, 0.2, 1)`
5. Implement optional 3D tilt (if enableTilt=true):
   - Desktop only: CSS perspective transform based on mouse position
   - Disabled on touch: `@media (hover: none)` or `@media (pointer: coarse)`

**Deliverable**: `site/app/components/GlassCard.tsx` (~80 lines)

**Next.js MCP**: Use for component testing

**Acceptance**:
- Glass effect visible in supported browsers
- Fallback works in unsupported browsers
- Hover animation smooth
- Tilt works on desktop, disabled on touch
- TypeScript compilation passes

---

### Agent 5: AnimatedText Component
**RTM**: REQ-3.1, 3.3
**Session**: Same as Agent 4
**Duration**: 15-20 minutes

**Tasks**:
1. Create `site/app/components/AnimatedText.tsx` with TypeScript interface:
   ```typescript
   interface AnimatedTextProps {
     children: React.ReactNode;
     className?: string;
     gradientFrom?: string;
     gradientTo?: string;
   }
   ```
2. Implement gradient clip text:
   - `background: linear-gradient(135deg, from, to)`
   - `background-clip: text`
   - `-webkit-background-clip: text`
   - `color: transparent`
3. Apply shimmer animation:
   - `background-size: 200% 200%`
   - `animation: gradient-shift 10-15s ease-in-out infinite`
4. Add fallback for unsupported gradient clip (solid color)
5. Verify WCAG AA contrast (≥4.5:1) at all animation states

**Deliverable**: `site/app/components/AnimatedText.tsx` (~60 lines)

**Next.js MCP**: Use for accessibility validation

**Acceptance**:
- Gradient clips to text correctly
- Shimmer animation smooth
- Contrast ratio ≥4.5:1 (validated with WAVE)
- Fallback provides solid color
- Reduced motion support works

---

### Agent 6: Gradient Mesh Background
**RTM**: REQ-3.2
**Session**: Same as Agent 4
**Duration**: 15-20 minutes

**Tasks**:
1. Decide implementation location: `site/app/page.tsx` (inline styles) or `site/app/globals.css` (global styles)
2. Create 3-4 radial gradients with Deep Knowledge colors:
   ```css
   background:
     radial-gradient(at 0% 0%, #0ea5e9 0%, transparent 50%),
     radial-gradient(at 100% 100%, #a855f7 0%, transparent 50%),
     radial-gradient(at 50% 50%, #06b6d4 0%, transparent 50%),
     #0f172a; /* Base color */
   ```
3. Implement morph animation (60-120s cycle):
   ```css
   @keyframes mesh-morph {
     0%, 100% { /* Position 1 */ }
     33% { /* Position 2 */ }
     66% { /* Position 3 */ }
   }
   ```
4. Apply `will-change: transform` only during animation (performance)
5. Test performance (no jank in profiler)

**Deliverable**: Gradient mesh background in `page.tsx` or `globals.css` (~30 lines)

**Next.js MCP**: Use for performance validation

**Acceptance**:
- Gradient mesh renders correctly
- Morph animation smooth
- No performance impact (jank-free)
- Will-change used appropriately

---

## PM Verification After Wave 2

**Checklist**:
- [ ] All 3 components render independently without errors
- [ ] GlassCard hover states work smoothly
- [ ] GlassCard tilt effect works on desktop, disabled on touch
- [ ] AnimatedText gradient and shimmer work
- [ ] Gradient mesh background animates smoothly
- [ ] REQ-3.4: Contrast ratio validation
  - Install WAVE extension
  - Load page with components
  - Verify all text ≥4.5:1 contrast
- [ ] Mark REQ-2.1, 2.2, 2.3, 3.1, 3.2, 3.3, 3.4 complete in RTM.md
- [ ] Git commit: `feat(components): Add GlassCard, AnimatedText, gradient mesh`

**Escape Hatch**: If contrast fails or components broken, STOP and fix

---

## Wave 3: Integration (Sequential - 3 Agents)

### Agent 7: Page Integration
**RTM**: Page.tsx integration
**Session**: Create new Wave 3 session
**Duration**: 20-25 minutes

**Tasks**:
1. Query Next.js MCP for current page.tsx structure
2. Import components:
   ```typescript
   import { NetworkBackground } from './components/NetworkBackground';
   import { GlassCard } from './components/GlassCard';
   import { AnimatedText } from './components/AnimatedText';
   ```
3. Add NetworkBackground as page background layer (absolute positioned)
4. Replace existing feature card divs with `<GlassCard>` wrapper
5. Wrap hero headline with `<AnimatedText>`
6. Integrate gradient mesh background (if not in globals.css)

**Deliverable**: `site/app/page.tsx` (~50 line changes)

**Next.js MCP**: Use extensively for runtime error checking

**Acceptance**:
- All components integrated without errors
- Layout preserved
- Animations work together
- No z-index or layering issues

---

### Agent 8: Layout & Responsive Fixes
**RTM**: Responsive validation
**Session**: Same as Agent 7
**Duration**: 15-20 minutes
**Dependencies**: Agent 7 completion

**Tasks**:
1. Test responsive behavior in DevTools:
   - Mobile: 375px × 667px
   - Tablet: 768px × 1024px
   - Desktop: 1920px × 1080px
2. Fix any layout issues:
   - Cards stack correctly (1 col mobile, 2 col tablet, 4 col desktop)
   - Typography scales appropriately
   - NetworkBackground node count adjusts
3. Verify no horizontal scroll at any breakpoint
4. Test touch targets ≥44x44px on mobile

**Deliverable**: Layout fixes in `page.tsx` or component files

**Next.js MCP**: Use for layout diagnostics

**Acceptance**:
- Layout correct at all 3 breakpoints
- No horizontal scroll
- Touch targets adequate

---

### Agent 9: Performance Optimization Pass
**RTM**: REQ-1.4 final validation
**Session**: Same as Agent 7
**Duration**: 15-20 minutes
**Dependencies**: Agent 8 completion

**Tasks**:
1. Profile animations in Chrome DevTools Performance tab:
   - Record 30s idle
   - Record 10s interaction (hover, scroll)
2. Optimize any jank or performance issues:
   - Check for layout thrashing
   - Verify GPU acceleration (transform/opacity only)
   - Check will-change usage
3. Verify 60fps target maintained
4. Verify CPU usage: <5% idle, <10% hover

**Deliverable**: Performance optimizations (if needed)

**Next.js MCP**: Query runtime diagnostics

**Acceptance**:
- FPS ≥60 consistently
- CPU usage within targets
- No layout thrashing detected

---

## PM Verification After Wave 3

**Checklist**:
- [ ] Full page visual inspection (all components integrated)
- [ ] Responsive testing passed at all breakpoints
- [ ] Performance profiling passed (60fps, CPU targets)
- [ ] No console errors or warnings
- [ ] Next.js MCP reports no runtime errors
- [ ] Mark integration complete in RTM.md
- [ ] Git commit: `feat(integration): Integrate all components into landing page`

**Escape Hatch**: If performance < targets, STOP and optimize

---

## Wave 4: Polish (Parallel - 3 Agents)

### Agent 10: Button Ripple Effects
**RTM**: REQ-4.1
**Session**: Create new Wave 4 session
**Duration**: 20-25 minutes

**Tasks**:
1. Identify primary CTA buttons in page.tsx
2. Implement CSS ripple effect:
   - Pseudo-element (::after)
   - Position absolute, centered on click
   - Transform: scale animation
   - Opacity: fade-out animation
   - Duration: 500ms
3. Add click event handling for ripple positioning (if needed via JS)
4. Test ripple animation
5. Verify no performance impact

**Deliverable**: Ripple effects on primary CTAs

**Next.js MCP**: Use for component testing

**Acceptance**:
- Ripple effect radiates from click point
- 500ms fade-out smooth
- No performance degradation

---

### Agent 11: Enhanced Hover States
**RTM**: REQ-4.2
**Session**: Same as Agent 10
**Duration**: 20-25 minutes

**Tasks**:
1. Audit all interactive elements:
   - Primary CTA (Get Started)
   - Secondary CTA (Browse Tools)
   - Tertiary CTA (Documentation)
   - Links (if any)
   - Cards (already have hover via GlassCard)
2. Ensure consistent hover timing (300ms)
3. Add distinct hover feedback where missing
4. Test hover animations smoothness

**Deliverable**: Enhanced hover states in CSS or component files

**Next.js MCP**: Use for visual validation

**Acceptance**:
- All interactive elements have hover feedback
- Consistent 300ms timing
- Smooth animations

---

### Agent 12: Page Load Animation Sequence
**RTM**: REQ-4.3
**Session**: Same as Agent 10
**Duration**: 20-25 minutes

**Tasks**:
1. Implement staggered fade-in for page elements:
   - Hero text: 0ms delay
   - Feature cards: 100ms, 200ms, 300ms, 400ms delays
   - CTAs: 500ms delay
2. Use CSS animations (opacity + transform)
3. Ensure no FOUC (Flash of Unstyled Content)
4. Test CLS (Cumulative Layout Shift) < 0.1:
   - Hard refresh testing
   - Lighthouse CLS metric

**Deliverable**: Page load animation in CSS

**Next.js MCP**: Use for CLS measurement

**Acceptance**:
- Staggered fade-in smooth
- No FOUC
- CLS < 0.1 (Lighthouse)

---

## PM Verification After Wave 4

**Checklist**:
- [ ] Test all micro-interactions (ripples, hovers, load animation)
- [ ] Verify consistent hover states across page
- [ ] Test page load animation (hard refresh multiple times)
- [ ] Run Lighthouse CLS check (< 0.1)
- [ ] Mark REQ-4.1, 4.2, 4.3 complete in RTM.md
- [ ] Git commit: `feat(polish): Add button ripples, hover enhancements, load animation`

**Escape Hatch**: If CLS > 0.1, STOP and fix layout shift

---

## Wave 5: Testing (Parallel - 3 Agents)

### Agent 13: Cross-Browser & Responsive Testing
**RTM**: REQ-T.5, T.6
**Session**: Create new Wave 5 session
**Duration**: 25-30 minutes

**Tasks**:
1. Test in Chrome, Firefox, Safari, Edge (120+):
   - Visual inspection
   - Animation smoothness
   - Glass effect rendering
   - Gradient rendering
2. Take screenshots at 3 breakpoints per browser (12 total):
   - 375px, 768px, 1920px × 4 browsers
3. Test responsive behavior:
   - Layout adapts correctly
   - Node count reduces on mobile
   - Typography scales
   - Touch targets adequate
4. Document any browser-specific issues
5. Fix critical issues discovered

**Deliverable**: Cross-browser validation report + fixes

**Next.js MCP**: Query errors across browsers (if available)

**Acceptance**:
- Consistent rendering across browsers
- Animations work in all browsers
- Fallbacks active where needed
- Responsive behavior correct

---

### Agent 14: Performance Testing
**RTM**: REQ-T.1, T.2
**Session**: Same as Agent 13
**Duration**: 25-30 minutes

**Tasks**:
1. Run Lighthouse performance audit:
   ```bash
   npx lighthouse http://localhost:3000 \
     --only-categories=performance \
     --output=html \
     --output-path=./lighthouse-performance.html
   ```
   Target: ≥95
2. Chrome DevTools performance profiling:
   - 30s idle recording
   - 10s interaction recording (hover, scroll)
3. Verify Core Web Vitals:
   - FCP < 1.5s
   - LCP < 2.5s
   - CLS < 0.1
   - TBT < 200ms
4. Verify animation performance:
   - 60fps consistently
   - CPU <5% idle, <10% hover
5. Fix performance issues if discovered

**Deliverable**: Performance audit report + optimizations

**Next.js MCP**: Use for performance diagnostics

**Acceptance**:
- Lighthouse Performance ≥95
- All Core Web Vitals pass
- Animation performance targets met

---

### Agent 15: Accessibility Testing
**RTM**: REQ-T.3, T.4
**Session**: Same as Agent 13
**Duration**: 25-30 minutes

**Tasks**:
1. Run Lighthouse accessibility audit:
   ```bash
   npx lighthouse http://localhost:3000 \
     --only-categories=accessibility \
     --output=html \
     --output-path=./lighthouse-accessibility.html
   ```
   Target: 100
2. Run axe DevTools scan:
   - Install axe DevTools extension
   - Scan page
   - Target: 0 critical/serious issues
3. Manual keyboard navigation:
   - Tab through all interactive elements
   - Verify focus indicators visible
   - Verify no keyboard traps
4. Screen reader testing (VoiceOver or NVDA):
   - Page title announced
   - Headings announced with levels
   - Decorative SVG ignored
   - Links have clear purpose
5. Color contrast validation (WAVE extension):
   - All text ≥4.5:1
   - Large text ≥3:1
6. Reduced motion testing:
   - Enable OS setting
   - Verify animations disabled
7. Fix accessibility issues discovered

**Deliverable**: Accessibility audit report + fixes

**Next.js MCP**: Query accessibility diagnostics (if available)

**Acceptance**:
- Lighthouse Accessibility = 100
- 0 critical/serious axe issues
- Keyboard navigation 100% functional
- Screen reader announcements correct
- Contrast ratios pass
- Reduced motion works

---

## PM Verification After Wave 5

**Checklist**:
- [ ] Review all 3 test reports (cross-browser, performance, accessibility)
- [ ] Verify Lighthouse Performance ≥95
- [ ] Verify Lighthouse Accessibility = 100
- [ ] Verify cross-browser compatibility (4 browsers × 3 breakpoints)
- [ ] Address any critical issues discovered
- [ ] Mark REQ-T.1 through T.6 complete in RTM.md
- [ ] Git commit: `test: Complete testing validation and fixes`
- [ ] Update RTM.md: Mark all requirements ✅ complete
- [ ] Request final UAT sign-off from user

**Escape Hatch**:
- If Lighthouse Performance < 90 → Roll back and re-optimize
- If Lighthouse Accessibility < 95 → Roll back and fix
- If critical cross-browser issues → Fix before UAT

---

## Final Deliverables

1. ✅ All 17 RTM requirements complete
2. ✅ 6 files created/modified:
   - NEW: `site/app/components/NetworkBackground.tsx`
   - NEW: `site/app/components/GlassCard.tsx`
   - NEW: `site/app/components/AnimatedText.tsx`
   - MODIFIED: `site/app/globals.css`
   - MODIFIED: `site/tailwind.config.ts`
   - MODIFIED: `site/app/page.tsx`
3. ✅ Performance validated (Lighthouse ≥95)
4. ✅ Accessibility validated (Lighthouse = 100)
5. ✅ Cross-browser validated (Chrome, Firefox, Safari, Edge)
6. ✅ Responsive validated (375px, 768px, 1920px)
7. ✅ Git history: 5 clean commits (foundation, components, integration, polish, testing)

---

## Document Status

**Status**: ✅ Planning Complete
**Next Action**: Execute Wave 1 dispatch (3 parallel coding-agents)
**Last Updated**: 2025-10-28
