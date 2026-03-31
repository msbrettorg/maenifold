---
id: embedded-spa
status: done
---

# embedded-spa

**Requirement**: FR-32
**Test Cases**: TC-32.8
**Component**: crates/maenifold-core/src/web/mod.rs (new), vendor/chrome-acp (submodule)

## Acceptance Criteria

1. `vendor/chrome-acp` is a git submodule pointing to https://github.com/Areo-Joe/chrome-acp
2. A build script or Makefile step builds `packages/web-client` (via `bun run build` or `npm run build`) producing a `dist/` directory of static assets
3. `rust-embed` embeds the compiled `dist/` directory into the maenifold binary at compile time
4. axum serves embedded assets at `/` — `index.html` for the root, correct MIME types for `.js`, `.css`, `.svg`, `.png`, `.woff2`
5. SPA client-side routing: any path not matching `/mcp`, `/ws/*`, or `/api/*` falls back to `index.html`
6. Files under `dist/assets/` get `Cache-Control: max-age=31536000, immutable`. `index.html` and any file at the `dist/` root get `Cache-Control: no-cache`
7. Without the `web` feature flag, no static assets are embedded and `/` returns 404
8. If `vendor/chrome-acp` submodule is not initialized or `dist/` does not exist, `cargo build --features web` fails with a clear error message: 'Web UI assets not found. Run `make web-assets` or `cd vendor/chrome-acp && bun install && bun run build`'

## Verdicts

- TC-32.8 (SPA assets embedded and served with correct MIME types): ADD — test does not exist yet
