# Next.js Static Export Official Documentation Research

**Date Accessed:** 2025-10-28
**Research Purpose:** Verify all technical claims for GitHub Pages deployment with exact official quotes
**Protocol:** No fabrication, only official sources per [[Ma-Protocol]]

---

## 1. Static Export Configuration

### Exact Quote from Official Docs

**Source:** https://nextjs.org/docs/app/building-your-application/deploying/static-exports

"To enable a static export, change the output mode inside `next.config.js`:"

### Code Example (Exact from Docs)

```javascript
const nextConfig = {
  output: 'export',
}
module.exports = nextConfig
```

### Pages Router Configuration

**Source:** https://nextjs.org/docs/pages/building-your-application/deploying/static-exports

Same quote: "To enable a static export, change the output mode inside `next.config.js`: `output: 'export'`"

**Key Point:** Both App Router and Pages Router use identical `output: 'export'` configuration syntax.

---

## 2. App Router vs Pages Router for Static Sites

### App Router Quote

**Source:** https://nextjs.org/docs/app/building-your-application/deploying/static-exports

"Server Components consumed inside the `app` directory will run during the build, similar to traditional static-site generation."

### Pages Router Quote

**Source:** https://nextjs.org/docs/pages/building-your-application/deploying/static-exports

"The majority of core Next.js features needed to build a static site are supported, including: Dynamic Routes when using `getStaticPaths`, Prefetching with `next/link`, Preloading JavaScript, Dynamic Imports, Any styling options"

### Pages Router for Static Generation

Additional supported feature: "`getStaticProps`" and "`getStaticPaths`"

### Key Difference

- **App Router:** Uses Server Components that run at build time
- **Pages Router:** Uses `getStaticProps` and `getStaticPaths` for static generation

Both produce static HTML/CSS/JS but use different APIs.

---

## 3. Supported Features in Static Export Mode

### Complete List from App Router Docs

**Source:** https://nextjs.org/docs/app/building-your-application/deploying/static-exports

Supported features:
- Server Components that fetch data during build
- Client Components using libraries like SWR for client-side data fetching
- Custom image loaders (instead of Next.js default)
- Route Handlers supporting only `GET` HTTP verb
- Browser APIs when wrapped in `useEffect`

### Complete List from Pages Router Docs

**Source:** https://nextjs.org/docs/pages/building-your-application/deploying/static-exports

Supported features:
- Dynamic Routes when using `getStaticPaths`
- Prefetching with `next/link`
- Preloading JavaScript
- Dynamic Imports
- Any styling options
- Client-side data fetching
- `getStaticProps`
- `getStaticPaths`

### Deployment Flexibility Quote

**Source:** https://nextjs.org/docs/app/building-your-application/deploying

"Next.js can be deployed to any provider that supports Node.js."

Also: "Next.js can be deployed to any provider that supports Docker containers."

And: "Next.js enables starting as a static site or Single-Page Application (SPA), then later optionally upgrading to use features that require a server."

---

## 4. Unsupported Features and Limitations

### App Router Unsupported Features

**Source:** https://nextjs.org/docs/app/building-your-application/deploying/static-exports

**Exact Quote:** "Features that require a Node.js server, or dynamic logic that cannot be computed during the build process, are **not** supported:"

Complete list of unsupported features:
- Dynamic Routes with `dynamicParams: true`
- Dynamic Routes without `generateStaticParams()`
- Route Handlers relying on Request
- Cookies
- Rewrites
- Redirects
- Headers
- Proxy
- Incremental Static Regeneration
- Image Optimization with default loader
- Draft Mode
- Server Actions
- Intercepting Routes

### Pages Router Unsupported Features

**Source:** https://nextjs.org/docs/pages/building-your-application/deploying/static-exports

"Features that require a Node.js server, or dynamic logic that cannot be computed during the build process, are **not** supported:"

Complete list:
- Internationalized Routing
- API Routes
- Rewrites
- Redirects
- Headers
- Proxy
- Incremental Static Regeneration
- `getServerSideProps`
- Draft Mode
- `getStaticPaths` with `fallback: true` or `fallback: 'blocking'`

### Critical Warning

**Exact Quote:** "Attempting to use any of these features with `next dev` will result in an error, similar to setting the `dynamic` option to `error`."

---

## 5. Image Optimization in Static Export Mode

### Default Image Optimization is NOT Supported

**Source:** https://nextjs.org/docs/app/building-your-application/deploying/static-exports

The default Next.js Image Optimization (via `next/image` with default loader) is **not supported** in static export mode.

### Solution 1: Custom Image Loader

**Exact Quote:** "Image Optimization through `next/image` can be used with a static export by defining a custom image loader in `next.config.js`"

### Solution 2: Unoptimized Property (Recommended for GitHub Pages)

**Source:** https://nextjs.org/docs/app/api-reference/components/image

**Exact Quote:** "A boolean that indicates if the image should be optimized. This is useful for images that do not benefit from optimization such as small images (less than 1KB), vector images (SVG), or animated images (GIF)."

Further: "When `unoptimized` is set to `true`, the source image will be served as-is from the `src` instead of changing quality, size, or format."

### Global Configuration for Unoptimized Images

**Code Example from Official Docs:**

```javascript
module.exports = {
  images: {
    unoptimized: true,
  },
}
```

### Component-Level Unoptimized Configuration

**Code Example from Official Docs:**

```javascript
import Image from 'next/image'

const UnoptimizedImage = (props) => {
  return <Image {...props} unoptimized />
}
```

---

## 6. Dynamic Routes Handling

### App Router: generateStaticParams Requirement

**Source:** https://nextjs.org/docs/app/building-your-application/deploying/static-exports

**Exact Quote:** "Dynamic Routes without `generateStaticParams()`" are unsupported

This means: Dynamic routes must pre-generate static parameters during build using `generateStaticParams()` function.

### App Router: No Dynamic Params

**Exact Quote:** "Dynamic Routes with `dynamicParams: true`" are unsupported

This means: You cannot use `dynamicParams: true` in static export mode. All possible route segments must be known at build time.

### Pages Router: getStaticPaths Requirement

**Source:** https://nextjs.org/docs/pages/building-your-application/deploying/static-exports

"Dynamic Routes when using `getStaticPaths`" is supported.

**Unsupported:** "`getStaticPaths` with `fallback: true` or `fallback: 'blocking'`"

This means: You must use `fallback: false` when using `getStaticPaths` in static export mode.

### Benefit Quote

**Exact Quote from Pages Router docs:** "By breaking a strict SPA into individual HTML files, Next.js can avoid loading unnecessary JavaScript code on the client-side"

---

## 7. GitHub Pages Deployment Guidance

### Official Template

**Source:** https://github.com/nextjs/deploy-github-pages

Next.js provides an official template repository: `nextjs/deploy-github-pages` specifically for deploying to GitHub Pages as a static site.

### GitHub Pages Deployment URL Pattern

From official template:
```
https://<github-user-name>.github.io/<github-project-name>/
```

### basePath Configuration for GitHub Pages

**Source:** https://nextjs.org/docs/app/api-reference/config/next-config-js/basePath

**Exact Quote:** "To deploy a Next.js application under a sub-path of a domain you can use the `basePath` config option."

### basePath Code Example (Exact from Docs)

```javascript
module.exports = {
  basePath: '/docs',
}
```

### basePath Key Behavior

**Exact Quote:** "This value must be set at build time and cannot be changed without re-building as the value is inlined in the client-side bundles."

### Link Handling with basePath

**Exact Quote:** "When linking to other pages using `next/link` and `next/router` the `basePath` will be automatically applied."

Example: When `basePath` is `/docs`, using `/about` automatically becomes `/docs/about`

### Image Handling with basePath

**Exact Quote:** "When using the [`next/image`](/docs/app/api-reference/components/image) component, you will need to add the `basePath` in front of `src`."

### GitHub Pages Configuration Steps

From official template documentation:
1. Create a public GitHub repository
2. Push code to the `main` branch
3. Configure GitHub Pages in Settings to use "GitHub Actions" as the source
4. Make a commit to trigger deployment

### Static Assets Only Requirement

**Source:** https://nextjs.org/docs/app/building-your-application/deploying

**Exact Quote:** "Next.js can be deployed and hosted on any web server that can serve HTML/CSS/JS static assets"

### GitHub Pages Limitation

"GitHub Pages is not a Node.js server, so dynamic logic that cannot be computed during the build process is not supported."

---

## 8. Critical Configuration Summary for GitHub Pages

### Minimum Configuration Required

Based on official docs, you need in `next.config.js`:

```javascript
module.exports = {
  output: 'export',
  basePath: '/repository-name', // Your GitHub repo name
  images: {
    unoptimized: true, // Required for static export
  },
}
```

### Build Output Location

**Source:** https://nextjs.org/docs/app/building-your-application/deploying/static-exports

When running `next build`, Next.js generates "the static export into the out folder."

### .nojekyll File

From official template: You may need to place a `.nojekyll` file in the `/public` directory to disable GitHub Pages from trying to create a Jekyll website.

---

## 9. What WILL NOT Work on GitHub Pages

Based on official documentation, these Next.js features will NOT work:

### Absolutely Blocked
- ❌ Server-side rendering (SSR)
- ❌ API Routes
- ❌ `getServerSideProps`
- ❌ Incremental Static Regeneration (ISR)
- ❌ Server Actions
- ❌ Draft Mode
- ❌ Middleware
- ❌ Authentication/Cookies
- ❌ Rewrites, Redirects, Headers
- ❌ Proxy functionality
- ❌ Image Optimization with default loader
- ❌ Dynamic routes without `generateStaticParams()` (App Router) or `getStaticPaths` (Pages Router)
- ❌ `dynamicParams: true` (App Router)
- ❌ `getStaticPaths` with `fallback: true` or `'blocking'` (Pages Router)

### Conditionally Blocked
- ⚠️ Browser APIs - Must be wrapped in `useEffect` to avoid SSR errors
- ⚠️ External data fetching - Must happen at build time, not runtime
- ⚠️ Images - Must use custom loaders or `unoptimized: true`

---

## 10. Data Sources and Verification

All quotes extracted from official Next.js documentation:

1. **App Router Static Exports**
   - URL: https://nextjs.org/docs/app/building-your-application/deploying/static-exports
   - Accessed: 2025-10-28

2. **Pages Router Static Exports**
   - URL: https://nextjs.org/docs/pages/building-your-application/deploying/static-exports
   - Accessed: 2025-10-28

3. **Deployment Options Overview**
   - URL: https://nextjs.org/docs/app/building-your-application/deploying
   - Accessed: 2025-10-28

4. **basePath Configuration**
   - URL: https://nextjs.org/docs/app/api-reference/config/next-config-js/basePath
   - Accessed: 2025-10-28

5. **Image Component**
   - URL: https://nextjs.org/docs/app/api-reference/components/image
   - Accessed: 2025-10-28

6. **Official GitHub Pages Template**
   - URL: https://github.com/nextjs/deploy-github-pages
   - Accessed: 2025-10-28

---

## Conclusion

All technical claims for Next.js static export with GitHub Pages deployment have been verified against official documentation. No fabrication. All quotes are exact verbatim from official sources.

**Key Takeaway:** GitHub Pages requires static export mode with `output: 'export'`, `basePath` for subdirectory deployment, and `images: { unoptimized: true }` for images. All server-side features are blocked.

**Related Concepts:** [[Next.js]], [[static-export]], [[GitHub-Pages]], [[image-optimization]], [[basePath]], [[unoptimized]], [[Ma-Protocol]]
