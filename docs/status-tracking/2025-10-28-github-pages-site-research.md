# Task: GitHub Pages Site Research and Options

**Date**: 2025-10-28

## Task

> I need a github pages site for /Users/brett/src/ma-collective/maenifold - Something modern and beautiful. Investigate the product, read /Users/brett/src/ma-collective/maenifold/README.md and /Users/brett/src/ma-collective/maenifold/docs/README.md and /Users/brett/src/ma-collective/maenifold/assets/branding and /Users/brett/src/ma-collective/maenifold/assets/demo-artifacts/README.md and /Users/brett/src/ma-collective/maenifold/src/assets. Search memory:// build_context, visualise, find_similar_concepts. I need you to really GROK what we're doing here. No changes, just well researched options. I'm tired of re-doing this each time things need formatting

## Task Restated

User wants comprehensive research and well-considered options for a modern, beautiful GitHub Pages site for the maenifold project. This is explicitly a research phase - NO implementation, NO changes. The goal is to deeply understand:

1. What maenifold IS (product understanding)
2. What maenifold DOES (functionality/purpose)
3. Visual identity (branding assets)
4. Knowledge graph structure (concepts and relationships)
5. Demo artifacts and use cases
6. Technical constraints and requirements

**Assumptions:**
- User is frustrated with previous iterations that required rework due to poor formatting/architecture choices
- Solution must work reliably on GitHub Pages (static site generation, no server-side processing)
- Site must reflect the sophisticated nature of the product (knowledge graphs, AI orchestration, cognitive workflows)
- Previous site attempts failed due to markdown/layout processing issues on GitHub Pages
- Need robust, production-ready solution that won't need constant rework

**What I will do:**
- Read all specified documentation files
- Search knowledge graph for related concepts
- Analyze branding assets
- Review demo artifacts to understand use cases
- Build comprehensive understanding of the product's value proposition
- Research modern static site options that work reliably on GitHub Pages
- Present multiple well-researched architectural options with pros/cons

**What I will NOT do:**
- Make any code changes
- Create any new files (except this tracking document)
- Commit or push anything
- Start implementation

## Plan

1. Read core documentation to understand the product
2. Explore branding assets for visual identity
3. Search knowledge graph for key concepts
4. Review demo artifacts to understand real-world usage
5. Synthesize findings into product understanding
6. Research modern static site generator options
7. Present architectural recommendations

## Status

### Research Completed

**Product Understanding:**
- Maenifold is a test-time reasoning infrastructure for AI agents
- Core concept: "Your AI is ephemeral. Your knowledge shouldn't be."
- Transforms ephemeral AI sessions into continuous collective intelligence through persistent knowledge graphs
- Built on 間 (Ma) principles - "the space between thoughts becomes knowledge"
- Not just RAG - a complete cognitive architecture with memory, graph, and reasoning layers

**Key Technical Details:**
- .NET 9.0, C#, SQLite with vectors (384-dim embeddings)
- MCP-compliant server with CLI/MCP full parity
- 54 cognitive assets: 28 workflows, 7 roles, 7 thinking hats, 12 linguistic perspectives
- Hybrid RRF search combining semantic + full-text
- Lazy graph construction from [[WikiLinks]]
- Tested at >1.1M relationships
- Real E2E testing with 25 agents finding actual bugs

**Visual Identity:**
- Logo: Animated neural network with pulsing central node, clean monospace typography
- Graph visualization: Dark background with heat-mapped nodes (blue→green→yellow→red by connection strength)
- Color scheme: GitHub-compatible (light/dark mode aware)
- Style: Technical, sophisticated, data-visualization focused

**Key Differentiators:**
1. Multi-day persistent reasoning sessions (sequential thinking)
2. Multi-agent orchestration with "PM Protocol" pattern
3. Complete observability (all thoughts visible as markdown)
4. Real testing philosophy (no mocks, actual bugs found)
5. Cognitive compounding effect over time
6. Files as source of truth (git-compatible, Obsidian-ready)

**Target Audience:**
- Developers building AI agent systems
- Teams needing persistent AI knowledge
- Multi-agent orchestration needs
- Anyone frustrated with stateless AI conversations

**Content Structure Needed:**
1. Landing page emphasizing the value prop and demo results
2. Getting started (install, configure, first use)
3. Tool documentation (26 tools with detailed parameters)
4. Workflow documentation (30 methodologies)
5. Demo artifacts showcase (68-minute orchestration story)
6. Architecture/philosophy deep dive
7. Real-world examples and patterns

### Previous Site Issues
- Markdown with layout frontmatter doesn't work on GitHub Pages
- Needed pre-built HTML, not raw markdown
- Site was deleted to start fresh

### Next: Compile architectural options

---

## Architectural Options for GitHub Pages Site

### Option 1: Astro (Recommended)
**Why it fits maenifold:**
- Built for content-heavy documentation sites
- Islands architecture for interactive visualizations (knowledge graph viewer)
- Zero JS by default (fast, matches the "lean" philosophy)
- Excellent MDX support for mixing markdown + React components
- Built-in collections for organizing tools/workflows
- GitHub Pages deployment built-in (`astro build` → static HTML)
- Active community, great DX

**Technical approach:**
- Use `.astro` components for layouts (Base, Docs, Landing)
- Use `.mdx` for tool documentation (allows embedding interactive examples)
- Collections for organizing tools, workflows, roles
- ViewTransitions API for smooth SPA-like navigation
- React/Svelte/Vue islands for graph visualizations only

**Strengths:**
- Perfect for docs + interactive elements mix
- Fast builds, fast sites
- Content-focused with escape hatches for interactivity
- Works perfectly with GitHub Pages (builds to static HTML)
- Can embed 3D graph visualization as island

**Weaknesses:**
- Learning curve for `.astro` component syntax
- Need to migrate 26 tool docs carefully

**Estimated effort:** 2-3 days for full site with polish

---

### Option 2: Next.js with Static Export
**Why it might fit:**
- Extremely popular, great ecosystem
- Can use App Router for better layouts
- React Server Components for content
- Good MDX integration via `@next/mdx`
- Static export mode works with GitHub Pages

**Technical approach:**
- App Router with nested layouts
- MDX for all content
- Static export (`next build && next export`)
- React components for everything

**Strengths:**
- Huge ecosystem
- Many examples/templates available
- Good TypeScript support
- Rich component libraries

**Weaknesses:**
- Overkill for primarily static content site
- Heavier bundle sizes by default
- More complex than needed
- Static export has some limitations
- Slower builds than Astro

**Estimated effort:** 3-4 days (complexity overhead)

---

### Option 3: VitePress
**Why it might fit:**
- Built specifically for documentation
- Vue-based but hides complexity
- Fast dev server (Vite)
- Great default theme
- Simple configuration

**Technical approach:**
- Markdown-first with Vue components
- Theme customization via CSS variables
- Custom components for special sections
- Built-in search

**Strengths:**
- Designed for docs
- Very fast
- Simple to get started
- Good defaults

**Weaknesses:**
- Less flexible than Astro for custom layouts
- Vue ecosystem (smaller than React)
- Harder to integrate complex visualizations
- Less control over HTML structure

**Estimated effort:** 2 days (fast but less customized)

---

### Option 4: Docusaurus (Meta)
**Why it might fit:**
- Purpose-built for docs
- React-based
- Versioning support
- Good plugin ecosystem

**Technical approach:**
- MDX for all content
- Sidebar/navbar configuration
- React components for custom elements
- Static build to GitHub Pages

**Strengths:**
- Made for this exact use case
- Great docs out of the box
- Versioning (useful for API changes)
- Search built-in

**Weaknesses:**
- Opinionated structure (less flexibility)
- React-heavy (heavier than needed)
- Harder to achieve custom branding
- Learning curve for customization

**Estimated effort:** 2-3 days (quick start, harder customization)

---

### Option 5: Eleventy (11ty)
**Why it might fit:**
- Extreme simplicity
- Framework-agnostic
- Works with any template language
- Very lightweight output

**Technical approach:**
- Nunjucks/Liquid templates
- Markdown for content
- Custom shortcodes for special elements
- Static build

**Strengths:**
- Minimal bundle size
- Maximum flexibility
- Simple mental model
- Fast builds

**Weaknesses:**
- More manual work for everything
- No built-in graph visualization support
- Harder to integrate modern UI patterns
- Steeper learning curve for complex layouts

**Estimated effort:** 4-5 days (more from scratch)

---

## Recommendation: Astro

**Why Astro is the best fit for maenifold:**

1. **Philosophy alignment**: Astro's "islands architecture" matches Ma protocol's minimalism - ship only what's needed
2. **Content + visualization**: Perfect mix for docs with interactive graph viz
3. **Developer experience**: Fast dev server, great DX, excellent TypeScript support
4. **GitHub Pages compatibility**: First-class support, no gotchas
5. **Future-proof**: Can add React/Svelte/Vue components as needed without full rewrite
6. **Performance**: Zero JS by default = fast, matches the technical sophistication of the product
7. **Flexibility**: Full control over HTML/CSS for brand expression

**Specific features for maenifold:**
- **3D Graph Visualization** as a React island (three.js/force-graph)
- **Tool documentation** with syntax-highlighted code examples
- **Workflow browser** with filterable/searchable interface
- **Demo artifacts** showcase with timeline view
- **Interactive examples** for key concepts
- **Dark mode** (essential for developer tools)
- **Fast search** via Pagefind integration

**Architecture:**
```
site/
├── src/
│   ├── pages/
│   │   ├── index.astro          # Landing page
│   │   ├── start.astro          # Quick start
│   │   ├── docs/
│   │   │   ├── architecture.astro
│   │   │   └── philosophy.astro
│   │   ├── tools/
│   │   │   ├── index.astro      # Tool browser
│   │   │   └── [tool].astro     # Dynamic tool pages
│   │   ├── workflows/
│   │   │   ├── index.astro      # Workflow browser
│   │   │   └── [workflow].astro # Dynamic workflow pages
│   │   └── demo/
│   │       └── index.astro      # Demo showcase
│   ├── layouts/
│   │   ├── Base.astro           # Base layout with header/footer
│   │   ├── Docs.astro           # Docs layout with sidebar
│   │   └── Landing.astro        # Landing page layout
│   ├── components/
│   │   ├── GraphVisualization.jsx  # React island for 3D graph
│   │   ├── CodeExample.astro
│   │   ├── ToolCard.astro
│   │   └── WorkflowCard.astro
│   └── content/
│       ├── tools/               # Tool docs as .mdx
│       └── workflows/           # Workflow docs as .mdx
├── public/
│   ├── assets/
│   │   ├── branding/
│   │   └── demo-artifacts/
└── astro.config.mjs
```

**Key implementation details:**
- Use Astro Content Collections for type-safe tool/workflow data
- Server-side syntax highlighting (shiki) for code examples
- View Transitions for smooth navigation
- Responsive design mobile-first
- Accessible (WCAG AA minimum)
- SEO optimized with proper meta tags
- RSS feed for updates

**Why not the others:**
- Next.js: Too heavy for primarily static content
- VitePress: Not flexible enough for custom branding/visualizations
- Docusaurus: Too opinionated, harder to achieve unique design
- Eleventy: Too much manual work, harder to maintain

**Risk mitigation:**
- Start with core layouts first (Base, Docs, Landing)
- Test GitHub Pages deployment early
- Migrate one tool doc to validate approach
- Add interactivity incrementally (graph viz can come later)
- Keep old content as reference during migration

This gives you a modern, beautiful, maintainable site that won't need constant rework and properly represents the sophistication of maenifold.
