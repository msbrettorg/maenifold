# MCP Test Apps — Coverage Research (T-303.1)

## Current state

The repository currently has MCP configuration for maenifold itself, plus a narrow in-test loopback server used by desktop unit tests. I did not find a curated set of MCP rendering apps that can be launched from `maenifold.json` for manual or packaged-app coverage.

- `find . -name "maenifold.json" -not -path "*/node_modules/*"` found only `outputs/t241-root-c/maenifold.json`; it declares `"mcpServers": []` and one `Copilot` agent, so it does not exercise MCP rendering.
- `assets/integrations/claude-code/plugin-maenifold/.mcp.json` configures a single stdio MCP server named `maenifold` with `command: "maenifold"` and `args: ["--mcp"]`.
- `assets/integrations/copilot/plugin-maenifold/.mcp.json` configures a single HTTP MCP server named `maenifold` at `http://127.0.0.1:8080/mcp` with `tools: ["*"]`.
- `apps/desktop/electron/config-file.ts` seeds missing user config with an empty `mcpServers` array and empty `agents` object.
- `apps/desktop/forge.config.ts` packages the maenifold binary as an Electron extra resource; it does not bundle reference MCP servers or example apps.
- `apps/desktop/src/__tests__/helpers/mcp-loopback.ts` creates a unit-test-only Streamable HTTP MCP server named `maenifold-loopback-test`. It supports basic `GatewayIndex`, `GatewayExecute`, `Pipeline`, `resources/read`, `resources/list`, and resource subscription notifications, but its tools return text JSON or simple errors only. It is not a reusable rendering coverage app.
- I did not find an `mcp-apps` UAT skill or sibling docs under `assets/integrations/skills`; the only MCP-named skill reference docs found there were `azure-sre-agent/references/mcp-connectors.md` and `maenifold-testing/references/mcp-gateway.md`.

Net: current coverage is effectively "maenifold-internal MCP" plus unit-level loopback helpers. That covers text-ish JSON, resource reads, and some status plumbing, but it does not provide a curated app suite for images, resource links, embedded resources, MCP Apps UI, elicitation, sampling, progress, long payloads, cancellation, or tool errors in the desktop renderer.

## Renderer capabilities

The desktop renderer supports more MCP result shapes than the current configured apps exercise:

- **Tool-call status and cancellation label:** `ToolCallCard.vue` maps `pending`, `in_progress`, `completed`, and `failed` to colored status classes and icons, and labels `cancelled` as "Cancelled" even though it falls through the default class/icon branches.
- **Tool-call detail expansion:** `ChatToolCallItem.vue` treats content, raw input, raw output, and MCP App UI metadata as details; completed tool calls with `appUi.resourceUri` render an `McpAppFrame`.
- **ACP/MCP content blocks:** `ChatContentBlocks.vue` renders text via `KnowledgeNoteRenderer`, base64 images as `data:${mimeType};base64,...`, audio as `data:${mimeType};base64,...`, resource links via `ResourceLinkPreview`, and embedded resources via `ResourcePreview`.
- **Resource links:** `ResourceLinkPreview.vue` renders a resource-link chip, reads the resource on expansion via `useMcpResources().readResource`, applies descriptor fallbacks, and dispatches the fetched resource to `ResourcePreview`.
- **Embedded resource MIME dispatch:** `resource-mime-dispatcher.ts` maps resources to markdown, JSON, code/text, image, video, audio, or generic download previews based on MIME type plus `text`/`blob`.
- **Resource preview rendering:** `ResourcePreview.vue` renders markdown, JSON, fenced code, expandable images, video, audio, or a generic download card.
- **Diff and terminal blocks:** `ToolCallCardExtensions.vue` renders `diff` content as a unified diff and `terminal` blocks either by referenced terminal ID or retained inline output. `useToolCallTerminal.ts` strips ANSI, shows exit/signal/released labels, and exposes "Output truncated."
- **Unknown tool-call blocks:** `ToolCallCardExtensions.vue` falls back to pretty JSON for unknown content block shapes.
- **Raw payloads and structured content fallback:** `ToolCallCardExtensions.vue` shows raw input/output via `vue-json-pretty`; `mcpClientOps.ts` converts `structuredContent` to JSON text for GatewayIndex error messages, but normal tool results appear to rely on ACP `rawOutput` or content blocks rather than a dedicated structured-content renderer.
- **MCP Apps UI:** `agent-capabilities-helpers.ts` extracts `_meta.ui.resourceUri` or `appUi.resourceUri` only when it starts with `ui://`; `McpAppFrame.vue` fetches that resource, sends sandbox HTML through `@modelcontextprotocol/ext-apps/app-bridge`, passes raw input/output into the app, handles iframe size changes, supports safe open-link handling, and scopes app tool calls to tools advertised by the originating MCP server.
- **Sandbox CSP:** `electron/main.ts` registers `mcp-app-sandbox://` as a privileged scheme and serves sandbox pages with a restrictive CSP, with optional safe `connectDomains` additions from UI metadata.
- **Permission-request detail rendering:** `PermissionDialog.vue` displays raw input, raw output, and content blocks in permission prompts.
- **Prompt templates and argument completion: implemented.** T-303.18.1 wires `prompts/list` + `prompts/get` through `useMcpPrompts` + `useMcpPromptCommands` + `McpPromptPicker.vue`, surfaced as `mcp.<server>.<prompt>` slash commands in the chat composer. T-303.19 wires `completion/complete` into prompt argument entry when the server advertises the `completions` capability.

## Coverage matrix

| Content type | Supported in renderer? | Has test app? | Recommended test app |
|---|---|---|---|
| Text response | Y | Y | maenifold-internal / current loopback helper |
| Image (base64 data URI) | Y | N | Adopt `@modelcontextprotocol/server-everything` `get-tiny-image`; add a tiny built-in image app only if packaging via npx is rejected |
| Audio (base64 data URI) | Y | N | Build small local test app returning an `audio` content block; official Everything docs do not list direct audio content |
| Video resource blob | Y | N | Adopt `modelcontextprotocol/ext-apps` `video-resource-server` example |
| Resource link (`file://` / `demo://`) | Y | N | Adopt `@modelcontextprotocol/server-everything` `get-resource-links` |
| Embedded resource | Y | N | Adopt `@modelcontextprotocol/server-everything` `get-resource-reference` |
| Structured content (`structuredContent`) | Partial | N | Adopt `@modelcontextprotocol/server-everything` `get-structured-content` and TypeScript SDK `mcpServerOutputSchema.ts` |
| MCP Apps / chart-style UI (`_meta.ui.resourceUri`) | Y | N | Adopt `modelcontextprotocol/ext-apps` `cohort-heatmap-server` or `customer-segmentation-server`; `mcp-ui` is a secondary Apache-2.0 option |
| Long response (>25KB) | Partial | N | Build small local long-text MCP app to produce deterministic 25KB, 100KB, and 1MB responses |
| Elicitation request | Unknown / not proven in renderer | N | Adopt TypeScript SDK `elicitationFormExample.ts` and `elicitationUrlExample.ts`, or Everything async elicitation tools for task mode |
| Sampling request | Unknown / not proven in renderer | N | Adopt Everything `trigger-sampling-request` and TypeScript SDK `toolWithSampleServer.ts` |
| Cancellation | Partial | N | Adopt Everything task lifecycle / tasks cancel path; build a local cancellable long-running app if host-level cancellation is needed |
| Tool error (`isError: true`) | Partial | Partial | Use current loopback `Unknown tool` error for transport/unit checks; add Everything/SDK explicit tool-error scenario for packaged rendering |
| Progress notifications | Transport-level support likely; UI coverage not proven | N | Adopt Everything `trigger-long-running-operation` and TypeScript SDK progress examples |
| Logging notifications | Transport-level support likely; UI coverage not proven | N | Adopt Everything `toggle-simulated-logging` and TypeScript SDK logging examples |
| Resource subscriptions / updates | Partial | Partial | Current loopback emits `notifications/resources/updated`; adopt Everything `toggle-subscriber-updates` for real server behavior |
| Binary/download fallback | Y | N | Adopt Everything `gzip-file-as-resource` or ext-apps `pdf-server` for non-previewable resources |
| Prompt templates (`prompts/list` + `prompts/get`) | Y | Y | T-303.18.1 wired `mcp.<server>.<prompt>` slash commands; `server-everything` `simple-prompt`/`args-prompt`/`completable-prompt` exercise the path. CDP UAT filed as T-303.18.2. |
| Argument completion (`completion/complete`) | Y | Y | Adopt `@modelcontextprotocol/server-everything` `completable-prompt` (declares completable args) |

Total content type / edge-case rows audited: 19.

## Everything server coverage

`@modelcontextprotocol/server-everything` (T-303.12, adopted 2026-05-13) is the
first curated test fixture profile for maenifold. The table below maps each
coverage matrix row to the specific Everything tool that exercises it, or
explains why the row remains uncovered.

Full config and install steps: [`apps/desktop/docs/mcp-test-fixtures/everything.md`](mcp-test-fixtures/everything.md)

| Content type | Exercised by Everything? | Everything tool |
|---|---|---|
| Text response | Y | `echo` |
| Image (base64 data URI) | Y | `get-tiny-image` |
| Audio (base64 data URI) | N | No audio tool in upstream Everything |
| Video resource blob | N | No video tool in upstream Everything |
| Resource link (`file://` / `demo://`) | Y | `get-resource-links` |
| Embedded resource | Y | `get-resource-reference` |
| Structured content (`structuredContent`) | Y | `get-structured-content` |
| MCP Apps / chart-style UI (`_meta.ui.resourceUri`) | N/A | `ext-apps` fixture, not Everything |
| Long response (>25KB) | N | No deterministic large-payload tool in upstream Everything |
| Elicitation request | Y | `simulate-research-query` (ambiguous=true) / `trigger-elicitation-request-async` |
| Sampling request | Y | `trigger-sampling-request` |
| Cancellation | Y | `simulate-research-query` (MCP Tasks cancel path via `tasks/cancel`) |
| Tool error (`isError: true`) | N | No explicit tool-error fixture in upstream Everything |
| Progress notifications | Y | `trigger-long-running-operation` |
| Logging notifications | Y | `toggle-simulated-logging` |
| Resource subscriptions / updates | Y | `toggle-subscriber-updates` |
| Binary/download fallback | Y | `gzip-file-as-resource` |

Everything closes **11 of 17** rows. Remaining gaps: audio, video, MCP Apps UI
(ext-apps fixture required), long responses (local fixture required), and
explicit tool-error rendering.

## ext-apps server coverage

`modelcontextprotocol/ext-apps` example servers (T-303.13, adopted 2026-05-13) close the
**MCP Apps / chart-style UI** row that Everything explicitly defers to this fixture.

Full config and install steps: [`apps/desktop/docs/mcp-test-fixtures/ext-apps.md`](mcp-test-fixtures/ext-apps.md)

| Content type | Exercised by ext-apps? | ext-apps server |
|---|---|---|
| MCP Apps / chart-style UI (`_meta.ui.resourceUri`) | Y | `cohort-heatmap-server` or `basic-server-vanillajs` |
| Video resource blob | Y | `video-resource-server` |
| Binary/download fallback | Y | `pdf-server` |
| All other 14 rows | N | ext-apps is UI-focused; use Everything for protocol coverage |

ext-apps closes **3 of 17** rows. Combined with Everything, **14 of 17** rows now have
a curated fixture. Remaining gaps: audio, long responses, and explicit tool-error rendering.

## Gaps

- No configured or bundled MCP app currently exercises **image** content blocks in the packaged UI.
- No configured or bundled MCP app currently exercises **audio** content blocks.
- No configured or bundled MCP app currently exercises **video** resources.
- No configured or bundled MCP app currently exercises **resource_link** expansion through `resources/read`.
- No configured or bundled MCP app currently exercises **embedded resource** content blocks.
- No configured or bundled MCP app currently exercises **structuredContent** as a first-class output path; current rendering appears partial because structured content is mainly preserved as raw output or converted to text in error formatting.
- No configured or bundled MCP app currently exercises **MCP Apps UI** (`_meta.ui.resourceUri`) in the desktop sandbox.
- No configured or bundled MCP app currently exercises **long responses over 25KB**.
- No configured or bundled MCP app currently exercises **elicitation**.
- No configured or bundled MCP app currently exercises **sampling**.
- No configured or bundled MCP app currently exercises **cancellation** of long-running tool/task flows.
- No configured or bundled MCP app currently exercises **progress notifications**.
- No configured or bundled MCP app currently exercises **logging notifications**.
- Tool error coverage exists in the loopback helper, but not as a curated manual rendering app.

## Adoption candidates

1. **Adopt: `modelcontextprotocol/servers` Everything server**
   - Link: https://github.com/modelcontextprotocol/servers/tree/main/src/everything
   - License: repository is transitioning MIT to Apache-2.0; permissive for selected reference server use.
   - Covers: text, image, resource links, embedded resources, structured content, progress notifications, logging notifications, sampling, tasks, resource subscriptions, binary/download fallback, and some elicitation-via-task flows.
   - Why first: it is explicitly documented as a test server for MCP client builders and aims to exercise all protocol features.

2. **Adopt: `modelcontextprotocol/typescript-sdk` server examples**
   - Link: https://github.com/modelcontextprotocol/typescript-sdk/tree/main/examples/server
   - License: MIT / Apache-2.0 transition; permissive.
   - Covers: structured output (`mcpServerOutputSchema.ts`), form elicitation, URL elicitation, sampling, progress, logging, Streamable HTTP, stateless transport, task interactions.
   - Why: official SDK examples are small, focused, and map cleanly to one rendering behavior each.

3. **Adopt: `modelcontextprotocol/python-sdk` snippets**
   - Link: https://github.com/modelcontextprotocol/python-sdk/tree/main/examples/snippets/servers
   - License: MIT.
   - Covers: structured output, images, progress, elicitation, sampling, notifications.
   - Why: useful when a Python/uv app is cheaper than a TypeScript app for single-behavior fixtures.

4. **Adopt: `modelcontextprotocol/ext-apps` examples**
   - Link: https://github.com/modelcontextprotocol/ext-apps/tree/main/examples
   - License: MIT / Apache-2.0 transition; permissive.
   - Covers: MCP Apps UI via `_meta.ui.resourceUri`, sandboxed iframe rendering, app bridge behavior, charts/visualizations, PDF/video/binary resource demos.
   - Recommended picks: `cohort-heatmap-server`, `customer-segmentation-server`, `video-resource-server`, `pdf-server`, and `basic-server-vanillajs`.

5. **Adopt with caution: `idosal/mcp-ui` examples**
   - Link: https://github.com/idosal/mcp-ui
   - License: Apache-2.0.
   - Covers: MCP Apps and legacy MCP-UI patterns, raw HTML resources, external URL resources, Web Component demos, and a hosted authless demo.
   - Why secondary: the repo says its patterns influenced the MCP Apps standard and the packages implement MCP Apps, but maenifold already uses `@modelcontextprotocol/ext-apps/app-bridge`, so official `ext-apps` examples should be primary.

6. **Build small local app: long-response fixture**
   - Link: internal follow-up only.
   - License: maenifold repo license.
   - Covers: deterministic >25KB, >100KB, and very long text/JSON output with no external dependency.
   - Why build: official examples demonstrate many protocol shapes, but a deterministic size-bound payload is simpler and more stable as a tiny local app.

7. **Build small local app: audio fixture**
   - Link: internal follow-up only.
   - License: maenifold repo license.
   - Covers: an `audio` content block with a tiny generated WAV/MP3 base64 payload.
   - Why build: current renderer supports audio, but the official Everything feature list does not list direct audio content.

8. **Build small local app: explicit cancellation fixture**
   - Link: internal follow-up only.
   - License: maenifold repo license.
   - Covers: long-running tool plus client cancellation and renderer status checks.
   - Why build: Everything task cancellation is valuable, but a direct deterministic cancellation fixture is likely needed to verify maenifold's exact UI status path.

No GPL or AGPL candidate is recommended in this research. The recommended adoption path stays with official MCP projects or Apache-2.0/MIT community projects.

## Proposed next steps

- **T-303.12 — Wire Everything MCP server:** Add a documented maenifold config profile for `@modelcontextprotocol/server-everything`; acceptance: packaged desktop can call text, image, resource_link, embedded resource, structured content, progress, sampling, and logging tools and the doc records observed renderer outcomes.
- **T-303.13 — Wire official MCP Apps UI examples:** Add one or two `modelcontextprotocol/ext-apps` examples (`cohort-heatmap-server` or `customer-segmentation-server`, plus `basic-server-vanillajs`) as MCP App sandbox fixtures; acceptance: `_meta.ui.resourceUri` renders in `McpAppFrame`, sizes correctly, and scoped tool calls are denied/allowed as expected.
- **T-303.14 — Add deterministic local long/audio fixture:** Build a tiny MCP test app that returns long text/JSON payloads and a tiny audio content block; acceptance: desktop renders or safely contains >25KB output and audio controls without layout breakage.
- **T-303.15 — Wire official elicitation and sampling fixtures:** Add TypeScript SDK `elicitationFormExample.ts`, `elicitationUrlExample.ts`, and `toolWithSampleServer.ts` profiles; acceptance: renderer behavior is captured for accepted, declined, cancelled, and unsupported-client cases.
- **T-303.16 — Add cancellation/progress stress fixture:** Add a deterministic long-running fixture for progress ticks, cancellation, task failure, and tool `isError`; acceptance: statuses, progress/log visibility, final cancelled/failed state, and raw error payload rendering are verified.

## References

### Repository file references

- `outputs/t241-root-c/maenifold.json:1-9` — only `maenifold.json` found; empty `mcpServers`.
- `assets/integrations/claude-code/plugin-maenifold/.mcp.json:1-10` — maenifold stdio MCP config.
- `assets/integrations/copilot/plugin-maenifold/.mcp.json:1-8` — maenifold HTTP MCP config.
- `apps/desktop/electron/config-file.ts:19-27` — missing config defaults to empty `mcpServers`.
- `apps/desktop/forge.config.ts:99-118` — packages maenifold binary, no bundled MCP test servers.
- `apps/desktop/src/__tests__/helpers/mcp-loopback.ts:71-129` — loopback server tools return JSON text and simple errors.
- `apps/desktop/src/__tests__/helpers/mcp-loopback.ts:131-170` — loopback resource read/list/subscribe/unsubscribe handlers.
- `apps/desktop/src/__tests__/helpers/mcp-loopback.ts:184-297` — loopback Streamable HTTP transport setup.
- `apps/desktop/src/components/ToolCallCard.vue:15-54` — status rendering and cancellation label.
- `apps/desktop/src/components/ChatToolCallItem.vue:15-38` — app UI detection and `McpAppFrame` rendering.
- `apps/desktop/src/components/ChatContentBlocks.vue:12-18` — image/audio data URI helpers.
- `apps/desktop/src/components/ChatContentBlocks.vue:24-75` — text, image, audio, resource_link, resource rendering branches.
- `apps/desktop/src/components/ResourceLinkPreview.vue:39-62` — resource-link expansion and fetch behavior.
- `apps/desktop/src/components/ResourceLinkPreview.vue:64-90` — descriptor fallbacks and MIME icon mapping.
- `apps/desktop/src/lib/resource-mime-dispatcher.ts:49-98` — markdown/json/code/image/video/audio/download dispatch.
- `apps/desktop/src/components/ResourcePreview.vue:42-108` — resource preview render branches.
- `apps/desktop/src/components/ToolCallCardExtensions.vue:36-42` — raw payload and detail detection.
- `apps/desktop/src/components/ToolCallCardExtensions.vue:173-247` — content, diff, terminal, unknown block, and raw JSON rendering.
- `apps/desktop/src/composables/useToolCallTerminal.ts:8-67` — terminal ID/output/status/truncation helpers.
- `apps/desktop/src/composables/mcpClientOps.ts:173-200` — structured content JSON fallback for error formatting.
- `apps/desktop/src/composables/mcpClientOps.ts:360-390` — raw tool call path.
- `apps/desktop/src/composables/mcpClientOps.ts:393-405` — list tools with app UI extraction and read resource.
- `apps/desktop/src/stores/agent-capabilities-helpers.ts:34-64` — `_meta.ui` / `appUi` extraction and `ui://` guard.
- `apps/desktop/src/components/McpAppFrame.vue:81-147` — sandbox URL, resource fetch, send tool input/output, size changes.
- `apps/desktop/src/components/McpAppFrame.vue:149-218` — app open-link handling and scoped tool calls.
- `apps/desktop/electron/main.ts:34-36` — `mcp-app-sandbox` privileged scheme.
- `apps/desktop/electron/main.ts:182-220` — sandbox CSP behavior.
- `apps/desktop/src/components/PermissionDialog.vue:157-176` — permission prompt raw/content details.
- `apps/desktop/src/components/__tests__/ToolCallCardExtensions.test.ts:136-193` — existing unit tests for text/image/audio/resource_link/resource.
- `apps/desktop/src/components/__tests__/ToolCallCardExtensions.test.ts:200-260` — existing unit tests for diff and terminal blocks.

### External references

- https://github.com/modelcontextprotocol/servers — official reference servers; README lists Everything, Fetch, Filesystem, Git, Memory, Sequential Thinking, and Time.
- https://github.com/modelcontextprotocol/servers/tree/main/src/everything — Everything server; README states it is a test server for MCP client builders and exercises prompts, tools, resources, sampling, and more.
- https://github.com/modelcontextprotocol/servers/blob/main/src/everything/docs/features.md — Everything features list: image, resource links, embedded resources, structured content, progress, logging, sampling, tasks, resource subscriptions, and binary resources.
- https://github.com/modelcontextprotocol/servers/blob/main/LICENSE — MIT / Apache-2.0 transition license text.
- https://github.com/modelcontextprotocol/typescript-sdk — TypeScript SDK; README lists runnable examples and MIT / Apache-2.0 transition.
- https://github.com/modelcontextprotocol/typescript-sdk/blob/main/docs/server.md — server guide for tools, resource links, structured content, progress, sampling, and elicitation.
- https://github.com/modelcontextprotocol/typescript-sdk/tree/main/examples/server — server examples index for elicitation, sampling, structured output, tasks, and notifications.
- https://github.com/modelcontextprotocol/typescript-sdk/blob/main/examples/server/src/mcpServerOutputSchema.ts — structured-output example.
- https://github.com/modelcontextprotocol/typescript-sdk/blob/main/LICENSE — MIT / Apache-2.0 transition license text.
- https://github.com/modelcontextprotocol/python-sdk — Python SDK; README covers structured output, images, progress, elicitation, sampling, and notifications.
- https://github.com/modelcontextprotocol/python-sdk/blob/main/LICENSE — MIT license.
- https://github.com/modelcontextprotocol/ext-apps — official MCP Apps extension; README covers `_meta.ui.resourceUri`, app bridge, sandboxed UIs, examples, and Apache-2.0 badge / MIT-Apache transition license.
- https://github.com/modelcontextprotocol/ext-apps/tree/main/examples — MCP Apps examples including heatmap, customer segmentation, video resource, PDF, QR, and basic framework servers.
- https://github.com/modelcontextprotocol/ext-apps/blob/main/LICENSE — MIT / Apache-2.0 transition license text.
- https://github.com/idosal/mcp-ui — MCP-UI SDK; README says `@mcp-ui/*` packages implement MCP Apps and provides server/client examples.
- https://github.com/idosal/mcp-ui/blob/main/LICENSE — Apache-2.0 license.
