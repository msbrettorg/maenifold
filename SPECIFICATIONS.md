# Sprint Specifications: Landing Page Redesign
## "The Living Manifold"

**Sprint ID**: `sprint-20251028-living-manifold`
**Branch**: `prototype/local-dev`
**PRD**: [docs/PRD-landing-page-redesign.md](docs/PRD-landing-page-redesign.md)
**RTM**: [RTM.md](RTM.md)
**Discovery Sessions**: `session-1761684454166`, `session-1761684591094`
**Specification Session**: `session-1761684647133`
**Created**: 2025-10-28

---

## Executive Summary

This document provides detailed implementation specifications for transforming the Maenifold landing page into an award-winning visual showcase embodying knowledge graph and manifold concepts through pure CSS animations and glassmorphic design.

**Implementation Approach**: 3 new React components + CSS enhancements + Tailwind config extension
**Timeline**: 4 waves of parallel agent dispatch (9-14 hours estimated)
**Stack**: Next.js 16 + React 19 + TypeScript 5.9 + Tailwind 4.1

---

## 1. Functional Requirements

### 1.1 NetworkBackground Component

**File**: `site/app/components/NetworkBackground.tsx` (NEW - ~100 lines)

**Purpose**: Animated SVG network visualization representing knowledge graph concepts with floating nodes and connection lines.

**Requirements**:
- **Node Generation**:
  - Generate 20-30 circular nodes with randomized positions
  - Node diameter: 8-16px (random within range)
  - Node opacity: 0.3-0.8 (random within range)
  - Node colors: Use Deep Knowledge palette (blue #0ea5e9, purple #a855f7, cyan #06b6d4)
  - Each node has unique key for React rendering

- **Connection Lines**:
  - Connect nodes within distance threshold (e.g., 300px)
  - Line width: 1-2px
  - Line opacity: 0.2-0.4
  - Gradient stroke from node color to node color

- **Animations**:
  - Float animation: 30-60s duration per node (randomized)
  - Pulse animation on lines: 3-5s duration, infinite
  - All animations use CSS @keyframes
  - GPU-accelerated: transform: translate3d() and opacity only

- **Responsive Behavior**:
  - Desktop (≥1024px): 20-30 nodes
  - Tablet (640-1023px): 15-20 nodes
  - Mobile (<640px): 10-15 nodes

- **Accessibility**:
  - SVG has `role="presentation"` (decorative)
  - `aria-hidden="true"`
  - Animations disabled with `prefers-reduced-motion`

**Technical Implementation**:
```typescript
interface NetworkBackgroundProps {
  className?: string;
}

export function NetworkBackground({ className }: NetworkBackgroundProps) {
  // Component implementation
}
```

**Acceptance Criteria**:
- [ ] SVG renders with viewBox="0 0 1920 1080"
- [ ] Node count appropriate for breakpoint (verified via DevTools)
- [ ] Animations run at 60fps (Chrome Performance profiler)
- [ ] CPU usage <5% idle (Task Manager + DevTools)
- [ ] `prefers-reduced-motion` disables animations
- [ ] No console errors or warnings

---

### 1.2 GlassCard Component

**File**: `site/app/components/GlassCard.tsx` (NEW - ~80 lines)

**Purpose**: Reusable glassmorphic card component with backdrop blur, hover effects, and optional 3D tilt for feature cards.

**Requirements**:
- **Base Styles**:
  - Background: `rgba(255, 255, 255, 0.1)` dark mode / `rgba(0, 0, 0, 0.05)` light mode
  - Backdrop filter: `blur(12px) saturate(180%)`
  - Border: 1px solid `rgba(255, 255, 255, 0.2)`
  - Border radius: 16px
  - Box shadow: `0 8px 32px 0 rgba(0, 0, 0, 0.37)`

- **Hover State**:
  - Transform: `translateY(-8px) scale(1.02)`
  - Enhanced shadow: `0 16px 48px 0 rgba(0, 0, 0, 0.45)`
  - Transition: 300ms cubic-bezier(0.4, 0, 0.2, 1)

- **3D Tilt Effect** (Optional):
  - Desktop only: `perspective(1000px) rotateX(deg) rotateY(deg) translateY(-8px)`
  - Tilt based on mouse position within card
  - Disabled on touch devices (CSS media query: `hover: none`)

- **Fallback**:
  - If `backdrop-filter` not supported: solid background color
  - Feature detection via CSS `@supports` rule

**Technical Implementation**:
```typescript
interface GlassCardProps {
  children: React.ReactNode;
  className?: string;
  enableTilt?: boolean;
}

export function GlassCard({ children, className, enableTilt = false }: GlassCardProps) {
  // Component implementation
}
```

**Acceptance Criteria**:
- [ ] Glass effect visible in supported browsers (Chrome, Firefox, Safari, Edge 120+)
- [ ] Fallback background renders in unsupported browsers
- [ ] Hover animation smooth (no jank, 60fps)
- [ ] Tilt effect works on desktop mouse movement
- [ ] Tilt disabled on touch devices
- [ ] TypeScript compilation passes with no errors

---

### 1.3 AnimatedText Component

**File**: `site/app/components/AnimatedText.tsx` (NEW - ~60 lines)

**Purpose**: Gradient text component with shimmer animation for hero headlines.

**Requirements**:
- **Gradient Text**:
  - Use `background-clip: text` and `-webkit-background-clip: text`
  - `color: transparent` to show gradient
  - Gradient: `linear-gradient(135deg, color1, color2)`
  - Default colors from Deep Knowledge palette

- **Shimmer Animation**:
  - Animate `background-position` for gradient shift
  - `background-size: 200% 200%` for animation space
  - Animation duration: 10-15s (slow, subtle)
  - Easing: ease-in-out

- **Accessibility**:
  - Ensure contrast ratio ≥4.5:1 at all animation states
  - Animation respects `prefers-reduced-motion`
  - Fallback to solid color if gradient not supported

**Technical Implementation**:
```typescript
interface AnimatedTextProps {
  children: React.ReactNode;
  className?: string;
  gradientFrom?: string;
  gradientTo?: string;
}

export function AnimatedText({
  children,
  className,
  gradientFrom = '#0ea5e9',
  gradientTo = '#a855f7'
}: AnimatedTextProps) {
  // Component implementation
}
```

**Acceptance Criteria**:
- [ ] Gradient renders and clips to text correctly
- [ ] Shimmer animation runs smoothly (60fps)
- [ ] Text readable (contrast ≥4.5:1 validated with WAVE)
- [ ] Fallback provides solid color text
- [ ] Animation disabled with `prefers-reduced-motion`
- [ ] TypeScript compilation passes

---

## 2. Non-Functional Requirements

### 2.1 Performance Requirements

**Load Performance**:
- First Contentful Paint (FCP): < 1.5s
- Largest Contentful Paint (LCP): < 2.5s
- Cumulative Layout Shift (CLS): < 0.1
- Total Blocking Time (TBT): < 200ms

**Runtime Performance**:
- Animation Frame Rate: 60fps (consistent)
- CPU Usage (Idle): < 5% average
- CPU Usage (Hover): < 10% average
- Memory Increase: < 50MB from baseline
- JavaScript Execution: < 5ms per frame

**Optimization Techniques**:
- Use `transform` and `opacity` for all animations (GPU-accelerated)
- Apply `will-change` only during active animations
- Avoid layout thrashing (batch DOM reads/writes)
- Minimize re-renders (React.memo for static components)
- Inline critical CSS and SVG

**Testing**:
```bash
# Lighthouse audit
npx lighthouse http://localhost:3000 \
  --only-categories=performance \
  --output=html \
  --output-path=./lighthouse-performance.html

# Target: Performance score ≥95
```

---

### 2.2 Accessibility Requirements (WCAG 2.1 Level AA)

**Color Contrast**:
- Normal text (< 18px): ≥ 4.5:1 contrast ratio
- Large text (≥ 18px or ≥ 14px bold): ≥ 3:1 contrast ratio
- UI components: ≥ 3:1 contrast ratio
- Validated across all gradient animation states

**Keyboard Navigation**:
- All interactive elements focusable with Tab key
- Logical tab order (top to bottom, left to right)
- Visible focus indicators (outline or ring)
- No keyboard traps
- Shift+Tab navigates backward

**Reduced Motion**:
```css
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
```

**Screen Readers**:
- Decorative SVG: `role="presentation"` and `aria-hidden="true"`
- Interactive elements: Clear labels or `aria-label`
- Heading hierarchy: h1 → h2 → h3 (no skipping)
- Links: Descriptive text (not "click here")

**Testing**:
```bash
# Lighthouse accessibility audit
npx lighthouse http://localhost:3000 \
  --only-categories=accessibility \
  --output=html \
  --output-path=./lighthouse-accessibility.html

# Target: Accessibility score = 100
```

---

### 2.3 Compatibility Requirements

**Browser Support**:
- Chrome/Edge: 120+ (Chromium-based)
- Firefox: 120+
- Safari: 17+
- Opera: Not tested (nice-to-have)

**Graceful Degradation**:
- `backdrop-filter` fallback: Solid background color
- CSS Grid fallback: Flexbox (not needed, Grid widely supported)
- Advanced animations: Skip with `prefers-reduced-motion`

**Device Support**:
- Desktop: 1920x1080 (primary)
- Tablet: 768x1024 (secondary)
- Mobile: 375x667 (iPhone SE baseline)

**GitHub Pages Constraints**:
- Static export only (no SSR)
- No Node.js runtime features
- No API routes
- All assets bundled at build time

---

### 2.4 Security Requirements (Ma Protocol: NO FAKE SECURITY)

**Trust the Framework**:
- Next.js handles XSS prevention (React escaping)
- No manual sanitization needed (trust React)
- No external dependencies beyond stack (no attack surface)

**No Unnecessary Restrictions**:
- No path validation (trust the file system)
- No rate limiting (local development)
- No input validation beyond TypeScript types

**What We DO Ensure**:
- No external CDN dependencies (inline everything)
- No analytics or tracking scripts
- No third-party JavaScript
- All assets served from same origin

---

## 3. Implementation Constraints

### 3.1 File Modifications

**New Files** (3):
1. `site/app/components/NetworkBackground.tsx` (~100 lines)
   - SVG network generation
   - Node/line rendering logic
   - Animation CSS classes

2. `site/app/components/GlassCard.tsx` (~80 lines)
   - Glassmorphic styles
   - Hover state logic
   - 3D tilt effect (optional)

3. `site/app/components/AnimatedText.tsx` (~60 lines)
   - Gradient text styles
   - Shimmer animation
   - Fallback handling

**Modified Files** (3):
1. `site/app/globals.css` (+~50 lines)
   - @keyframes for float animation
   - @keyframes for pulse animation
   - @keyframes for gradient-shift animation
   - Reduced motion media query enhancements

2. `site/tailwind.config.ts` (+~20 lines)
   - Custom colors: Deep Knowledge palette
   - Custom utilities (if needed)
   - Theme extension

3. `site/app/page.tsx` (~50 line changes)
   - Import new components
   - Replace existing cards with GlassCard
   - Wrap hero text with AnimatedText
   - Add NetworkBackground as page background layer

**No Changes To**:
- `site/app/components/Header.tsx` (existing, not impacted)
- `site/app/components/Footer.tsx` (existing, not impacted)
- `site/app/layout.tsx` (no changes needed)
- `site/package.json` (no new dependencies)

---

### 3.2 Technology Stack

**Allowed** (Existing):
- ✅ Next.js 16.0.0
- ✅ React 19.2.0
- ✅ TypeScript 5.9.3
- ✅ Tailwind CSS 4.1.16
- ✅ PostCSS 8.5.6
- ✅ CSS3 (animations, transforms, gradients)

**Forbidden** (Per RTM MUST NOT):
- ❌ Three.js / WebGL
- ❌ GSAP / Anime.js / Framer Motion
- ❌ Canvas-based animation libraries
- ❌ Video processing libraries
- ❌ External CDN assets
- ❌ JavaScript animation frameworks

**Rationale**: GitHub Pages static hosting + performance targets + Ma Protocol (no unnecessary abstractions)

---

## 4. Testing Specifications

### 4.1 Component Unit Testing

**NetworkBackground Component**:
```typescript
// Test cases
- SVG structure validation (viewBox, nodes, lines)
- Node count by breakpoint (20-30 desktop, 10-15 mobile)
- Animation class application
- Accessibility attributes (role, aria-hidden)
- Reduced motion behavior
```

**GlassCard Component**:
```typescript
// Test cases
- Props interface validation (children, className, enableTilt)
- Glass effect styles applied
- Hover state styles applied
- Tilt effect on desktop only
- Touch device detection (no tilt)
```

**AnimatedText Component**:
```typescript
// Test cases
- Gradient rendering
- Shimmer animation applied
- Contrast ratio validation (≥4.5:1)
- Fallback solid color
- Reduced motion behavior
```

---

### 4.2 Visual Regression Testing

**Cross-Browser Matrix**:
| Browser | Version | Desktop | Mobile | Status |
|---------|---------|---------|--------|--------|
| Chrome  | 120+    | ✅      | ✅     | Pending |
| Firefox | 120+    | ✅      | ✅     | Pending |
| Safari  | 17+     | ✅      | ✅     | Pending |
| Edge    | 120+    | ✅      | ✅     | Pending |

**Responsive Breakpoints**:
- Mobile: 375px × 667px (iPhone SE)
- Tablet: 768px × 1024px (iPad)
- Desktop: 1920px × 1080px (Full HD)

**Testing Procedure**:
1. Load page in each browser
2. Take full-page screenshots at each breakpoint
3. Visual inspection for consistency
4. Animation smoothness validation (60fps target)

---

### 4.3 Performance Testing

**Lighthouse Audits**:
```bash
# Performance audit
npx lighthouse http://localhost:3000 \
  --only-categories=performance \
  --output=html \
  --output-path=./lighthouse-performance.html

# Targets
- Performance: ≥ 95
- FCP: < 1.5s
- LCP: < 2.5s
- CLS: < 0.1
- TBT: < 200ms
```

**Runtime Profiling**:
```bash
# Chrome DevTools Performance tab
1. Open DevTools → Performance
2. Click Record
3. Let page idle for 30 seconds
4. Stop recording
5. Verify:
   - FPS: 60fps (green line, no drops)
   - CPU: < 5% average
   - Memory: Stable (no leaks)
```

---

### 4.4 Accessibility Testing

**Automated Tools**:
- axe DevTools: 0 critical/serious issues
- Lighthouse: Accessibility score = 100
- WAVE: No errors or contrast failures

**Manual Testing**:
1. **Keyboard Navigation**:
   - Tab through all interactive elements
   - Verify focus indicators visible
   - Verify no keyboard traps

2. **Screen Reader** (VoiceOver/NVDA):
   - Page title announced
   - Headings announced with levels
   - Decorative SVG ignored
   - Links have clear purpose

3. **Color Contrast**:
   - WAVE extension validation
   - Manual contrast checker for gradient states
   - All text ≥4.5:1, large text ≥3:1

4. **Reduced Motion**:
   - Enable OS setting
   - Verify animations disabled/minimized
   - Page remains functional

---

## 5. Acceptance Criteria

### Component-Level Acceptance

**NetworkBackground**:
- [ ] SVG renders with correct viewBox and aspect ratio
- [ ] Node count appropriate for breakpoint
- [ ] Animations run at 60fps (verified)
- [ ] CPU usage <5% idle
- [ ] Reduced motion support works
- [ ] No console errors

**GlassCard**:
- [ ] Glass effect visible in supported browsers
- [ ] Fallback works in unsupported browsers
- [ ] Hover animation smooth (60fps)
- [ ] Tilt effect works on desktop
- [ ] Tilt disabled on touch devices
- [ ] TypeScript types correct

**AnimatedText**:
- [ ] Gradient clips to text correctly
- [ ] Shimmer animation smooth (60fps)
- [ ] Contrast ratio ≥4.5:1
- [ ] Fallback provides solid color
- [ ] Reduced motion support works

---

### System-Level Acceptance

**Performance**:
- [ ] Lighthouse Performance ≥ 95
- [ ] FCP < 1.5s, LCP < 2.5s, CLS < 0.1, TBT < 200ms
- [ ] 60fps animations maintained
- [ ] CPU usage targets met

**Accessibility**:
- [ ] Lighthouse Accessibility = 100
- [ ] 0 critical/serious issues (axe DevTools)
- [ ] Keyboard navigation 100% functional
- [ ] Screen reader announcements correct
- [ ] Color contrast validated
- [ ] Reduced motion implementation complete

**Cross-Browser**:
- [ ] Chrome: All features work
- [ ] Firefox: All features work
- [ ] Safari: All features work
- [ ] Edge: All features work

**Responsive**:
- [ ] Mobile (375px): Layout correct, no horizontal scroll
- [ ] Tablet (768px): Layout correct, cards in 2 cols
- [ ] Desktop (1920px): Layout correct, cards in 4 cols

---

## 6. Wave 1 Agent Dispatch Specifications

### Task 1: NetworkBackground Component
**Agent**: coding-agent
**Estimated Time**: 20-25 minutes
**Sequential Thinking Session**: Create new session for this task
**Deliverable**: `site/app/components/NetworkBackground.tsx`

**Specifications**:
- Create React Server Component with TypeScript
- Generate 20-30 SVG nodes with randomized positions
- Create connection lines between nearby nodes
- Apply animation CSS classes (defined in Wave 1 Task 3)
- Implement responsive node count reduction
- Add accessibility attributes (role="presentation", aria-hidden)
- Export as named export: `export function NetworkBackground({ className }: NetworkBackgroundProps)`

**Acceptance**:
- Component renders without errors
- SVG structure correct (viewBox, nodes, lines)
- TypeScript compilation passes
- Props interface defined correctly

---

### Task 2: Tailwind Config Extension
**Agent**: coding-agent
**Estimated Time**: 10-15 minutes
**Sequential Thinking Session**: Same as Task 1
**Deliverable**: `site/tailwind.config.ts` (modified)

**Specifications**:
- Add Deep Knowledge color palette to theme.extend.colors
- Colors: primary (#0ea5e9), secondary (#a855f7), accent (#06b6d4)
- Gradients: Add custom gradient utilities if needed
- Verify build succeeds after changes

**Acceptance**:
- Build succeeds (`npm run build` or dev server restart)
- Colors available as Tailwind utilities (e.g., `bg-primary`, `text-secondary`)
- TypeScript types updated (if needed)

---

### Task 3: Animation Keyframes
**Agent**: coding-agent
**Estimated Time**: 15-20 minutes
**Sequential Thinking Session**: Same as Task 1
**Deliverable**: `site/app/globals.css` (modified)

**Specifications**:
- Add @keyframes float: translate3d animation, 30-60s duration, ease-in-out
- Add @keyframes pulse: opacity animation, 3-5s duration, infinite
- Add @keyframes gradient-shift: background-position animation for shimmer
- Add enhanced reduced motion media query
- Verify animations work in browser

**Acceptance**:
- Animations defined correctly
- GPU acceleration used (transform, opacity)
- Reduced motion media query disables animations
- No syntax errors in CSS

---

## 7. Definition of Done

**Specifications Complete When**:
1. ✅ All component specifications documented with acceptance criteria
2. ✅ Non-functional requirements defined (performance, accessibility, compatibility)
3. ✅ Implementation constraints listed (files, dependencies)
4. ✅ Testing specifications complete (unit, visual, performance, accessibility)
5. ✅ Wave 1 agent dispatch specifications ready
6. ✅ Document reviewed and committed to git

**Sprint Complete When**:
1. All component acceptance criteria met
2. All system-level acceptance criteria met
3. All RTM requirements validated
4. UAT sign-off received
5. Workflow memory artifacts generated

---

**Document Status**: ✅ Complete
**Next Action**: Advance workflow to Step 4 (RTM Creation) and dispatch Wave 1 agents
**Last Updated**: 2025-10-28
