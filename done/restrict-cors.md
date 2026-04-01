---
id: T-80.138
title: Restrict CORS to Localhost
status: done
pipeline: sdd
requirement: FR-32
---

# Restrict CORS to Localhost

**Requirement**: FR-32-SEC
**Traceability**: TC-SEC.2

## Objective

`CorsLayer::permissive()` allows any origin. For a local dev tool this is overly broad. Restrict to localhost origins.

## Task

- **Verdict**: REWRITE
- Find `CorsLayer::permissive()` in the web/API route setup
- Replace with a CORS policy that allows only:
  - `http://localhost:*`
  - `http://127.0.0.1:*`
  - `http://[::1]:*`
- Allow all methods and headers (needed for MCP)

## Acceptance Criteria

1. CORS layer restricts origins to localhost variants
2. MCP HTTP transport still works (methods/headers not restricted)
3. `cargo check -p maenifold-core --features web` succeeds

## Scoped Test Command

```bash
cargo check -p maenifold-core --features web
```

## SWE Remediation

**Status**: COMPLETE

### Changes Made

1. **`crates/maenifold-core/src/mcp/mod.rs`** — Added `pub(crate) fn localhost_cors()` gated behind `#[cfg(any(feature = "http", feature = "web"))]`. Uses `AllowOrigin::predicate` to accept only origins matching `http://localhost[:port]`, `http://127.0.0.1[:port]`, and `http://[::1][:port]`. Methods and headers remain `AllowMethods::any()` / `AllowHeaders::any()` per MCP requirements.

2. **`crates/maenifold-core/src/mcp/http.rs`** — Replaced `.layer(CorsLayer::permissive())` with `.layer(super::localhost_cors())`. Removed unused `tower_http::cors::CorsLayer` import.

3. **`crates/maenifold-core/src/mcp/web.rs`** — Replaced `let cors = CorsLayer::permissive()` with `let cors = super::localhost_cors()`. Removed unused `use tower_http::cors::CorsLayer` import.

### Decisions

- Used a predicate-based origin check (not a static list) to match any port on localhost variants without enumerating them.
- Bare origins without a port (e.g. `http://localhost`) are also accepted for completeness.
- Shared the implementation via a crate-internal function in `mod.rs` to avoid duplication between `http.rs` and `web.rs`.

### Verification

```
cargo check -p maenifold-core --features web  # ✅ passed (0 new warnings)
```

## Red-team Audit

**Auditor**: red-team agent
**Verdict**: ✅ PASS

### Evidence Checked

| Check | Result |
|---|---|
| `CorsLayer::permissive()` removed from `http.rs` | ✅ Zero matches in `grep -rn 'CorsLayer::permissive' crates/maenifold-core/src/` |
| `CorsLayer::permissive()` removed from `web.rs` | ✅ Same grep confirms no occurrences anywhere in `src/` |
| `http.rs` uses `super::localhost_cors()` | ✅ Line 27 |
| `web.rs` uses `super::localhost_cors()` | ✅ Line 40 |
| `localhost_cors()` only allows localhost origins | ✅ See predicate analysis below |
| Methods and headers unrestricted for MCP | ✅ `AllowMethods::any()`, `AllowHeaders::any()` |
| `cargo check -p maenifold-core --features web` | ✅ Compiles (exit 0) |

### Predicate Security Analysis

The `AllowOrigin::predicate` in `mod.rs` (lines 17-26) accepts:

| Pattern | Matches |
|---|---|
| `s == "http://localhost"` | bare localhost (port 80 implicit) |
| `s.starts_with("http://localhost:")` | `http://localhost:<any_port>` |
| `s == "http://127.0.0.1"` | bare IPv4 loopback |
| `s.starts_with("http://127.0.0.1:")` | `http://127.0.0.1:<any_port>` |
| `s == "http://[::1]"` | bare IPv6 loopback |
| `s.starts_with("http://[::1]:")` | `http://[::1]:<any_port>` |

**Bypass vectors tested (all blocked):**

- `http://localhost.evil.com` — Fails: `starts_with("http://localhost:")` requires a colon immediately after `localhost`, and the bare equality check won't match either. ✅ Blocked.
- `http://localhost.evil.com:8080` — Fails: `starts_with("http://localhost:")` would match `http://localhost:` prefix… but `localhost.evil.com:8080` starts with `http://localhost.`, not `http://localhost:`. ✅ Blocked.
- `https://localhost:3000` — Fails: all patterns require `http://` prefix. ✅ Blocked (intentional; the server only binds HTTP on 127.0.0.1).
- `http://127.0.0.2:3000` — Fails: only `127.0.0.1` is matched. ✅ Blocked.
- `null` origin (sandboxed iframe) — Fails: no pattern matches `"null"`. ✅ Blocked.
- Empty/missing `Origin` header — CORS middleware skips; browser same-origin requests still work. ✅ Correct behavior.

### Observations (non-blocking)

- `https://localhost:*` is not allowed. This is correct for the current HTTP-only server, but if TLS termination is ever added, the predicate should be updated.
- The predicate is shared via `pub(crate)`, preventing accidental re-export — good encapsulation.

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
| No `CorsLayer::permissive()` in `src/mcp/` | `grep -rn 'CorsLayer::permissive' src/mcp/` → 0 matches | ✅ |
| `http.rs` uses `localhost_cors()` | Line 27: `.layer(super::localhost_cors())` | ✅ |
| `web.rs` uses `localhost_cors()` | Line 40: `let cors = super::localhost_cors()` | ✅ |
| Predicate restricts to localhost variants | `mod.rs:14-28` — matches `http://localhost[:port]`, `http://127.0.0.1[:port]`, `http://[::1][:port]` only | ✅ |
| Methods/headers unrestricted for MCP | `AllowMethods::any()`, `AllowHeaders::any()` confirmed | ✅ |
| Compiles with `--features web` | exit 0 | ✅ |

### Collateral Impact

- Both `http.rs` and `web.rs` share the same `localhost_cors()` factory — no code path missed.
- No other CORS configuration exists in the crate.
- Unused import cleanup applied (no dead `CorsLayer` imports remain).

### Red-team Findings Addressed

Red-team tested 6 bypass vectors (subdomain spoofing, HTTPS, `null` origin, etc.) — all blocked. No blocking findings raised.
