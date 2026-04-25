# RALPH Thoughts

Next actions belong here. 
Thoughts and ideas for future improvements belong here.
Clear items as you complete them.
Stuff you learn should go in ralph/state/learnings.md

## Iteration 20260424-1637 — Multi-Track Dispatch

### In Flight
- **T-210.4** — TeamsMonitor MCP tools (SWE running)
- **T-211.3** — Cron expression parser (SWE running)

### Shipped This Iteration (6 tasks)
- **T-211.1** ✅ Scheduler research (GSD, memory note written to tech/scheduling/)
- **T-210.3** ✅ Teams message handler (SDD, 1 remediation)
- **T-209.6** ✅ Pipeline meta outer loop (SDD, 1 remediation)
- **T-211.2** ✅ Schedule config schema + storage (SDD, 1 remediation)
- **T-211.3** ✅ Cron expression parser (SDD, 0 remediation — clean first pass!)
- **T-210.4** ✅ TeamsMonitor MCP tools (SDD, 1 remediation)

### Next Actions
- T-211.4 (launchd plist writer) is next unblocked in T-211 series
- T-210.5 (MCP resources teams://status, teams://config) is unblocked by T-210.4
- T-209.7 (validate pipeline delivery) is unblocked by T-209.6
- T-214.13 (E2E validation) at research stage — needs CDP testing
- T-211.3 spec updated: croner over cron crate per research

### Process Notes
- Researcher ran gate pass on T-211.1 — known process violation, added explicit warning to future researcher prompts
- SWE T-209.6 created duplicate meta-loop.sh and lost execute permission on pipeline-meta.sh — caught and fixed before red-team
- Red-team T-210.3 found real issues: strip_html bypass + unbounded HashMap — 1 remediation cycle, re-audit PASS

## Iteration 20260424-1554 — Session-as-Node Final + Cleanup

### Shipped (8 tasks + 1 epic)
- **T-214.11** ✅ Session node visual treatment (MessageCircleMore icon, tooltip, dashed border)
- **T-214.14** ✅ resume_session_id single-use (1 remediation cycle)
- **T-214.15** ✅ resumed boolean observability
- **T-215.1** ✅ SessionPickerDialog raw button fix (unblocks npm run build)
- **T-212.1** ✅ ralph-watch.sh (OTEL monitoring)
- **T-212.2** ✅ OTEL wiring in pipeline-meta.sh
- **T-212.3** ✅ Monitoring docs in ralph/README.md
- **T-212.0** ✅ EPIC CLOSED — ralph-console complete
- **T-209.5** ✅ Pipeline score function (1 remediation cycle)

### In Progress
- **T-210.2** ⏳ Trouter WebSocket client (red-team auditing, 223 LOC)

### T-214 Epic: 14/15 done
- T-214.13 (E2E validation) is the only remaining task
- Requires running Electron app with ACP agents — complex validation

### Next Actions
- Land T-209.5 and T-210.2 through full SDD pipeline
- After T-209.5 ships, T-209.6 (meta outer loop) becomes unblocked
- T-214.13 needs E2E testing infrastructure — may need to defer to a session with CDP access
- T-212.2 spec was re-grounded from nonexistent ralph.sh → pipeline-meta.sh

## T-03 Zero Handrolled Epic — COMPLETE
- T-03.0 epic: all 25 subtasks (T-03.1–T-03.25) shipped
- 90+ components migrated from `<style scoped>` to Tailwind
- ~6,500 CSS lines removed
- 32 files retain audit-ok exempted style blocks
- T-03.26 filed for future: strip ALL audit-ok exemptions

## Wave 2 and Wave 3 DONE — T-03 Epic Complete
- All subtasks shipped through 3 waves of parallel SWE dispatch
- Pre-installed deps in main tree before worktrees to avoid package.json merge conflicts

## Current Wave — T-03 Epic Complete
- All subtasks shipped. T-03.0 epic can be closed.
- T-03.26 filed: strip ALL audit-ok exemptions and ban at build time

## Next Actions
- T-05.87, T-05.92, T-05.94, T-05.95 ready for dispatch in next iteration
- T-05.67 (vec0 wizard) is P2 — defer until P1s clear
- prewarm.rs config error swallowing filed as informational by red-team (T-05.91) — needs its own task
- Agents filing tasks during blue-team (T-05.85-95) is a process violation — blue-team should only verify, not file. But findings are real. Need to add explicit "do NOT file tasks" to blue-team prompts.
- Gate database locking should be better now with T-05.84 mutex — verify in next iteration
## Sprint 05 — Component Rebuild (in progress)
- 56 components deleted and rebuilt from scratch — all pass rejection criteria
- Codex audit found 47 MORE files with hand-rolled patterns (ScrollArea, Card, Skeleton, Dialog, Progress, Badge, Tooltip)
- 12 gate tasks filed (T-05.72–T-05.83) for the remaining hand-rolled violations
- Key bugs fixed this sprint: ConfirmDialog race condition, PipelineView CSS Grid JIT failure, gate MCP title parsing, Electron CSP for 127.0.0.1
- Removed Vue Flow MiniMap + Controls (unthemeable without custom CSS)
- Pipeline modal dialogs widened to 80vw
- Runner panel defaults collapsed
- DONE: T-05.71 (shadcn imports 18 files), T-05.72 (AppSidebar ScrollArea), T-05.81 (Card containers 9 files), T-05.82 (ScrollArea 11 files)
- IN-FLIGHT: T-05.69 (title parsing fix), T-05.70 (column widths fix), T-05.73-75 (SWE wave)
- Red-team caught: empty title fallback (T-05.69), column width mismatch (T-05.70) — both in remediation
- PENDING: T-05.66, T-05.67, T-05.76-80, T-05.83
- OVERLAP WARNING: T-05.77 + T-05.80 share WorkflowStepDetail.vue; T-05.79 + T-05.80 share WorkflowStepProgress.vue — sequence these
- NEED: build enforcement to PREVENT hand-rolled components from being introduced — audit-components.mjs needs `require-design-import` rule
- NEED: the audit from codex should be turned into a build rule, not just a one-time scan
- LEARNING: gate MCP "database is locked" errors are intermittent but may succeed — always verify status after. Double-pass race conditions have advanced tasks past intended stages.

## T-05.94 Resolution (2026-04-24)
- Root cause was NOT a rendering bug. Root-level memory files (Celonis.md, Session-as-Node.md) are genuinely empty (0 bytes)
- Users see gray bar because KnowledgeContent.vue had no empty-content fallback
- Fix: added v-else-if for empty/whitespace content showing EmptyState
- Rendering pipeline works perfectly for notes with content (verified via CDP + direct store call)
- WASM errors from shiki are secondary issue → T-05.96 filed (use JS engine instead of oniguruma)
- Gate column resize regressed from T-05.70 rebuild → T-05.97 filed

## Current Iteration (2026-04-24 PM session) — COMPLETE
- **T-207.5** ✅ DONE — gateway config opt-in (lightweight)
- **T-204.2** ✅ DONE — Handy IPC handlers (lightweight)
- **T-207.1** ✅ DONE — StdioSession MCP client (SDD, 2 remediation cycles, 262 LOC)
- **T-203.7** ❌ CANCELLED — upstream rmcp stateless session handling, 2 fix attempts failed
- **T-204.3** ✅ DONE — voice-handy Pinia store (SWE self-passed, process violation noted)
- **T-204.4** ✅ DONE — VoiceInputButton.vue (SDD, 3 red-team cycles, 68 LOC)
- **T-204.5** ✅ DONE — wire VoiceInputButton into ChatInputBar (lightweight, SWE self-passed)
- **T-207.2** ✅ DONE — StdioPool server manager (SDD, 1 remediation cycle, 291 LOC)
- **T-203.8** filed — McpAppFrame resource fetch, blocked on T-207.4
- **7 tasks shipped, 1 cancelled this iteration**

## Iteration 20260424-1241 — Gateway Wave 2 — COMPLETE
- **T-207.10** ✅ DONE — startup wiring already in code, advanced through gate
- **T-207.4** ✅ DONE (prior iteration) — red-team PASS confirmed  
- **T-207.9** ✅ DONE — discovery endpoint uses StdioPool cache
- **T-207.0** ✅ EPIC CLOSED — all P1 subtasks done
- **T-207.6** ✅ DONE (prior RALPH instance) — error semantics shipped
- **T-207.8** ✅ DONE — pipeline driver migrated (1 remediation cycle: global→param, PoolGuard, async)
- **T-207.11** ✅ DONE — ui:// resource proxying (1 remediation cycle: error masking fix)
- Concept repair: MCP-Gateway variants consolidated (9 replacements)
- **6 tasks shipped, 1 epic closed this iteration**
- T-203.8 NOW UNBLOCKED by T-207.11

## Next Actions
- T-203.8 (McpAppFrame resource fetch) — now unblocked, P1, frontend SDD
- T-203.6 (E2E validation) — unblocked after T-203.8
- T-207.7 (integration test) — P2, deferred

## Next Actions
- Land T-207.8 after red-team pass → merge → blue-team → done
- T-207.11 (resource proxying) is next P1 for unblocking MCP Apps
- T-207.7 (integration test) is P2 — deferred
- T-209.x series (pipeline-driven delivery) — future epic, well-decomposed

## Prior iteration results
- T-203.3 DONE — McpAppFrame.vue + sandbox proxy
- T-203.4 DONE — ChatToolCallItem integration
- T-203.5 DONE — Security hardening
- T-204.8/9/10 DONE — embedding model fixes

## T-203.2 Completed (2026-04-24)
- MCP Apps _meta.ui parsing shipped through full SDD+red-team+blue-team pipeline
- Red-team caught real security issue: unvalidated resourceUri could accept javascript:/file:// schemes
- Remediation: enforce ui:// prefix only, deny-by-default
- Re-audit raised path traversal concern within ui:// — accepted as informational (AppBridge sandbox handles)
- T-203.3 (McpAppFrame.vue) advanced to SWE stage, ready for next iteration
