# ADR — Accepted residual risk: compromised-renderer fs/write permission

**Status:** Accepted
**Date:** 2026-05-18
**Decision-maker:** Brett (user)
**Supersedes task:** T-309.3 (was T-245.19 / T-244.45)

## Context

The fs/write permission flow in the Electron app prompts the user before any
agent-driven write. The prompt-and-resolve IPC currently lives on the renderer
side: `window.electronAPI.onWritePermissionPrompt` delivers the request, and
`window.electronAPI.resolveWritePermission` returns the decision.

Red-team identified a compromised-renderer threat: malicious code running
inside the renderer (XSS via untrusted markdown, malicious npm dep, dev-tools
injection) could subscribe to the prompt channel and auto-resolve `allow_once`
before the user sees the dialog, then call `writeTextFile` directly.

## Decision

**Accept the residual risk. Do not move the prompt to a main-owned trusted
surface at this time.**

## Rationale

1. **Defense-in-depth already bounds blast radius.** `requireNonSymlinkWritePath`
   and `requireWriteWithinMemory` (enforced in main) constrain any write —
   approved or auto-approved — to `~/maenifold/memory`. Arbitrary filesystem
   write is not possible regardless of renderer trust.

2. **No active incident.** No XSS, supply-chain compromise, or DOMPurify bypass
   has been observed. The threat is theoretical for a single-user product whose
   threat model is "the user trusts their own machine and the agents they ran."

3. **Cargo-cult-security guardrail.** Per AGENTS.md P0 trigger #5, unrequested
   security work is forbidden. Moving the prompt off the renderer adds Electron
   `dialog.showMessageBox` plumbing or an isolated `webContents` surface, both
   of which introduce build/UX/test surface area for a threat we have no
   evidence of in production.

## Trigger to revisit

Re-open this decision if ANY of:

- A real XSS or supply-chain incident is detected in this product.
- A DOMPurify (or successor sanitizer) bypass is published that affects our
  markdown render path.
- An external security review requires it.
- User explicitly authorizes the hardening work.

## Implementation note

No code change. Path-constraint defenses remain enforced in main:

- `crates/maenifold-core/...` write helpers gate on `requireWriteWithinMemory`.
- Symlink-resolve gate prevents escape via symlinks inside `~/maenifold/memory`.
