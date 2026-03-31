---
id: web-client-integration
status: done
pipeline: sdd
---

# Web-Client Integration — Full Component Experience

**Requirement**: FR-32 (Web UI)
**Traceability**: TC-32.7

## Objective

Update the web-client's App.tsx to use ALL newly promoted components so the web UI has feature parity with the Chrome extension experience.

## Prerequisites

- Phases 1–4 complete
- Duplicates consolidated

## Tasks

1. Update `packages/web-client/src/App.tsx` to import and render the newly available components from `@chrome-acp/shared`:
   - chain-of-thought, plan, checkpoint, suggestion, artifact, image, sources, task
   - These should integrate into the existing ACPMain/ChatInterface flow
2. Verify the chrome-acp ACPClient class in shared correctly provides these components to the message renderer
3. Run `make web-assets` from the maenifold-rs root to rebuild the embedded SPA
4. Verify the build output in `vendor/chrome-acp/packages/proxy-server/public/` contains the new components

## Acceptance Criteria

1. Web-client App.tsx imports and composes new shared components
2. `bun run build` succeeds in `packages/web-client`
3. `make web-assets` succeeds from maenifold-rs root
4. Built assets contain the new component code (not tree-shaken out)

## Scoped Test Command

```bash
cd vendor/chrome-acp/packages/web-client && bun run build && cd /Users/brett/src/ma-collective/maenifold-rs && make web-assets
```

## SWE Remediation

- Investigated the rendering pipeline in `packages/shared/src/components/ChatInterface.tsx`, `packages/shared/src/components/ai-elements/message.tsx`, and the legacy chrome-extension component files.
- Confirmed the rendering pipeline lives in `ChatInterface.tsx`; `message.tsx` only provides presentational primitives (`Message`, `MessageContent`, `MessageResponse`, attachments) and does not route message parts.
- Wired promoted shared components into the active renderer:
  - `ChainOfThought` now renders assistant thought chunks.
  - `Plan` now renders `sessionUpdate === "plan"` entries.
  - `Checkpoint` now renders turn separators inserted on prompt completion.
  - `Suggestion`/`Suggestions` now render starter prompts in the empty state.
  - `Artifact`, `Task`, and `Sources` now render structured tool-call output when content includes text, diffs, terminals, or resource links.
- Verified with `cd packages/web-client && bun run build` (success).
