# RALPH Thoughts

Handoff from this iter to the next. Replace at iter end.

## Iter close 2026-05-15 — sprints 320 + 324 shipped

**Working tree clean. dev branch up to date.**

Backlog: 80 todo / 1170 done (was 76 / 1164 at iter start; net +6 done = 4 shipped tasks + 2 new follow-up tasks filed during work).

## In flight at handoff

Nothing in flight. All dispatched workers landed + audited + merged + gate-advanced.

## What landed this iter

- **Sprint 324** (config env passthrough portability): T-324.0 umbrella, T-324.1 schema (envFromFile + envFromCommand), T-324.2 adapter resolution. Pattern matches OpenAI Codex `[auth] command/args` + Claude Code `apiKeyHelper`. End-to-end verified: AcpExec → Codex returns model output for gpt-5.4 / gpt-4o.
- **Sprint 320** (embedding UI fix): T-320.0 umbrella, T-320.1 backend vec0 load before DROP, T-320.1.1 frontend confirm-dialog race fix. CDP UAT verified end-to-end model swap with toast surfacing the backend message + daemon-restart prompt.
- **Constitution edits**: UAT in scope (was out-of-scope); two-file ralph state model formalized (learnings APPEND, thoughts REPLACE); README + apps/desktop/README + CHANGELOG document the new envFromFile/envFromCommand schema.

## Stale items still in todo (next-iter triage candidates)

- **T-322.35** — codify-CDP-bound-skip-rule. Made stale by the UAT-restriction removal. Close as superseded.
- **UAT-only items aging in todo for sprints 301, 302, 303, 304** — 12+ items. Per the new "UAT in scope" rule, these are now actionable via CDP per `scripts/uat-launch.sh`. Triage at iter open: which are still relevant after sprints 320 + 324 changes? Some may be obsolete.
- **T-309.3** — accepted-residual-risk per cargo-cult-security P0. Migrate to `blocked: true` in frontmatter once T-310.1 lands.

## Open product gaps surfaced this iter

- **Daemon-deploy script untested per prior iter-19 thoughts** — still untested. Verify the happy path early next iter.
- **Embedding model swap UX**: backend now correctly clears vec0 tables and signals `reembedRequired: true`. Frontend toast says "Restart maenifold to load the new model" but the daemon doesn't restart automatically. Next-iter candidate: explicit "Restart now" button OR auto-restart on activate (with confirmation dialog warning).
- **HFS+ case-insensitive risk**: this iter hit the `README.md` vs `readme.md` edge case. If it recurs, file a `make audit` lint that scans for case-mismatch staging issues.

## Suggested entry point for next loop

Per "lowest-numbered first" — start with `T-301.21-sprint-300-pipeline-uat-consolidated.md` and the other sprint 30x UAT items. The UAT-restriction removal makes them actionable now.
