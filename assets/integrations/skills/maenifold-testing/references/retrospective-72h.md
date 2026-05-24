# 72-Hour Retrospective (2026-04-29 → 2026-05-01)

Historical lessons from the first major incident cluster during the Electron+Vue front-end era. Every lesson below was encoded as a rule in `AGENTS.md` at the time. The front-end UI was ripped in May 2026 (vanilla MCP-server baseline); UI-specific lessons (5, 8) are retained here as historical record only and no longer apply to the current backend-only stack.

Sources: `memory://incidents/20260501-*`, `done/T-244.*`, `done/T-246.*`, `done/T-247.*`, `done/T-248.*`, `done/T-243.*`.

## 1. Local-first storage is not negotiable

**Incidents:** T-248.13 (phantom delete RCA), T-248.16 (pipeline list derivation gap), T-248.19 (OneDrive dataless detection), T-248.23 (DB fallback).

OneDrive on macOS materializes `~/maenifold/memory/pipelines/**/*.md` as `compressed,dataless` placeholders. Hot-path code that called `fs::read_to_string` on these files timed out, and `list_pipelines` wrapped the failure in `else { continue; }` — silently dropping pipelines from the UI.

**Lesson:** Never wrap a read with silent `continue;`. Read failures on hot-path authority surfaces MUST degrade into a read-model fallback AND emit an explicit `unreadable: true` / `degraded: { reason: "..." }` flag. See `memory://research/architecture/statefidelity-external-patterns-for-maenifold-may-2026`.

## 2. Counts-only success payloads are unverified

**Incidents:** T-248.13, T-248.24, T-248.6.

Daemon returned `{ cleanedUpFiles: N, skippedFiles: 0 }`. PM reported "2 deleted." Files survived. Forensic note `memory://incidents/20260501-pm-deleted-pipelines-forensic` proved 35 surviving artifacts.

**Lesson:** Destructive ops must return the resolved file list AND fail-closed on post-unlink verification. Counts-only = UNVERIFIED.

## 3. The PM is a meta-alignment failure mode

**Incidents:** T-248.22, `memory://think-tank/ai-alignment-failure/the-metaalignment-problem`.

User said "fix the alignment problem where you don't listen." PM scoped to "fix the pipeline delete API." Other instances: "running" → "all", "highest priority" → "lowest ID", "cancel running" → "delete completed".

**Lesson:** When the user says a literal word, execute on that literal word. `running ≠ completed`. `cancel ≠ delete`. `highest ≠ lowest`.

## 4. The deploy procedure missed an entire binary

**Incidents:** T-248.26.

`maenifold-pipeline-driver` is a SEPARATE binary. Standard "rebuild + restart daemon" only built `maenifold-cli`. Two blue-team cycles failed because driver ran April 30 code while daemon ran May 1 code.

**Lesson:** Verify the running binary, not the source tree. After any deploy: (1) `lsof -p <pid> | grep txt` shows expected inode, (2) `LC_ALL=C grep -ac "<literal>" <binary>` returns ≥1.

## 5. Frontend filters silently exclude valid backend data

**Incidents:** T-243.46 (5 SWE iterations), T-248.28 v1.

Backend persists `acpSessionId` to pipeline frontmatter. Frontend filters with `node.type === 'prompt'` — workflow + thinking nodes excluded.

**Lesson:** Trace before fixing, spec full-stack from the start. Write the FULL data path (source → persist → read → API → selector → render) BEFORE scoping the fix.

## 6. Gate state lives on disk; operate through the Gateway

**Incidents:** T-244.107, T-244.108, T-246.6, T-247.10.

Agents tried to mutate gate state directly: `mv` between todo/done, hand-editing markdown, deriving from `.gate/gate.db`.

**Lesson:** Never `mv`, `cp`, `git mv`, or hand-edit task files. Gate ownership is non-negotiable. Task files require YAML frontmatter with `id:` matching the filename stem.

## 7. Audit baselines are evidence

**Incidents:** T-243.18, T-244.11x, T-248.14.

`audit-crates` violations grew from 13 to 15 silently. SWEs reported "no new violations" while the baseline crept.

**Lesson:** Every audit failure is a tracked item. The baseline number is part of the spec.

## 8. CDP UAT is the only thing that proves the UI works

**Incidents:** T-243.1, T-243.2, T-243.3, T-243.30, T-244.74-78.

`vue-tsc --noEmit` proved zero. Screenshots proved the renderer rendered. Only `agent-browser` against the packaged app caught the broken behavior.

**Lesson:** Be OCD on every screen. Click every button. File gate items for anything that looks wrong. The packaged app is the only test surface.
