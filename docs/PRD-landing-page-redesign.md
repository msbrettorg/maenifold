# Product Requirements Document: Landing Page Redesign
## "The Living Manifold"

**Version**: 1.0
**Date**: 2025-10-28
**Author**: Claude Code
**Status**: Draft
**Project**: Maenifold Landing Page Visual Enhancement

---

## Executive Summary

This PRD outlines the comprehensive redesign of the Maenifold landing page to create an award-winning, visually stunning experience that embodies the core concept of a living knowledge graph. The redesign transforms the current functional landing page into an "achingly beautiful" showcase that visually represents knowledge as an interconnected manifold while maintaining static site compatibility for GitHub Pages hosting.

### Vision Statement
> "Create a landing page experience where visitors feel they're looking at knowledge itself—alive, interconnected, breathing—through a mathematically inspired, visually captivating interface that stops people in their tracks."

---

## 1. Goals & Objectives

### Primary Goals
1. **Visual Impact**: Create a "wow" moment within the first 3 seconds of page load
2. **Brand Embodiment**: Visually represent the knowledge graph and manifold concepts through design
3. **Performance**: Maintain fast load times and smooth 60fps animations on GitHub Pages
4. **Conversion**: Increase engagement metrics (time on page, click-through to tools/docs)

### Success Metrics
- **Subjective**: "Achingly beautiful" feedback from stakeholders
- **Performance**: Page load < 2s, animations at 60fps
- **Engagement**: 30%+ increase in average session duration
- **Technical**: 95+ Lighthouse performance score maintained
- **Compatibility**: Perfect rendering across modern browsers (Chrome, Firefox, Safari, Edge)

---

## 2. User Stories

### Primary User: First-Time Visitor
**AS A** developer/researcher visiting Maenifold for the first time
**I WANT TO** immediately understand what Maenifold is through stunning visuals
**SO THAT** I feel compelled to explore further and understand the knowledge graph concept

**Acceptance Criteria**:
- Visual network/graph representation loads within 1 second
- Hero message is readable and impactful
- Clear path to next action (Get Started, Browse Tools, Documentation)
- Smooth, performant animations that don't distract from content

### Secondary User: Returning Technical User
**AS A** technical user returning to explore Maenifold
**I WANT TO** quickly navigate to tools and documentation
**SO THAT** I can implement Maenifold in my workflow

**Acceptance Criteria**:
- Navigation remains clear and accessible
- Interactive elements provide immediate feedback
- Design enhances rather than hinders information architecture

---

## 3. Design Specifications

### 3.1 Visual Design System

#### Color Palette: "Deep Knowledge"
```css
/* Primary Colors */
--color-primary: #0ea5e9;      /* Sky Blue - clarity, intelligence */
--color-secondary: #a855f7;     /* Purple - creativity, depth */
--color-accent: #06b6d4;        /* Cyan - connections, energy */

/* Backgrounds */
--color-dark-base: #0f172a;     /* Slate 900 - deep space */
--color-dark-mid: #1e293b;      /* Slate 800 */
--color-light-base: #f8fafc;    /* Slate 50 - light, emergence */

/* Gradients */
--gradient-primary: linear-gradient(135deg, #0ea5e9 0%, #a855f7 100%);
--gradient-mesh: radial-gradient(at 0% 0%, #0ea5e9 0%, transparent 50%),
                 radial-gradient(at 100% 100%, #a855f7 0%, transparent 50%),
                 radial-gradient(at 50% 50%, #06b6d4 0%, transparent 50%);
```

#### Typography Scale
```css
/* Hero Typography */
--text-hero-primary: 3.5rem;    /* 56px, line-height: 1.1 */
--text-hero-secondary: 2.5rem;  /* 40px, line-height: 1.2 */
--text-lead: 1.25rem;           /* 20px, line-height: 1.6 */

/* Font Weights */
--font-bold: 700;
--font-semibold: 600;
--font-normal: 400;
```

#### Spacing & Layout
```css
/* Container */
--container-max: 1280px;
--section-padding-y: 4rem;
--section-padding-x: 1.5rem;

/* Card Spacing */
--card-padding: 1.5rem;
--card-gap: 1rem;
--card-border-radius: 1rem;
```

### 3.2 Component Specifications

#### A. Animated Network Background

**Visual Description**:
- 20-30 nodes representing knowledge concepts
- Nodes: Circular, 8-16px diameter, semi-transparent with glow
- Connection lines: 1-2px, gradient colored, animated flow
- Movement: Slow drift (20-40s per full traversal)

**Implementation**:
```html
<!-- SVG-based, inline in HTML -->
<svg class="network-bg" viewBox="0 0 1920 1080">
  <!-- Nodes with animated positions -->
  <!-- Lines with animated stroke-dashoffset -->
</svg>
```

**Animation Specs**:
- Node float: `@keyframes float` - 30-60s duration, ease-in-out
- Line pulse: `@keyframes pulse` - 3-5s duration, infinite
- Opacity fade: 0.3 to 0.8
- Transform: translate3d for GPU acceleration

**Performance Target**: < 5% CPU usage, 60fps

#### B. Glassmorphic Feature Cards

**Visual Description**:
- Background: `rgba(255, 255, 255, 0.1)` (dark) / `rgba(0, 0, 0, 0.05)` (light)
- Backdrop filter: `blur(12px) saturate(180%)`
- Border: 1px solid `rgba(255, 255, 255, 0.2)`
- Shadow: `0 8px 32px 0 rgba(0, 0, 0, 0.37)`
- Border radius: 16px

**Hover State**:
- Transform: `translateY(-8px) scale(1.02)`
- Shadow: Enhanced to `0 16px 48px 0 rgba(0, 0, 0, 0.45)`
- Border glow: Add colored shadow matching accent
- Transition: `all 0.3s cubic-bezier(0.4, 0, 0.2, 1)`

**3D Tilt Effect**:
```css
/* On hover, card tilts toward cursor */
transform: perspective(1000px) rotateX(${y}deg) rotateY(${x}deg) translateY(-8px);
```

#### C. Animated Gradient Mesh Background

**Visual Description**:
- Base: Dark gradient (#0f172a → #1e293b)
- Overlay: 3-4 radial gradients (blue, purple, cyan)
- Animation: Gradients slowly move/morph (60-120s cycle)

**Implementation**:
```css
@keyframes mesh-morph {
  0%, 100% { /* Position 1 */ }
  33% { /* Position 2 */ }
  66% { /* Position 3 */ }
}
```

**Performance**: Use `will-change: transform` sparingly, prefer opacity changes

#### D. Enhanced Typography

**Hero Text Treatment**:
- Primary headline: Gradient clip, animated shimmer
- Secondary headline: Solid with subtle glow
- Lead paragraph: Increased line-height (1.7), max-width for readability

**Gradient Animation**:
```css
@keyframes gradient-shift {
  0%, 100% { background-position: 0% 50%; }
  50% { background-position: 100% 50%; }
}
/* background-size: 200% 200% for shift effect */
```

#### E. Interactive Buttons

**Primary CTA** (Get Started):
- Background: Solid gradient (#0ea5e9 → #06b6d4)
- Hover: Brightness increase + scale(1.05)
- Active: Ripple effect from click point
- Shadow: Colored glow matching gradient

**Secondary CTA** (Browse Tools):
- Border: 2px gradient border
- Background: Transparent → subtle gradient on hover
- Hover: Fill with low-opacity gradient

**Tertiary CTA** (Documentation):
- Border: 2px neutral
- Hover: Background color shift

---

## 4. Scope Definition

### 4.1 In Scope (What We WILL Do)

#### Visual Enhancements
- ✅ **Animated SVG network background** with floating nodes and connection lines
- ✅ **Glassmorphic feature cards** with backdrop blur and transparency effects
- ✅ **Animated gradient mesh background** that morphs and shifts colors
- ✅ **Enhanced typography** with gradient text and shimmer animations
- ✅ **3D card hover effects** using CSS transforms and perspective
- ✅ **Smooth transitions** across all interactive elements (300-500ms)
- ✅ **Ripple effects** on primary CTA buttons
- ✅ **Responsive design** optimized for mobile, tablet, and desktop
- ✅ **Dark mode support** (already exists, maintain and enhance)

#### Technical Implementation
- ✅ **Pure CSS animations** using @keyframes and transforms
- ✅ **Inline SVG** for critical network visualization
- ✅ **Tailwind 4 utilities** with custom CSS where needed
- ✅ **GPU-accelerated animations** using transform and opacity
- ✅ **Performance optimization** targeting 60fps and <2s load time
- ✅ **Accessibility compliance** meeting WCAG 2.1 Level AA
- ✅ **Cross-browser compatibility** for modern browsers (Chrome, Firefox, Safari, Edge)
- ✅ **Static export compatibility** for GitHub Pages hosting

#### Testing & Quality Assurance
- ✅ **Visual regression testing** across breakpoints
- ✅ **Performance profiling** using Lighthouse and WebPageTest
- ✅ **Accessibility auditing** with axe DevTools and manual testing
- ✅ **Cross-browser testing** on major browsers
- ✅ **Mobile device testing** on real devices
- ✅ **Load time testing** on throttled connections (3G simulation)

### 4.2 Out of Scope (What We WON'T Do)

#### Excluded Technologies
- ❌ **Three.js / WebGL** - Too heavy for static hosting, requires fallbacks, adds complexity
- ❌ **Canvas-based animations** - Accessibility concerns, performance unpredictable
- ❌ **JavaScript animation libraries** (GSAP, Anime.js, Framer Motion) - Unnecessary dependencies
- ❌ **External animation files** - Increases load time, deployment complexity
- ❌ **Video backgrounds** - Large file sizes, accessibility issues, performance concerns
- ❌ **Complex particle systems** - Performance intensive, not static-friendly

#### Excluded Features
- ❌ **Interactive node clicking** - Defer to V2, requires state management
- ❌ **User-customizable themes** - Beyond current scope, defer to V2
- ❌ **A/B testing framework** - Marketing concern, not design
- ❌ **Analytics tracking enhancements** - Separate implementation
- ❌ **Backend integration** - Static site only
- ❌ **Page transitions between routes** - Defer to V2 (View Transitions API)
- ❌ **Sound effects** - Accessibility concern, unnecessary
- ❌ **Mouse trail effects** - Distracting, not aligned with professional aesthetic

#### Excluded Browser Support
- ❌ **Internet Explorer 11** - End of life, not supported
- ❌ **Legacy browsers** (Chrome < 120, Firefox < 120, Safari < 17)
- ❌ **Opera Mini** - Limited CSS support
- ❌ **UC Browser** - Inconsistent rendering

#### Excluded Platforms
- ❌ **Server-side rendering** beyond Next.js static export
- ❌ **Non-static deployment targets** (Vercel, Netlify dynamic features)
- ❌ **Progressive Web App (PWA)** features - Defer to future iteration

### 4.3 Scope Boundaries

**What happens if scope creep occurs**:
1. Evaluate impact on timeline and performance budget
2. Document as "future enhancement" if not critical
3. Seek stakeholder approval for scope changes
4. Update PRD version and track changes

**Definition of Done**:
A feature is complete when:
1. Implementation matches design specifications
2. All acceptance criteria met
3. Performance benchmarks achieved
4. Accessibility requirements validated
5. Cross-browser testing passed
6. Code reviewed and merged
7. Deployed successfully to GitHub Pages

---

## 5. Technical Requirements

### 5.1 Technology Stack

**Constraints** (GitHub Pages Static Hosting):
- ✅ No server-side rendering required
- ✅ No build-time dependencies beyond Next.js static export
- ✅ Pure CSS animations (no JavaScript animation libraries)
- ✅ Inline SVG (no external SVG files for critical path)
- ✅ Modern CSS features (with graceful degradation)

**Approved Technologies**:
- Next.js 16 (static export)
- Tailwind CSS 4 (utility classes + custom CSS)
- Inline SVG for network visualization
- CSS3 animations and transforms
- CSS Grid and Flexbox for layout

**Prohibited**:
- Three.js or WebGL (too heavy, fallback needed)
- JavaScript animation libraries (GSAP, Anime.js)
- External animation dependencies
- Canvas-based animations (accessibility concerns)

### 5.2 Browser Support

**Target Browsers**:
- Chrome/Edge 120+ (chromium)
- Firefox 120+
- Safari 17+

**Graceful Degradation**:
- Backdrop-filter fallback: solid background
- CSS Grid fallback: Flexbox
- Advanced animations: Skip on `prefers-reduced-motion`

### 5.3 Performance Requirements

**Load Performance**:
- First Contentful Paint (FCP): < 1.5s
- Largest Contentful Paint (LCP): < 2.5s
- Cumulative Layout Shift (CLS): < 0.1
- Total Blocking Time (TBT): < 200ms

**Runtime Performance**:
- 60fps animation target
- CPU usage < 10% during idle animations
- Memory usage < 50MB increase
- No layout thrashing

**Optimization Techniques**:
- Use `transform` and `opacity` for animations (GPU accelerated)
- `will-change` on actively animating elements only
- Lazy load non-critical assets
- Inline critical CSS
- Debounce/throttle scroll/mouse events if needed

### 5.4 Accessibility Requirements

**WCAG 2.1 Level AA Compliance**:
- ✅ Color contrast ratios: 4.5:1 for text, 3:1 for UI components
- ✅ Keyboard navigation for all interactive elements
- ✅ Focus indicators clearly visible
- ✅ `prefers-reduced-motion` respected
- ✅ Semantic HTML structure
- ✅ ARIA labels where appropriate
- ✅ Alt text for decorative SVGs (`role="presentation"`)

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

### 5.5 Responsive Design

**Breakpoints** (Tailwind defaults):
- Mobile: 320px - 639px (sm)
- Tablet: 640px - 1023px (md, lg)
- Desktop: 1024px+ (xl, 2xl)

**Component Behavior**:
- Network background: Scales to viewport, density reduces on mobile
- Feature cards: Stack vertically on mobile (1 col), 2 cols tablet, 4 cols desktop
- Typography: Scales down on mobile (3rem → 2.5rem hero)
- Animations: Reduced complexity on mobile for performance

---

## 5. Implementation Plan

### 5.1 File Structure

```
site/
├── app/
│   ├── page.tsx                    # Main landing page (MODIFY)
│   ├── globals.css                 # Global styles (MODIFY)
│   └── components/
│       ├── NetworkBackground.tsx   # New animated network
│       ├── GlassCard.tsx           # New glassmorphic card
│       └── AnimatedText.tsx        # New gradient text component
├── public/
│   └── assets/
│       └── (existing assets)
└── tailwind.config.ts              # May need custom utilities
```

### 5.2 Implementation Phases

#### Phase 1: Foundation (Core Structure)

**Tasks**:
1. Create `NetworkBackground.tsx` component
   - Generate SVG node structure
   - Implement basic CSS animations
   - Test performance
2. Update color system in Tailwind config
3. Create custom CSS for gradients and effects
4. Set up animation keyframes

**Deliverable**: Working animated network background

**Validation**:
- [ ] Network renders on all breakpoints
- [ ] Animations run at 60fps
- [ ] No console errors
- [ ] prefers-reduced-motion works

#### Phase 2: Glassmorphism & Cards

**Tasks**:
1. Create `GlassCard.tsx` component
2. Implement backdrop-filter with fallbacks
3. Add hover states and 3D transforms
4. Apply to feature cards
5. Test hover interactions

**Deliverable**: Glassmorphic feature cards with hover effects

**Validation**:
- [ ] Cards render with glass effect
- [ ] Hover animations smooth
- [ ] Fallback works without backdrop-filter
- [ ] Touch interactions work on mobile

#### Phase 3: Typography & Gradients

**Tasks**:
1. Create `AnimatedText.tsx` for gradient text
2. Implement gradient mesh background
3. Update hero typography scale
4. Add shimmer animations
5. Test readability

**Deliverable**: Enhanced typography with animated gradients

**Validation**:
- [ ] Text is readable on all backgrounds
- [ ] Gradient animations perform well
- [ ] Contrast ratios meet WCAG AA
- [ ] Mobile typography scales properly

#### Phase 4: Micro-interactions & Polish

**Tasks**:
1. Enhance button hover states
2. Add ripple effects to CTAs
3. Implement smooth page transitions
4. Add loading state animation
5. Fine-tune timing and easing

**Deliverable**: Polished interactions throughout

**Validation**:
- [ ] All interactive elements have feedback
- [ ] Transitions feel smooth and intentional
- [ ] Loading states are clear
- [ ] No janky animations

#### Phase 5: Testing & Optimization

**Tasks**:
1. Cross-browser testing
2. Performance profiling (Lighthouse)
3. Accessibility audit
4. Mobile device testing
5. Optimization pass

**Deliverable**: Production-ready landing page

**Validation**:
- [ ] Lighthouse score 95+
- [ ] Works in Chrome, Firefox, Safari, Edge
- [ ] WCAG AA compliant
- [ ] Mobile performance acceptable
- [ ] GitHub Pages deployment successful

---

## 6. Dependencies & Constraints

### Technical Dependencies
- Next.js 16.0.0 (already in place)
- Tailwind CSS 4.1.16 (already in place)
- Modern browser CSS features:
  - `backdrop-filter` (with fallback)
  - CSS Grid
  - CSS animations
  - `clip-path`
  - CSS custom properties

### Constraints
1. **GitHub Pages Hosting**: No server-side rendering, all static
2. **Performance Budget**: Page weight < 500KB (excluding fonts)
3. **Load Time**: Must load in < 3s on 3G connection
4. **Browser Support**: Only modern browsers (no IE11)
5. **Accessibility**: Must maintain WCAG AA compliance

### Risk Mitigation

**Risk**: Animations cause poor performance on low-end devices
**Mitigation**:
- Use GPU-accelerated properties only
- Implement performance monitoring
- Provide reduced-motion alternative
- Test on low-end devices (throttled)

**Risk**: Glassmorphism not supported in older browsers
**Mitigation**:
- Solid background fallback
- Progressive enhancement approach
- Feature detection

**Risk**: SVG network too complex, slows page load
**Mitigation**:
- Inline critical SVG
- Optimize node count (20-30 max)
- Use CSS animations instead of SMIL
- Lazy load non-critical elements

---

## 7. Comprehensive Testing Strategy

### 7.1 Testing Philosophy

**Testing Approach**: Multi-layered validation ensuring quality, performance, and accessibility at every stage of development.

**Testing Types**:
1. **Unit Testing**: Individual component validation
2. **Visual Testing**: Cross-browser and responsive design validation
3. **Performance Testing**: Load time, runtime performance, and optimization
4. **Accessibility Testing**: WCAG compliance and assistive technology compatibility
5. **Integration Testing**: Component interaction and system behavior
6. **User Acceptance Testing**: Stakeholder validation

**Test-Driven Development**: Tests are written alongside features, not after. Each component has defined acceptance criteria before implementation begins.

---

### 7.2 Unit Testing: Component-Level Validation

#### 7.2.1 NetworkBackground Component

**Test Cases**:

| Test ID | Test Description | Expected Result | Priority |
|---------|-----------------|-----------------|----------|
| NET-001 | SVG renders with correct viewBox | SVG element present with viewBox="0 0 1920 1080" | P0 |
| NET-002 | Minimum 20 nodes generated | >= 20 circle elements in SVG | P0 |
| NET-003 | Connection lines exist | >= 15 line/path elements connecting nodes | P0 |
| NET-004 | Animations apply to nodes | All nodes have animation CSS class | P0 |
| NET-005 | Performance: CPU usage < 5% | Monitor task manager during idle animation | P1 |
| NET-006 | Performance: 60fps maintained | Chrome DevTools FPS counter shows 60fps | P0 |
| NET-007 | Reduced motion respected | Animations pause/skip with prefers-reduced-motion | P0 |
| NET-008 | Mobile: Node density reduced | < 15 nodes on viewports < 640px | P1 |

**Test Procedure**:
```bash
# Visual inspection
1. Load page in browser
2. Open DevTools -> Elements
3. Inspect SVG structure
4. Verify node count and connections
5. Check animation classes applied

# Performance testing
1. Open DevTools -> Performance
2. Start recording
3. Let page idle for 30 seconds
4. Stop recording
5. Verify: FPS avg >= 58, CPU avg < 5%
```

**Acceptance Criteria**:
- [ ] All P0 tests pass
- [ ] All P1 tests pass
- [ ] No console errors
- [ ] Visual matches design mockup

---

#### 7.2.2 GlassCard Component

**Test Cases**:

| Test ID | Test Description | Expected Result | Priority |
|---------|-----------------|-----------------|----------|
| GLS-001 | Backdrop blur applied | CSS backdrop-filter: blur(12px) present | P0 |
| GLS-002 | Fallback for no backdrop-filter support | Solid background color applied when not supported | P0 |
| GLS-003 | Border glow visible | Box-shadow with color present | P1 |
| GLS-004 | Hover state transform | Transform: translateY(-8px) on hover | P0 |
| GLS-005 | 3D tilt effect (desktop) | Perspective transform applied on mousemove | P1 |
| GLS-006 | Touch devices: No tilt | Tilt effect disabled on touch devices | P1 |
| GLS-007 | Transition smoothness | Transition duration 300ms, no jank | P0 |
| GLS-008 | Responsive: Mobile stack | Cards stack vertically on < 640px | P0 |

**Test Procedure**:
```javascript
// Browser console test
const card = document.querySelector('.glass-card');
const styles = window.getComputedStyle(card);

// Check backdrop filter
console.log('Backdrop filter:', styles.backdropFilter); // Should include "blur"

// Check hover (trigger hover state)
card.dispatchEvent(new MouseEvent('mouseenter'));
setTimeout(() => {
  const hoverStyles = window.getComputedStyle(card);
  console.log('Transform on hover:', hoverStyles.transform);
}, 100);
```

**Acceptance Criteria**:
- [ ] Glass effect visible in supported browsers
- [ ] Fallback works in unsupported browsers
- [ ] Hover animations smooth and performant
- [ ] Touch interactions work on mobile

---

#### 7.2.3 AnimatedText Component

**Test Cases**:

| Test ID | Test Description | Expected Result | Priority |
|---------|-----------------|-----------------|----------|
| TXT-001 | Gradient applied to text | background-clip: text, gradient visible | P0 |
| TXT-002 | Gradient animation runs | Background-position changes over time | P1 |
| TXT-003 | Text remains readable | Contrast ratio >= 4.5:1 at all animation states | P0 |
| TXT-004 | Shimmer effect subtle | Animation doesn't distract from content | P1 |
| TXT-005 | Mobile: Typography scales | Font size reduces appropriately on mobile | P0 |
| TXT-006 | Fallback: No gradient support | Solid color text displayed | P1 |

**Acceptance Criteria**:
- [ ] Gradient text visible and readable
- [ ] Animation enhances, doesn't distract
- [ ] Passes contrast ratio checks

---

### 7.3 Visual Regression Testing

#### 7.3.1 Cross-Browser Visual Testing

**Browsers to Test**:
- Chrome 120+ (macOS, Windows, Android)
- Firefox 120+ (macOS, Windows)
- Safari 17+ (macOS, iOS)
- Edge 120+ (Windows)

**Test Procedure**:
1. Load page in each browser
2. Take full-page screenshots at each breakpoint
3. Compare against baseline screenshots
4. Document any visual differences
5. Validate differences are acceptable or require fixes

**Screenshot Breakpoints**:
- Mobile: 375px x 667px (iPhone SE)
- Tablet: 768px x 1024px (iPad)
- Desktop: 1920px x 1080px (Full HD)
- Large Desktop: 2560px x 1440px (2K)

**Checklist per Browser**:

| Visual Element | Chrome | Firefox | Safari | Edge | Status |
|----------------|--------|---------|--------|------|--------|
| Network background renders | [ ] | [ ] | [ ] | [ ] | Pending |
| Network animations smooth | [ ] | [ ] | [ ] | [ ] | Pending |
| Glass effect visible | [ ] | [ ] | [ ] | [ ] | Pending |
| Card hover effects work | [ ] | [ ] | [ ] | [ ] | Pending |
| Typography gradients visible | [ ] | [ ] | [ ] | [ ] | Pending |
| Gradient animations run | [ ] | [ ] | [ ] | [ ] | Pending |
| Buttons render correctly | [ ] | [ ] | [ ] | [ ] | Pending |
| Layout responsive | [ ] | [ ] | [ ] | [ ] | Pending |
| Dark mode works | [ ] | [ ] | [ ] | [ ] | Pending |

---

#### 7.3.2 Responsive Design Testing

**Test Cases by Breakpoint**:

**Mobile (375px)**:
- [ ] Network background simplified (fewer nodes)
- [ ] Cards stack vertically (1 column)
- [ ] Hero text scales down to 2.5rem
- [ ] Buttons stack or go full-width
- [ ] No horizontal scroll
- [ ] Touch targets >= 44x44px
- [ ] Animations remain performant

**Tablet (768px)**:
- [ ] Network background at medium density
- [ ] Cards in 2 columns
- [ ] Hero text at 3rem
- [ ] Buttons in row
- [ ] Layout balanced
- [ ] Hover states work (if not touch-only)

**Desktop (1920px)**:
- [ ] Network background at full density
- [ ] Cards in 4 columns
- [ ] Hero text at 3.5rem (56px)
- [ ] All hover effects active
- [ ] Layout centered, max-width applied
- [ ] Animations at full fidelity

**Test Procedure**:
```bash
# Chrome DevTools Device Emulation
1. Open DevTools (F12)
2. Toggle device toolbar (Cmd+Shift+M)
3. Select device: iPhone SE, iPad, Responsive
4. Test each breakpoint
5. Take screenshots
6. Validate against checklist
```

---

### 7.4 Performance Testing

#### 7.4.1 Load Performance

**Tools**:
- Google Lighthouse (Chrome DevTools)
- WebPageTest (webpagetest.org)
- Chrome DevTools Network tab

**Test Procedure**:

**Lighthouse Audit**:
```bash
# Run from command line
npx lighthouse http://localhost:3000 \
  --only-categories=performance,accessibility \
  --output=html \
  --output-path=./lighthouse-report.html \
  --chrome-flags="--headless"

# Target Scores
Performance: >= 95
Accessibility: >= 100
Best Practices: >= 95
```

**WebPageTest**:
1. Go to webpagetest.org
2. Enter URL: https://www.maenifold.com
3. Select location: Multiple (US, EU, Asia)
4. Select connection: 3G, 4G, Cable
5. Run test
6. Validate metrics

**Metrics to Track**:

| Metric | Target | Measurement Method | Priority |
|--------|--------|-------------------|----------|
| First Contentful Paint (FCP) | < 1.5s | Lighthouse | P0 |
| Largest Contentful Paint (LCP) | < 2.5s | Lighthouse | P0 |
| Cumulative Layout Shift (CLS) | < 0.1 | Lighthouse | P0 |
| Total Blocking Time (TBT) | < 200ms | Lighthouse | P0 |
| Time to Interactive (TTI) | < 3.5s | Lighthouse | P1 |
| Total Page Size | < 500KB | Network tab | P1 |
| Number of Requests | < 20 | Network tab | P1 |

**3G Testing** (Slow connection simulation):
```bash
# Chrome DevTools -> Network tab
1. Open DevTools
2. Go to Network tab
3. Select throttling: "Slow 3G"
4. Reload page (hard refresh)
5. Measure load time
6. Target: Page usable in < 5s
```

---

#### 7.4.2 Runtime Performance

**Animation Performance Testing**:

**Test Procedure**:
```bash
# Chrome DevTools Performance Profiling
1. Open DevTools -> Performance tab
2. Click "Record" (Cmd+E)
3. Let page idle for 30 seconds (animations running)
4. Stop recording
5. Analyze:
   - FPS: Should be solid 60fps (green line)
   - CPU: Average < 10%, peaks < 25%
   - Memory: No leaks (stable or declining)
   - Long tasks: None > 50ms
```

**Metrics**:

| Metric | Target | Failure Threshold | Priority |
|--------|--------|-------------------|----------|
| Average FPS | 60fps | < 58fps | P0 |
| FPS drops | None | > 3 drops below 50fps | P0 |
| CPU usage (idle) | < 5% | > 15% | P0 |
| CPU usage (hover) | < 10% | > 25% | P1 |
| Memory usage increase | < 50MB | > 100MB | P1 |
| JavaScript execution time | < 5ms/frame | > 10ms/frame | P0 |

**Scroll Performance**:
```bash
1. Open DevTools -> Rendering
2. Enable "Frame Rendering Stats"
3. Scroll page up and down rapidly
4. Monitor FPS counter
5. Target: 60fps maintained during scroll
```

---

### 7.5 Accessibility Testing

#### 7.5.1 Automated Accessibility Audit

**Tools**:
- axe DevTools (Chrome extension)
- Lighthouse Accessibility audit
- WAVE browser extension

**Test Procedure**:

**axe DevTools**:
```bash
1. Install axe DevTools extension
2. Load page
3. Open DevTools -> axe DevTools tab
4. Click "Scan ALL of my page"
5. Review issues:
   - Critical: MUST fix (0 allowed)
   - Serious: MUST fix (0 allowed)
   - Moderate: Should fix
   - Minor: Nice to fix
```

**Expected Results**:
- [ ] 0 Critical issues
- [ ] 0 Serious issues
- [ ] < 5 Moderate issues (with documented exceptions)
- [ ] Lighthouse Accessibility score: 100

---

#### 7.5.2 Manual Accessibility Testing

**Keyboard Navigation Test**:

**Test Cases**:

| Test ID | Action | Expected Result | Status |
|---------|--------|-----------------|--------|
| KBD-001 | Press Tab from page load | Focus moves to first interactive element (skip to content or logo) | [ ] |
| KBD-002 | Tab through all interactive elements | Logical focus order, all elements reachable | [ ] |
| KBD-003 | Focus indicators visible | Clear visual indicator on all focused elements | [ ] |
| KBD-004 | Press Enter on CTA button | Navigation occurs | [ ] |
| KBD-005 | Press Escape on modal (if any) | Modal closes | N/A |
| KBD-006 | No keyboard traps | Can Tab forward and Shift+Tab backward freely | [ ] |

**Test Procedure**:
1. Load page
2. Close mouse/trackpad (use only keyboard)
3. Tab through entire page
4. Verify every interactive element is reachable
5. Verify focus indicators are visible
6. Verify no focus traps exist

---

**Screen Reader Testing**:

**Screen Readers to Test**:
- VoiceOver (macOS/iOS)
- NVDA (Windows)
- JAWS (Windows) - if available

**Test Procedure (VoiceOver example)**:
```bash
# macOS
1. Enable VoiceOver (Cmd+F5)
2. Navigate page with VO+Right Arrow
3. Listen to announcements
4. Verify:
   - Page title announced
   - Headings announced with level
   - Links announced with purpose
   - Images have alt text or role="presentation"
   - Form fields have labels
   - Status messages announced
```

**Test Cases**:

| Element | Expected Announcement | Status |
|---------|----------------------|--------|
| Page title | "Maenifold" | [ ] |
| Hero heading | "Your agent is ephemeral" heading level 1 | [ ] |
| Feature cards | Card content readable, heading level 3 | [ ] |
| CTA buttons | "Get Started, button" with clear purpose | [ ] |
| Network background | Ignored (decorative, role="presentation") | [ ] |
| Navigation links | Link purpose clear from text | [ ] |

---

**Color Contrast Testing**:

**Test Procedure**:
```bash
# Using browser extension (e.g., WAVE)
1. Install WAVE extension
2. Load page
3. Click WAVE icon
4. Check "Contrast" tab
5. Verify all text meets WCAG AA: 4.5:1 (normal), 3:1 (large)

# Manual check
1. Use Contrast Checker (webaim.org/resources/contrastchecker/)
2. Sample foreground and background colors
3. Verify ratios:
   - Normal text: >= 4.5:1
   - Large text (18px+ or 14px+ bold): >= 3:1
   - UI components: >= 3:1
```

**Critical Color Pairs to Test**:

| Foreground | Background | Minimum Ratio | Actual Ratio | Status |
|------------|------------|---------------|--------------|--------|
| Hero text (white) | Dark background (#0f172a) | 4.5:1 | TBD | [ ] |
| Card text (slate-600) | Glass card background | 4.5:1 | TBD | [ ] |
| Button text (white) | Button background (blue-600) | 4.5:1 | TBD | [ ] |
| Link text | Background | 4.5:1 | TBD | [ ] |

---

**Reduced Motion Testing**:

**Test Procedure**:
```bash
# macOS
System Settings -> Accessibility -> Display -> Reduce Motion: ON

# Windows
Settings -> Accessibility -> Visual Effects -> Animation Effects: OFF

# Chrome DevTools emulation
DevTools -> Rendering -> Emulate CSS media feature prefers-reduced-motion: reduce
```

**Test Cases**:
- [ ] All animations pause or run at minimal duration (<0.01s)
- [ ] Page remains functional without animations
- [ ] No dizzying motion effects
- [ ] Critical content still visible

---

### 7.6 Integration Testing

**Page Load Sequence**:

**Test Cases**:

| Test ID | Test Description | Expected Result | Status |
|---------|-----------------|-----------------|--------|
| INT-001 | Network background loads before content | Background visible, then content fades in | [ ] |
| INT-002 | No layout shift during load | CLS score < 0.1 | [ ] |
| INT-003 | Animations start after page interactive | Animations don't block interactivity | [ ] |
| INT-004 | Dark mode toggle works | Theme switches, animations continue | [ ] |
| INT-005 | Navigation links work | Clicking links navigates to correct pages | [ ] |
| INT-006 | CTAs work | Buttons navigate to correct destinations | [ ] |

---

### 7.7 User Acceptance Testing (UAT)

**Stakeholder Review Checklist**:

**Visual Quality** (Subjective):
- [ ] "Wow" factor present - stops you in your tracks
- [ ] Embodies knowledge graph concept visually
- [ ] Feels modern and premium (award-worthy)
- [ ] Animations enhance, don't distract
- [ ] Colors evoke intelligence and depth
- [ ] Typography is impactful and readable

**Functional Quality**:
- [ ] All CTAs clear and clickable
- [ ] Navigation intuitive
- [ ] Mobile experience excellent
- [ ] Dark mode looks great
- [ ] Loads quickly
- [ ] Feels responsive and smooth

**Brand Alignment**:
- [ ] Represents Maenifold's core mission
- [ ] Communicates "living knowledge graph"
- [ ] Professional and trustworthy
- [ ] Distinct from competitors

---

### 7.8 Testing Timeline & Milestones

**Phase 1: Unit Testing** (During Development)
- Test each component as it's built
- Validate acceptance criteria before moving to next component

**Phase 2: Visual Regression** (After Component Completion)
- Cross-browser testing on major browsers
- Responsive testing on key breakpoints
- Screenshot comparison

**Phase 3: Performance Testing** (Before Deployment)
- Lighthouse audits
- WebPageTest validation
- Runtime profiling
- Optimization if needed

**Phase 4: Accessibility Testing** (Before Deployment)
- Automated tools (axe, WAVE)
- Keyboard navigation
- Screen reader testing
- Color contrast validation

**Phase 5: UAT** (Pre-Production)
- Stakeholder review
- Feedback incorporation
- Final sign-off

**Phase 6: Production Validation** (Post-Deployment)
- Verify on live URL
- Real device testing
- Monitor analytics
- Address any issues

---

### 7.9 Test Reporting

**Test Report Template**:

```markdown
# Landing Page Redesign - Test Report
**Date**: [Date]
**Tester**: [Name]
**Environment**: [Browser, OS, Device]

## Summary
- Total Tests: X
- Passed: X
- Failed: X
- Blocked: X
- Pass Rate: X%

## Failed Tests
[List of failed tests with details]

## Blockers
[Any issues preventing testing]

## Recommendations
[Suggested fixes or improvements]

## Screenshots
[Attach relevant screenshots]

## Sign-off
- [ ] Ready for production
- [ ] Needs fixes
```

---

### 7.10 Definition of Testing Complete

Testing is complete when:
1. ✅ All P0 unit tests pass
2. ✅ All P1 unit tests pass
3. ✅ Cross-browser visual tests pass
4. ✅ Responsive design validates across breakpoints
5. ✅ Lighthouse Performance >= 95
6. ✅ Lighthouse Accessibility = 100
7. ✅ 0 critical/serious accessibility issues
8. ✅ Keyboard navigation 100% functional
9. ✅ Screen reader testing passed
10. ✅ Color contrast meets WCAG AA
11. ✅ Reduced motion implementation verified
12. ✅ Integration tests pass
13. ✅ UAT sign-off received
14. ✅ Production deployment validated

---

## 8. Visual Testing Checklist

**Desktop (1920x1080)**:
- [ ] Network background animates smoothly
- [ ] Cards have glass effect with blur
- [ ] Typography gradients visible
- [ ] Hover states work on all elements
- [ ] Layout centered and balanced

**Tablet (768x1024)**:
- [ ] Cards reflow to 2 columns
- [ ] Typography scales appropriately
- [ ] Touch targets minimum 44x44px
- [ ] Network remains performant

**Mobile (375x667)**:
- [ ] Cards stack vertically
- [ ] Hero text readable
- [ ] Buttons full-width or stacked
- [ ] Network simplified
- [ ] No horizontal scroll

### 7.2 Performance Testing

**Tools**:
- Chrome DevTools Performance tab
- Lighthouse CI
- WebPageTest
- Real device testing

**Benchmarks**:
```
Lighthouse Score Targets:
- Performance: 95+
- Accessibility: 100
- Best Practices: 95+
- SEO: 100

Core Web Vitals:
- LCP: < 2.5s
- FID: < 100ms
- CLS: < 0.1
```

### 7.3 Browser Testing Matrix

| Browser | Version | Desktop | Mobile | Status |
|---------|---------|---------|--------|--------|
| Chrome  | 120+    | ✅      | ✅     | Pass   |
| Firefox | 120+    | ✅      | ✅     | Pass   |
| Safari  | 17+     | ✅      | ✅     | Pass   |
| Edge    | 120+    | ✅      | ✅     | Pass   |

### 7.4 Accessibility Testing

**Tools**:
- axe DevTools
- WAVE browser extension
- Keyboard navigation manual test
- Screen reader test (VoiceOver/NVDA)

**Checklist**:
- [ ] All interactive elements keyboard accessible
- [ ] Focus indicators visible
- [ ] Color contrast meets WCAG AA
- [ ] Animations respect prefers-reduced-motion
- [ ] Semantic HTML structure
- [ ] ARIA labels where needed
- [ ] Alt text for all images
- [ ] No keyboard traps

---

## 8. Success Criteria

### Must Have (MVP)
- ✅ Animated network background visible and performant
- ✅ Glassmorphic cards with hover effects
- ✅ Enhanced typography with gradients
- ✅ Smooth transitions and animations
- ✅ 60fps performance on desktop
- ✅ Mobile responsive
- ✅ WCAG AA compliant
- ✅ Works on GitHub Pages

### Should Have
- ✅ 3D card tilt effects on hover
- ✅ Ripple effects on buttons
- ✅ Gradient mesh background animation
- ✅ Page load animation sequence
- ✅ Parallax scroll effects (subtle)

### Nice to Have
- ⭕ Mouse-reactive network (nodes respond to cursor)
- ⭕ Dark/light mode toggle with transition
- ⭕ Easter egg interactions
- ⭕ Custom cursor effect

---

## 9. Future Enhancements (Post-MVP)

### V2 Considerations
1. **Interactive Network**: Click nodes to reveal related concepts
2. **3D Manifold Visualization**: WebGL-based (with fallback)
3. **Concept Explorer**: Hover over features to see knowledge graph connections
4. **Animated Transitions**: Between pages using View Transitions API
5. **Performance Modes**: User can toggle "simple" vs "full" animations

---

## 10. Appendix

### A. Design Inspiration Sources

**Network Visualizations**:
- Nayuki.io - Animated Floating Graph Nodes
- D3.js Force Layout examples
- Observable HQ graph visualizations

**UI Trends**:
- Glassmorphism examples from Dribbble
- Awwwards 2024-2025 winners
- CSS Design Awards featured sites

**Mathematical Inspiration**:
- Calabi-Yau manifold visualizations
- Network topology diagrams
- Knowledge graph representations

### B. Code Standards

**CSS**:
- Use Tailwind utilities where possible
- Custom CSS in `globals.css` with clear comments
- BEM naming for custom classes
- Prefer CSS custom properties for themeable values

**TypeScript/React**:
- Functional components with TypeScript
- Props interfaces defined
- Accessibility props included (aria-labels, etc.)
- Comments for complex logic

**Performance**:
- No inline styles (except dynamic)
- Minimize re-renders
- Memoize expensive calculations
- Use React.memo for static components

### C. Stakeholder Sign-off

**Reviewed By**: [Pending]
**Approved By**: [Pending]
**Date**: [Pending]

---

## Document Control

**Version History**:
- v1.0 (2025-10-28): Initial PRD draft

**Related Documents**:
- Technical Architecture: `/docs/architecture.md`
- Brand Guidelines: [To be created]
- Previous Session Summary: [Conversation history]

**Contact**:
For questions or clarifications, refer to project repository issues.

---

**END OF DOCUMENT**
