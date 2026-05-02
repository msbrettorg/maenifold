# RALPH Testing Cycle

Use this reference when testing/debugging happens inside a RALPH autonomous improvement cycle.

## Required reads

1. Repo-root `AGENTS.md`
2. `ralph/AGENTS.md`
3. `ralph/state/thoughts.md`
4. Relevant entries in `ralph/state/learnings.md`

RALPH work is constrained. Respect the allowed/off-limits paths in `ralph/AGENTS.md`.

## Iteration stance

- One improvement per iteration.
- Prefer simplification over addition.
- Prefer removing dead code over adding new code.
- Use gate as source of truth via the integrated `maenifold.gate` Gateway domain.
- Record reusable lessons in `ralph/state/learnings.md` and update this skill when the lesson changes future testing behavior.

## Quality gates

RALPH's local constitution requires:

```bash
cargo build --release -p <crate>
cargo test -p <crate>
```

Write a `memory://` note for accepted changes when operating under RALPH rules.

## Testing lesson handoff

When a RALPH cycle discovers testing process drift, update the focused file under `assets/integrations/skills/maenifold-testing/references/` rather than expanding `SKILL.md`.
