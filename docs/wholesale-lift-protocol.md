# Wholesale Lift Commit Hygiene Protocol

## Purpose

Use this protocol for any future wholesale-lift task: adopting a new upstream library, component, dispatcher, protocol surface, or other source artifact by copying the upstream implementation into Maenifold before adapting it.

The goal is a clean, reviewable history where reviewers can answer three separate questions without reverse-engineering a mixed diff:

1. What exactly came from upstream?
2. What was the minimum host adaptation required for Maenifold to compile and run it?
3. What Maenifold-specific functionality was layered on top?

This protocol documents the implicit constraints used during the ACP UI lift case study in `done/T-244.71.md`.

## Three-commit pattern

| Commit | Name | Scope |
|---|---|---|
| C1 | lift-only | Files copied byte-for-byte from upstream, plus license/attribution headers only. No logic changes. No Maenifold-specific edits. No "while we're here" cleanup. |
| C2 | adapt-only | Minimum required to compile in our host: Tauri to Electron IPC, design tokens, and LOC splits per audit rules. No new features. No Maenifold-extension hooks. Each non-trivial adaptation gets a one-line comment justifying it. |
| C3 | extension-only | Maenifold-specific functionality added via slots, hooks, or wrappers. Lifted code stays unmodified. |

The commits are intentionally narrow. If a change cannot be explained by the declared commit name, it belongs in a different commit or a separate task.

## What NOT to put in each commit type

### C1: lift-only

C1 MUST NOT contain:

- Any inert hooks.
- Any Maenifold-specific slots.
- Any IPC wiring changes.
- Any new capabilities not present in upstream.
- Any design-system conversion.
- Any host-environment adaptation.
- Any "cleanup", reshaping, renaming, or speculative simplification.

C1 is provenance. It exists so reviewers can compare lifted files against upstream and trust that the only intentional differences are path placement plus license or attribution headers.

### C2: adapt-only

C2 MUST NOT contain:

- New IPC channels not present in upstream.
- OS-shell or file-opening capabilities, including `shell.openPath`.
- Absolute-path opening surfaces.
- Dispatcher modifications that get later deleted.
- Maenifold-specific feature hooks or extension slots.
- New product behavior beyond the minimum required to compile and run in the Maenifold host.

C2 is host adaptation. It may translate Tauri IPC to Electron IPC, convert raw styles to Maenifold design tokens, and split files to satisfy audit rules. It is not a feature commit.

### C3: extension-only

C3 MUST NOT modify lifted code in place.

Extensions wrap or compose; they never patch the lifted artifact directly. Use slots, hooks, wrappers, adapter files, or companion extension modules so the upstream lift remains auditable and future upstream comparisons stay clean.

## Explicit prohibition

Any OS-shell, file-opening, absolute-path-opening, or new privileged-IPC capability MUST NOT be introduced as part of a lift unless the lift task body explicitly includes a permission-gated design for that capability.

Examples of prohibited lift-time capabilities unless explicitly authorized by the task:

- `shell.openPath`
- Opening arbitrary absolute paths supplied by an agent, tool result, or upstream payload.
- New Electron IPC channels that expose host filesystem or OS-shell behavior.
- Dispatcher paths that convert upstream metadata into privileged host actions.

If a wholesale lift reveals that such a capability might be useful, do not smuggle it into C2 or C3. Keep the lift clean and require an explicit permission-gated design in the task body before adding the capability.

## Red-team audit guidance

Red-team review should separate two audit dimensions:

| Dimension | Question | Verdict impact |
|---|---|---|
| Live-tip safety | Does the final development tip pass the task acceptance criteria and avoid unsafe live behavior? | PASS or FAIL criterion. |
| Intermediate-history hygiene | Did each interim commit stay within its declared scope? | Informational caveat when live tip is safe; document it for future sprints, but do not block solely on this dimension if live-tip safety passes. |

Intermediate-history hygiene still matters. It is how future wholesale lifts stay reviewable and how reviewers catch drift before mixed commits hide source provenance, host adaptation, and Maenifold extensions in the same diff.

## References

- Case study: `done/T-244.71.md`.
- Original lift pattern in `done/T-244.71.md`:
  - C1 lift verbatim: `imported acp-ui@<sha>`.
  - C2 adaptation: Tauri to Electron IPC, design tokens, and LOC splits.
  - C3 Maenifold extension: slots and hooks for Maenifold-specific behavior.
- Drift incidents cited by `todo/T-304.7-245.2-acp-ui-lift-commit-hygiene.md`:
  - `8b0c6a46b` (`adapt for Maenifold`) added the inert `ToolCallCard` details slot even though slots and hooks belonged in the extension commit.
  - `92db1c47a` (`Maenifold extensions via slots/hooks`) briefly introduced `shell.openPath` IPC and absolute-path opening before `29eb0317b` removed it.
  - `92db1c47a` modified the lifted dispatcher artifact before it was later deleted.
  - `29eb0317b` (`fix(desktop): T-244.71 PM rework`) removed the unsafe and dead-code drift by deleting the dead dispatcher, wiring `current_mode_update` in the live path, and dropping `shell.openPath`.
