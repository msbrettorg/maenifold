# Sprint 244 Agents tab performance audit — T-244.73

## Environment

- Worktree: `worktrees/T-244.73`
- Baseline reference: `/tmp/acp-ui` at `09f6ecf`
- Harness: `apps/desktop/src/composables/__tests__/agents-perf.test.ts`
- Command: `npx vitest run src/composables/__tests__/agents-perf.test.ts`
- Runtime: headless jsdom + Vue Test Utils + Node performance hooks.
- acp-ui comparison: stripped dispatcher reference matching `/tmp/acp-ui/src/stores/session.ts` `agent_message_chunk` behavior. Direct acp-ui mount was not used because the Tauri store imports are not directly runnable in this Electron/jsdom test environment.

## Metrics

| Scenario | acp-ui/reference | ours-before | ours-after H1 | Target | Result |
| --- | ---: | ---: | ---: | ---: | --- |
| 100 `agent_message_chunk` notifications, total | 0.09 ms dispatcher-only | 2017.33 ms | 131.43 ms | < 2000 ms | PASS |
| 100 `agent_message_chunk` notifications, max chunk | n/a | 1860.72 ms | 7.07 ms | < 16 ms | PASS |
| 100 `agent_message_chunk` notifications, average chunk | n/a | 20.17 ms | 1.31 ms | < 16 ms | PASS |
| 50 `tool_call_update` notifications | n/a | 58.55 ms | 62.58 ms | < 750 ms | PASS |
| 2000-token markdown message with code block, warmed markdown engine | n/a | 100.43 ms | 113.07 ms | < 1000 ms | PASS |

## Hypothesis disposition

| Hypothesis | Status | Evidence | Action |
| --- | --- | --- | --- |
| H1 — Markdown re-parse on every chunk | Confirmed | Before the fix, the first streamed chunk forced `KnowledgeNoteRenderer`/`MarkdownView` to initialize and parse the growing text block, producing a 1860.72 ms max chunk and 2017.33 ms total for 100 chunks. | Fixed by rendering the active assistant text stream as escaped plain text while `isLoading` is true, then using the existing markdown path after streaming finishes. |
| H2 — Deep reactivity on messages array | Not confirmed as current bottleneck | Tool update stream completed in 58.55–62.58 ms total and the test asserts non-target sibling tool-call object identity is preserved. | No `shallowRef`/store split applied. |
| H3 — Streaming text concatenation cost | Not confirmed as current bottleneck | After removing markdown parsing from the hot streaming path, 100 content appends plus DOM updates completed in 131.43 ms total / 1.31 ms average. | No chunk-array rewrite applied. |
| H4 — ContentBlock dispatcher repeats expensive work | Not confirmed for target scenarios | Warmed 2000-token markdown with an embedded code block rendered in 113.07 ms, under the 1000 ms harness budget. | No `v-memo`, async renderer split, or block-hash cache applied. |
| H5 — Auto-scroll thrashing | Not measured by jsdom layout | The headless harness does not perform real layout, so it cannot quantify scroll layout thrash. Since H1 brought max chunk render below 16 ms, no scroll throttling was applied in this task. | Leave for a browser/CDP layout-specific follow-up if field reports persist. |
| H6 — Pinia subscriber spam | Not confirmed as current bottleneck | Render-trigger probe stayed at 0 for the host component in the focused harness while the child tree updated within budget. | No Pinia subscription changes applied. |
| H7 — IPC / ACP layer batching | Not confirmed as current bottleneck | The acp-ui-style immediate dispatcher reference completed 100 chunks in 0.09–0.10 ms, and our full render path is now under frame budget without batching. | No batching/coalescing applied; avoids adding visible latency. |

## User-visible trade-off

During an active assistant stream, markdown syntax is displayed as escaped plain text instead of fully rendered markdown. Once the stream completes (`isLoading` becomes false), the existing `ChatContentBlocks` / `KnowledgeNoteRenderer` markdown pipeline renders the final content. This intentionally trades progressive markdown formatting for frame-safe streaming; maximum additional latency to final formatted markdown is the remaining duration of the active turn, not an added timer.

## Commands run

```text
npx vitest run src/composables/__tests__/agents-perf.test.ts
```

Latest passing metrics:

```text
[agents-perf] chunks=100 total=131.43ms max=7.07ms avg=1.31ms renderTriggers=0 acpUiReference=0.10ms
[agents-perf] toolUpdates=50 total=62.58ms renderTriggers=0
[agents-perf] bigMessageTokens=2000 total=113.07ms
```
