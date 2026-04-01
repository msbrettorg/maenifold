---
id: T-80.17
title: Bound Relay MPSC Channel
status: done
pipeline: sdd
requirement: FR-32
---

# Bound Relay MPSC Channel

**Requirement**: FR-32-SEC
**Traceability**: TC-SEC.3

## Objective

The `mpsc::unbounded_channel()` in `acp/relay.rs` between the WebSocket relay and worker can buffer indefinitely if the browser stalls or disconnects slowly. Switch to a bounded channel with backpressure.

## Task

- **Verdict**: REWRITE
- In `crates/maenifold-core/src/acp/relay.rs`, find `mpsc::unbounded_channel()`
- Replace with `mpsc::channel(256)` (or similar reasonable bound)
- Handle the `SendError` / channel-full case gracefully (drop oldest or return error to sender)
- Ensure the worker loop handles the bounded channel correctly

## Acceptance Criteria

1. `unbounded_channel` replaced with bounded `channel(N)` where N is documented
2. Channel-full case handled without panic
3. `cargo check -p maenifold-core --features web` succeeds

## Scoped Test Command

```bash
cargo check -p maenifold-core --features web
```

## SWE Remediation

**Status**: ✅ Complete
**File changed**: `crates/maenifold-core/src/acp/relay.rs`

### Changes Made

1. **Channel creation** (lines 109-112): Replaced both `mpsc::unbounded_channel()`
   calls with `mpsc::channel::<WorkerCommand>(256)` and
   `mpsc::channel::<WorkerEvent>(256)`. Capacity of **256** documented inline
   with a comment referencing FR-32-SEC / TC-SEC.3.

2. **Command channel (socket → worker) – `try_send` with backpressure**:
   All three `command_tx.send(…)` call sites in the WebSocket handler now use
   `command_tx.try_send(…)` with a three-arm match:
   - `Ok(())` — command queued, continue.
   - `TrySendError::Full(_)` — sends an `"error"` frame (`"Server busy –
     command backlog full"`) back to the WebSocket client and `continue`s the
     loop (non-panicking, non-blocking).
   - `TrySendError::Closed(_)` — worker has exited; `break` the socket loop.

3. **Event channel (worker → socket) – async backpressure**:
   All `event_tx.send(…)` calls inside the async worker (`run_agent_worker`,
   `handle_worker_command`) now use `.send(…).await`, providing natural
   backpressure if the socket handler falls behind.

4. **Pre-runtime error path**: The single `event_tx.send(…)` inside
   `spawn_agent_worker` (before the Tokio runtime exists) uses
   `event_tx.try_send(…)` since no async context is available.

5. **Type signatures updated**: `spawn_agent_worker`, `run_agent_worker`, and
   `handle_worker_command` now accept `mpsc::Sender` / `mpsc::Receiver` instead
   of their `Unbounded` counterparts.

6. **`text_rx` left unchanged**: The `mpsc::UnboundedReceiver<String>` from
   `AcpManager::take_text_stream()` is an internal AcpManager channel, outside
   this spec's scope.

### Verification

```
$ cargo check -p maenifold-core --features web
   ...
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.13s
```

Zero new warnings. All pre-existing warnings are unrelated (unused import
`Serialize`, dead code in `WorkerEvent::UnexpectedExit`, etc.).

## Red-team Audit

**Auditor**: red-team agent
**Verdict**: ✅ PASS

### Evidence Checked

| Check | Result |
|---|---|
| `unbounded_channel()` removed from `relay.rs` | ✅ `grep -rn 'unbounded_channel' relay.rs` returns 0 matches |
| Replaced with `mpsc::channel::<WorkerCommand>(256)` | ✅ Line 111 |
| Replaced with `mpsc::channel::<WorkerEvent>(256)` | ✅ Line 112 |
| Capacity documented with inline comment | ✅ Lines 109-110 reference FR-32-SEC / TC-SEC.3 |
| Channel-full handled without panic | ✅ See analysis below |
| Function signatures updated to bounded types | ✅ `mpsc::Sender`/`mpsc::Receiver` used |
| `cargo check -p maenifold-core --features web` | ✅ Compiles (exit 0) |

### Backpressure Analysis

**Command channel (socket → worker) — `try_send` path:**

Three call sites at lines 145, 161, 177 all use a three-arm match:
- `Ok(())` → continue
- `TrySendError::Full(_)` → sends error JSON frame to WebSocket client ("Server busy – command backlog full"), then `continue` (non-blocking, non-panicking)
- `TrySendError::Closed(_)` → `break` (worker exited, socket loop terminates cleanly)

**Event channel (worker → socket) — async `.send().await` path:**

All `event_tx.send(…).await` calls (lines 285, 288, 323, 327, 334, 336, 347, 350) use the async `.send()` which provides natural backpressure: the worker blocks if the socket handler is slow. All return values are discarded with `let _ =`, so a closed receiver does not panic — it simply drops the event, and the worker will discover the closed channel on the next loop iteration.

**Pre-runtime edge case (line 251):**
Uses `try_send` (not `.await`) because no async runtime exists yet. `let _ =` discards any `Full` or `Closed` error. Correct — at this point the session is aborting anyway.

### Residual `Unbounded` types — intentionally out of scope

`mpsc::UnboundedReceiver<String>` appears at lines 271 and 310 for `text_rx`. This is the `AcpManager::take_text_stream()` internal channel and is **correctly identified as out of scope** in the SWE remediation. It is an internal API surface between the ACP manager and the relay worker, not the browser-facing channel that this spec targets.

### Panic surface audit

`grep -rn 'panic\|unwrap\|expect' relay.rs` found:
- Line 27: `serde_json::to_string(…).unwrap()` — serializing a `serde_json::json!()` literal, which is infallible. No real panic risk.
- Line 119: `.unwrap_or_default()` — safe fallback.
- Lines 159, 176: `.unwrap_or(".")`, `.unwrap_or_default()` — safe fallbacks.
- No `panic!()` or `unreachable!()` macros found.

### Observations (non-blocking)

- The capacity of 256 is reasonable for a single-user local dev tool. If this ever becomes multi-tenant, it should be made configurable.
- The `Full` error path sends a human-readable error message to the WebSocket client. This is appropriate for a dev tool but would want rate-limiting in a production context to avoid the error frame itself becoming a DoS amplification vector.

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
| No `unbounded_channel()` constructor in `relay.rs` | `grep -rn 'unbounded_channel' relay.rs` → 0 matches | ✅ |
| Command channel bounded at 256 | Line 111: `mpsc::channel::<WorkerCommand>(256)` | ✅ |
| Event channel bounded at 256 | Line 112: `mpsc::channel::<WorkerEvent>(256)` | ✅ |
| Channel-full handled without panic | `try_send` with `Full` → error frame to WS client, `Closed` → break | ✅ |
| Compiles with `--features web` | exit 0 | ✅ |

### Residual `UnboundedReceiver<String>` — Correctly Scoped Out

Lines 271 and 310 reference `mpsc::UnboundedReceiver<String>` as a **type annotation** for `text_rx`, which is a channel owned by `AcpManager` (the upstream ACP SDK). No `unbounded_channel()` constructor call exists in this file. The spec targets the relay's own command/event channels — those are now bounded. No action needed.

### Collateral Impact

- No new warnings introduced.
- Function signatures updated consistently (`Sender`/`Receiver` replace `UnboundedSender`/`UnboundedReceiver` for command/event channels).
- Backpressure semantics are correct: socket→worker uses non-blocking `try_send`, worker→socket uses async `.send().await`.

### Red-team Findings Addressed

Red-team confirmed all 3 `command_tx` call sites and all 8 `event_tx` call sites handle errors without panicking. Panic surface audit found only safe `.unwrap()` on infallible serialization. No blocking findings.
