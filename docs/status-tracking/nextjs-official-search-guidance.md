# Next.js Official Search Guidance Research - 2025-10-28

## Task

Research official Next.js documentation for search functionality recommendations in static export mode.

**Requirements:**
- ONLY use official Next.js documentation (nextjs.org)
- Search for "search", "site search", "documentation search"
- Look in the static export documentation for search mentions
- Find if Next.js recommends any specific search solutions
- Check the Next.js examples repository for search implementations
- If Next.js does NOT provide official search guidance, state that explicitly

**Output Required:**
Create a markdown document with:
1. Exact quotes from official Next.js docs about search (if any)
2. Officially recommended search solutions (if any)
3. Next.js examples that include search (with GitHub URLs)
4. Explicit statement if NO official search guidance exists
5. Source URLs for all findings

## Task Restated

I need to search the official Next.js documentation (nextjs.org) specifically for:
1. Search functionality recommendations for static export mode
2. Any mentions of search in static export documentation
3. Official examples that include search implementations

**Assumptions:**
- I will only cite official Next.js sources (nextjs.org domain and official Next.js GitHub repositories)
- I will provide exact quotes where available
- I will explicitly state if no official guidance exists
- I will not infer or assume recommendations that are not explicitly stated

**What I will do:**
- Search Next.js documentation for search-related content
- Check static export documentation specifically
- Look for Next.js official examples with search implementations
- Document all findings with source URLs

**What I will NOT do:**
- Include third-party recommendations
- Make assumptions about what Next.js "would recommend"
- Include community solutions unless officially endorsed

## Plan

1. Search Next.js documentation for "search" and "static export"
2. Check the static export documentation page directly
3. Search for Next.js examples repository for search implementations
4. Document findings with exact quotes and URLs
5. Update this document with complete findings

## Status

Research completed. Findings documented below.

---

# Research Findings

## Executive Summary

**EXPLICIT STATEMENT: Next.js does NOT provide official recommendations for search solutions specifically for static export mode.**

The official Next.js documentation:
- Does NOT recommend any specific search service or library for static sites
- Does NOT mention search functionality in the static export documentation
- Does NOT provide guidance on implementing site search for statically exported sites
- DOES provide one example (Algolia integration) but without explicit recommendation or static export guidance

## 1. Static Export Documentation - No Search Mentions

### Source: https://nextjs.org/docs/app/guides/static-exports

The official static export documentation **does not mention search functionality at all**.

The documentation focuses on:
- How to enable static export (`output: 'export'` in next.config.js)
- Supported and unsupported features
- Image optimization
- Route configuration

### Unsupported Features in Static Export

**Exact quote from documentation:**

> "Features that require a Node.js server, or dynamic logic that cannot be computed during the build process, are not supported"

Specifically unsupported features include:
- "Route Handlers that rely on Request"
- "Cookies"
- "Rewrites"
- "Redirects"
- "Headers"
- "Proxy"
- "Incremental Static Regeneration"
- "Image Optimization with the default `loader`"
- "Draft Mode"
- "Server Actions"
- "Intercepting Routes"

**Implication for search:** Any search solution requiring server-side API routes or dynamic server functionality would NOT work with static export.

## 2. Next.js Learn Tutorial - Client-Server Search (Not for Static Export)

### Source: https://nextjs.org/learn/dashboard-app/adding-search-and-pagination

Next.js provides a tutorial on implementing search, but this is **for server-rendered applications, NOT static exports**.

### Exact Quotes:

> "Your search functionality will span the client and the server. When a user searches for an invoice on the client, the URL params will be updated, data will be fetched on the server..."

### Recommended Approach (Server-Rendered Apps Only):

**Benefits of URL Search Parameters:**
1. **Shareability**: "Since the search parameters are in the URL, users can bookmark the current state of the application, including their search queries and filters..."
2. **Server-side rendering**: URL parameters can be consumed directly on the server to render initial state
3. **Analytics**: "Having search queries and filters directly in the URL makes it easier to track user behavior without requiring additional client-side logic"

**Essential Next.js Hooks:**
- `useSearchParams` - Access current URL parameters
- `usePathname` - Read the current URL's pathname
- `useRouter` - Enable programmatic navigation

**Best Practice - Debouncing:**
> "Debouncing is a programming practice that limits the rate at which a function can fire."

**Recommended library**: `use-debounce` package with suggested 300ms delay

**CRITICAL NOTE:** This tutorial approach requires server-side data fetching and is **NOT compatible with static export mode**.

## 3. Official Next.js Examples

### Source: https://github.com/vercel/next.js/tree/canary/examples

Only **ONE** official example related to search was found:

### with-algolia-react-instantsearch

**GitHub URL:** https://github.com/vercel/next.js/tree/canary/examples/with-algolia-react-instantsearch

**Description from README:**
> "This example shows how you can use Algolia React InstantSearch to perform your search in an application developed with Next.js"

**Key Features Demonstrated:**
- Integration with Algolia React InstantSearch library
- URL synchronization: "how you can keep in sync the Url with the search"
- Pre-configured e-commerce index for quick testing
- Customizable with own Algolia account data

**Installation:**
```bash
npx create-next-app --example with-algolia-react-instantsearch with-algolia-react-instantsearch-app
```

**Configuration:** Update `APP_ID`, `API_KEY`, and `INDEX_NAME` in `components/Search.tsx`

**IMPORTANT LIMITATIONS:**
- The example does NOT specify if it works with static export
- The example does NOT include `output: 'export'` configuration
- Algolia React InstantSearch typically requires client-side JavaScript
- No documentation about using this example in static export mode

## 4. What Next.js Documentation DOES NOT Say

**Explicit Statement:** After comprehensive research of official Next.js documentation and examples, the following do NOT exist:

1. ❌ No official recommendation for search solutions for static export mode
2. ❌ No mention of search functionality in static export documentation
3. ❌ No guidance on client-side search libraries for static sites
4. ❌ No recommendations for:
   - Algolia (beyond one example with no explicit recommendation)
   - Fuse.js
   - Lunr.js
   - Pagefind
   - Any other search solution
5. ❌ No discussion of search indexing strategies for static sites
6. ❌ No examples demonstrating search in static export mode

## 5. Summary of Search-Related Official Next.js Content

| Content Type | URL | Applicability to Static Export | Recommendation? |
|--------------|-----|-------------------------------|-----------------|
| Static Export Guide | https://nextjs.org/docs/app/guides/static-exports | No search mentions | No |
| Learn Tutorial: Search & Pagination | https://nextjs.org/learn/dashboard-app/adding-search-and-pagination | Server-rendered only, NOT static | No |
| Example: Algolia InstantSearch | https://github.com/vercel/next.js/tree/canary/examples/with-algolia-react-instantsearch | Unknown, not documented | No explicit recommendation |
| API: useSearchParams | https://nextjs.org/docs/app/api-reference/functions/use-search-params | Client Component hook | Technical reference only |
| SEO: Search Systems | https://nextjs.org/learn/seo/introduction-to-seo/search-systems | About Google/Bing, not site search | No |

## 6. Technical Context for Search in Static Export

Based on the documented unsupported features, any search solution for Next.js static export must:

✅ **Must be:**
- Fully client-side (no server API routes)
- Compatible with static HTML/CSS/JS files
- Able to work without Node.js server
- Able to work without dynamic routes or server components

❌ **Cannot rely on:**
- Server-side API routes
- Dynamic server rendering
- Route Handlers with Request objects
- Server Actions

## Conclusion

**Next.js provides NO official guidance or recommendations for implementing search functionality in static export mode.**

The only search-related content in official Next.js documentation is:
1. A server-rendered search tutorial (incompatible with static export)
2. One Algolia example (without explicit recommendation or static export guidance)
3. Technical API references for URL search parameters

**Ma Protocol Compliance:** This research makes no assumptions. Next.js documentation is silent on search solutions for static sites.
