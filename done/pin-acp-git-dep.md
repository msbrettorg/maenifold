# Pin agent-client-protocol Git Dependency

**Requirement**: FR-32-SEC
**Traceability**: TC-SEC.1

## Objective

The `agent-client-protocol` crate is referenced as a raw `git =` dependency with no pinned `rev =` or `tag =`. This is a supply chain risk — the dependency can change between builds.

## Task

- **Verdict**: REWRITE
- In `crates/maenifold-core/Cargo.toml`, find the `agent-client-protocol` git dependency
- Add a `rev = "<current_commit_sha>"` pin to lock it to the exact commit currently in Cargo.lock
- Extract the current pinned rev from `Cargo.lock` to ensure consistency

## Acceptance Criteria

1. `agent-client-protocol` dependency has `rev = "..."` in Cargo.toml
2. The rev matches what's already resolved in Cargo.lock
3. `cargo check -p maenifold-core --features web` succeeds

## Scoped Test Command

```bash
cargo check -p maenifold-core --features web
```

## SWE Remediation

**Status**: ✅ COMPLETE

### Changes Made

- **`crates/maenifold-core/Cargo.toml`** — Added `rev = "a0917aafe98c95d8e3654d5c441e0cd347df654f"` to the `agent-client-protocol` git dependency, pinning it to the exact commit previously resolved in `Cargo.lock`.

### Before

```toml
agent-client-protocol = { git = "https://github.com/agentclientprotocol/rust-sdk", optional = true }
```

### After

```toml
agent-client-protocol = { git = "https://github.com/agentclientprotocol/rust-sdk", rev = "a0917aafe98c95d8e3654d5c441e0cd347df654f", optional = true }
```

### Verification

| Acceptance Criterion | Result |
|---|---|
| `agent-client-protocol` has `rev = "..."` in Cargo.toml | ✅ |
| Rev matches Cargo.lock resolved SHA | ✅ `a0917aafe98c95d8e3654d5c441e0cd347df654f` |
| `cargo check -p maenifold-core --features web` succeeds | ✅ |

## Red-team Audit

**Auditor**: red-team agent
**Verdict**: ✅ PASS

### Evidence Checked

| Check | Result |
|---|---|
| `Cargo.toml` has `rev = "a0917aafe98c95d8e3654d5c441e0cd347df654f"` | ✅ Present on the `agent-client-protocol` line |
| `Cargo.lock` resolved source contains the same SHA | ✅ `source = "git+...?rev=a0917aafe98c95d8e3654d5c441e0cd347df654f#a0917aafe98c95d8e3654d5c441e0cd347df654f"` |
| SHA in Cargo.toml matches SHA in Cargo.lock (both query param and fragment) | ✅ Exact match |
| `cargo check -p maenifold-core --features web` | ✅ Compiles (exit 0, 0 new warnings from this change) |

### Attack Surface Review

- **Supply chain pin is effective**: With `rev =` present, `cargo update` cannot silently advance to a newer commit on the default branch. An attacker who pushes a malicious commit to `main` of the upstream repo will not affect this build.
- **No weakening vectors found**: The dependency remains `optional = true` (gated behind `web`), so builds without `--features web` are unaffected.

### Observations (non-blocking)

- The pin uses a full 40-character SHA, not a short prefix — good; short prefixes are vulnerable to collision attacks.
- Consider adopting a `tag =` pin in the future if the upstream crate publishes tagged releases, for better auditability.

## Blue-team Verification

**Verifier**: blue-team agent
**Verdict**: ✅ PASS

### Compilation Gate

```
cargo check -p maenifold-core --features web
  → exit 0, 4 pre-existing warnings (none from this change)
```

### Acceptance Criteria Verified

| Criterion | Evidence | Result |
|---|---|---|
| `rev = "..."` present on `agent-client-protocol` | `Cargo.toml:16` — `rev = "a0917aafe98c95d8e3654d5c441e0cd347df654f"` | ✅ |
| Rev is full 40-char SHA (not short prefix) | 40 hex characters confirmed | ✅ |
| Compiles with `--features web` | exit 0 | ✅ |

### Collateral Impact

- No new warnings introduced by this change.
- No other dependencies modified.
- `Cargo.lock` remains consistent (no re-resolve triggered).

### Red-team Findings Addressed

Red-team raised no blocking findings. Non-blocking suggestion (migrate to `tag =` when upstream tags) acknowledged — no action required now.
