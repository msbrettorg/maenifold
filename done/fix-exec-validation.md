---
id: fix-exec-validation
status: done
pipeline: sdd
---

# Fix Executable Validation to Use access()

**Requirement**: FR-32-SEC
**Traceability**: TC-SEC.4

## Objective

The agent command validation in `acp/config.rs` checks `mode & 0o111` (any execute bit set) but this doesn't verify the *current user* can execute. A file with mode `0o711` owned by root would pass but fail at runtime.

## Task

- **Verdict**: REWRITE
- In `crates/maenifold-core/src/acp/config.rs`, find the executable check
- Replace the mode-bit check with `std::os::unix::fs::access` or `nix::unistd::access` with `AccessFlags::X_OK`, or use `std::process::Command` with a dry-run approach
- Alternatively, use `libc::access(path, libc::X_OK)` which checks real user permissions
- Keep the existing path-exists and is-file checks

## Acceptance Criteria

1. Executable validation checks real user permissions, not just mode bits
2. Non-executable files correctly rejected
3. `cargo check -p maenifold-core --features web` succeeds

## Scoped Test Command

```bash
cargo check -p maenifold-core --features web
```

## SWE Remediation

**Status**: COMPLETE
**Files changed**:
- `Cargo.toml` — added `libc = "0.2"` to workspace dependencies
- `crates/maenifold-core/Cargo.toml` — added `libc.workspace = true` to dependencies
- `crates/maenifold-core/src/acp/config.rs` — rewrote executable validation

**What changed**:
1. Removed `std::os::unix::fs::PermissionsExt` import and the `mode & 0o111` bit-mask check.
2. Added `std::ffi::CString` import and `libc` dependency.
3. Replaced the mode-bit check with `libc::access(path, libc::X_OK)` which delegates to the POSIX `access(2)` syscall. This checks whether the **real user ID** of the calling process has execute permission on the file, accounting for ownership, group membership, and ACLs — not just whether any execute bit is set.
4. Added a `CString` conversion guard that rejects paths containing interior NUL bytes.
5. Retained the existing `fs::metadata` call for existence / IO-error handling (returns early with clear warnings for missing or inaccessible commands).

**Acceptance criteria verification**:
- ✅ Executable validation checks real user permissions via `access(2)` with `X_OK`
- ✅ Non-executable files rejected (access returns non-zero → agent excluded with warning)
- ✅ `cargo check -p maenifold-core --features web` succeeds with zero new warnings

## Red-team Audit

**Verdict**: ✅ PASS

### Checks Performed

| # | Check | Result |
|---|-------|--------|
| 1 | Old `mode & 0o111` / `PermissionsExt` removed | ✅ grep returns no matches |
| 2 | `libc::access(c_path.as_ptr(), libc::X_OK)` present | ✅ Line 125 |
| 3 | Interior-NUL guard via `CString::new()` | ✅ Lines 112-120 |
| 4 | `libc = "0.2"` in workspace `[workspace.dependencies]` | ✅ `Cargo.toml:47` |
| 5 | `libc.workspace = true` in maenifold-core deps | ✅ `crates/maenifold-core/Cargo.toml:20` |
| 6 | `fs::metadata` existence check retained before access call | ✅ Lines 92-108 |
| 7 | SAFETY comment on `unsafe` block is accurate | ✅ NUL-terminated, read-only syscall, no aliasing |
| 8 | `cargo check -p maenifold-core --features web` | ✅ Exit 0, zero new warnings (4 pre-existing in relay.rs/support.rs) |

### Advisory Findings

**[Low] No `#[cfg(unix)]` platform guard**
`libc::access` and `libc::X_OK` are POSIX-only. The module will fail to compile if the crate is ever targeted to Windows. Acceptable today — this is a macOS/Linux MCP server managing POSIX child processes — but consider adding `#[cfg(unix)]` to `validate_agent_entry` with a Windows stub that skips the check or uses `_access` if cross-platform support is ever needed.

**[Low] TOCTOU between `fs::metadata()` and `libc::access()`**
A file could theoretically change between the two calls. Not exploitable here: both are informational startup-time checks, and the actual execution happens later via `Command::new()` which will independently fail if the binary disappears or becomes non-executable. The window is negligible and the blast radius is a clear warning message, not silent misbehavior.

**[Informational] `access(2)` checks real UID — correct choice**
POSIX `access()` checks the *real* user ID, not the effective UID. This is the right semantic for config validation: it answers "can the human operator execute this?" rather than "can a potentially setuid-elevated process execute this?" No action needed.

No blocking issues found.
