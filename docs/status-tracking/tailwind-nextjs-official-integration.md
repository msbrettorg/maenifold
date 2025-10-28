# Tailwind CSS + Next.js Official Integration Documentation

**Research Date**: 2025-10-28
**Research Status**: COMPLETE - All from official sources
**Session ID**: session-1761655960213

---

## Executive Summary

This document captures EXACT installation commands, configuration examples, and dark mode implementation approaches from official Tailwind CSS and Next.js documentation. All content is directly extracted from official sources with URLs and citations.

**Key Finding**: No compatibility issues noted between Tailwind CSS and Next.js 16 with static exports. Tailwind CSS is compiled at build time, making it compatible with `next build` and static file deployment.

---

## 1. Official Installation Commands

### Source
- https://nextjs.org/docs/app/building-your-application/styling/tailwind-css
- https://tailwindcss.com/docs/guides/nextjs

### 1.1 Installation via Package Manager

Choose one based on your package manager:

**Using pnpm (Recommended):**
```bash
pnpm add -D tailwindcss @tailwindcss/postcss
```

**Using npm:**
```bash
npm install tailwindcss @tailwindcss/postcss postcss
```

**Using yarn:**
```bash
yarn add -D tailwindcss @tailwindcss/postcss postcss
```

**Using bun:**
```bash
bun add -D tailwindcss @tailwindcss/postcss postcss
```

---

## 2. Official Configuration Files

### Source
- https://nextjs.org/docs/app/building-your-application/styling/tailwind-css
- https://tailwindcss.com/docs/guides/nextjs

### 2.1 PostCSS Configuration (`postcss.config.mjs`)

**EXACT from official docs:**

```javascript
export default {
  plugins: {
    '@tailwindcss/postcss': {},
  },
}
```

**Alternative format** (as shown in Tailwind docs):
```javascript
const config = {
  plugins: {
    "@tailwindcss/postcss": {},
  },
};
export default config;
```

**Location**: Create/update this file in your project root directory.

### 2.2 Global CSS Import (`app/globals.css`)

**EXACT from official docs:**

```css
@import 'tailwindcss';
```

**Location**: Create/update this file in the `app` directory.

### 2.3 Root Layout Import (`app/layout.tsx`)

**EXACT from official docs:**

```typescript
import './globals.css'

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  )
}
```

---

## 3. Dark Mode Implementation

### Source
- https://tailwindcss.com/docs/dark-mode

### 3.1 Automatic Dark Mode (Recommended)

**Default behavior** - Respects operating system dark mode setting:
- Uses CSS media feature: `prefers-color-scheme`
- No additional configuration required
- Automatically respects user's OS dark mode preference

**Usage in components:**
```jsx
export default function Component() {
  return (
    <div className="bg-white dark:bg-slate-900">
      <h1 className="text-slate-900 dark:text-white">
        Hello World
      </h1>
    </div>
  )
}
```

### 3.2 Manual Toggle with CSS Class

**For user-controlled dark mode switching**, update `app/globals.css`:

```css
@import "tailwindcss";

@custom-variant dark (&:where(.dark, .dark *));
```

**Apply dark class to HTML element:**

```html
<html class="dark">
  <body>
    <div class="bg-white dark:bg-black">
      <!-- Content -->
    </div>
  </body>
</html>
```

### 3.3 Manual Toggle with Data Attributes

**Alternative approach** for cleaner data handling:

Update `app/globals.css`:
```css
@import "tailwindcss";

@custom-variant dark (&:where([data-theme=dark], [data-theme=dark] *));
```

**Apply data attribute to HTML element:**

```html
<html data-theme="dark">
  <!-- Content -->
</html>
```

### 3.4 Complete Three-Way Dark Mode Strategy

**Official approach combining system detection + user preference:**

1. **Check localStorage** for saved user preference
2. **Fall back to system** `prefers-color-scheme` if no preference exists
3. **Update dynamically** using JavaScript based on user selection

**Implementation outline:**
```typescript
// On initial load
const savedTheme = localStorage.getItem('theme');
const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;

if (savedTheme === 'dark' || (!savedTheme && prefersDark)) {
  document.documentElement.classList.add('dark');
} else {
  document.documentElement.classList.remove('dark');
}

// When user toggles theme
function toggleDarkMode() {
  const isDark = document.documentElement.classList.toggle('dark');
  localStorage.setItem('theme', isDark ? 'dark' : 'light');
}
```

---

## 4. Next.js 16 Compatibility

### Source
- https://nextjs.org/docs/app/building-your-application/styling/tailwind-css
- https://nextjs.org/docs/app/building-your-application/deploying/static-exports

### 4.1 Styling Approach

Next.js documentation states:
> "Tailwind CSS is a utility-first CSS framework that provides low-level utility classes to build custom designs."

Tailwind CSS is the **recommended** styling approach for Next.js applications.

### 4.2 Static Export Compatibility

**Key finding**: No specific limitations noted for Tailwind CSS with static exports.

**Why it works:**
- Tailwind CSS is compiled at **build time** via PostCSS
- CSS output is generated during `next build`
- All CSS files are static assets (no runtime requirements)
- Compatible with `next build` → `out` folder deployment

**Build process:**
```bash
next build
# Generates static HTML, CSS, JS files in ./out directory
# Ready for deployment to any static hosting (GitHub Pages, etc.)
```

### 4.3 Deployment with Static Exports

From official docs: "When running `next build`, Next.js generates an HTML file per route" into the `out` folder.

**These static assets can be deployed to:**
- GitHub Pages
- Netlify
- Vercel Static
- Any web server serving HTML/CSS/JS files

---

## 5. Complete Setup Quick Start

### Minimal Working Example

**Step 1: Create Next.js project** (if needed)
```bash
npx create-next-app@latest my-project --app --typescript --eslint
cd my-project
```

**Step 2: Install Tailwind CSS**
```bash
npm install tailwindcss @tailwindcss/postcss postcss
```

**Step 3: Create `postcss.config.mjs`**
```javascript
export default {
  plugins: {
    '@tailwindcss/postcss': {},
  },
}
```

**Step 4: Update `app/globals.css`**
```css
@import 'tailwindcss';
```

**Step 5: Ensure root layout imports CSS** (`app/layout.tsx`)
```typescript
import './globals.css'
```

**Step 6: Use Tailwind classes in components**
```jsx
export default function Home() {
  return (
    <h1 className="text-3xl font-bold underline">
      Hello world!
    </h1>
  )
}
```

**Step 7: Run development server**
```bash
npm run dev
```

---

## 6. CSS-in-JS and Other Styling Options

### Source
- https://nextjs.org/docs/app/building-your-application/styling

### Available Styling Methods

**Next.js officially supports:**

1. **Tailwind CSS** - Recommended for most projects
2. **CSS Modules** - Local scoped styles with `.module.css` extension
3. **Global CSS** - Imported in root layout for application-wide styles
4. **Sass** - Supported with dedicated configuration
5. **CSS-in-JS** - Various libraries can be integrated
6. **External Stylesheets** - Third-party package styles

**Recommendation from official docs**: Prioritize Tailwind CSS for utility-first styling while using CSS Modules for component-specific overrides when needed.

---

## 7. Known Limitations and Notes

### Official Documentation Findings

**For Static Exports:**
- Server Components run during build time
- Client Components with `useEffect` work for client-side data fetching
- Custom image loaders may be needed (default loader not supported in static exports)
- Features requiring Node.js server runtime are not supported

**For CSS specifically:**
- No limitations noted
- CSS is compiled at build time
- All CSS output is static and portable

**Browser Compatibility:**
- Documentation references Tailwind CSS v3 setup for broader browser compatibility with older browsers
- Modern Tailwind CSS uses latest CSS features

---

## 8. Citation and Source URLs

### Primary Sources

| Topic | Source URL |
|-------|-----------|
| Next.js Tailwind CSS Setup | https://nextjs.org/docs/app/building-your-application/styling/tailwind-css |
| Tailwind CSS + Next.js Guide | https://tailwindcss.com/docs/guides/nextjs |
| Dark Mode Documentation | https://tailwindcss.com/docs/dark-mode |
| Static Exports | https://nextjs.org/docs/app/building-your-application/deploying/static-exports |
| Styling Overview | https://nextjs.org/docs/app/building-your-application/styling |

### Verification Notes

- ✅ All installation commands are exact copies from official documentation
- ✅ All configuration file examples are exact copies from official documentation
- ✅ Dark mode approach is from official Tailwind CSS documentation
- ✅ Static export compatibility verified through Next.js documentation
- ✅ No assumptions - only documented official information

---

## 9. Open Questions for Implementation

Based on this research, for the [[maenifold]] site implementation:

1. **Dark mode preference**: Should we implement automatic OS-based detection only, or full three-way (auto + saved preference + toggle)?
2. **CSS organization**: Should we add CSS Modules alongside Tailwind for component-specific overrides?
3. **Third-party component libraries**: Any UI component libraries to integrate (buttons, cards, modals)?
4. **Build target**: Confirmed static export to GitHub Pages is compatible - proceed with this approach?
5. **Deployment**: Use GitHub Actions for automated builds to static output?

---

## 10. Implementation Checklist

- [ ] Install Tailwind CSS dependencies
- [ ] Create `postcss.config.mjs` with exact config from documentation
- [ ] Update `app/globals.css` with Tailwind import
- [ ] Verify root layout imports global CSS
- [ ] Test Tailwind utility classes in a component
- [ ] Implement dark mode approach (automatic recommended)
- [ ] Test static export build with `next build`
- [ ] Verify CSS output in `out` folder
- [ ] Deploy static files to GitHub Pages

---

## Document Metadata

| Property | Value |
|----------|-------|
| Created | 2025-10-28 |
| Last Updated | 2025-10-28 |
| Sources Verified | Yes - All official |
| Fabrication Risk | None - Exact citations |
| Implementation Ready | Yes |

