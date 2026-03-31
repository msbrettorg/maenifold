---
id: frontmatter_sessions
status: done
pipeline: tdd
---

# Test Evaluation: frontmatter_sessions
**File:** `crates/maenifold-core/tests/frontmatter_sessions.rs`
**Test count:** 4
**Evaluated:** 2026-05-28
**Traceability:** T-META-001.6, T-12.6, RTM FR-24.6, RTM NFR-1

---

## Summary

This file claims to cover two distinct behavioral concerns: (1) how `cleanup_abandoned_sessions`
handles sessions with and without a `modified` frontmatter timestamp field, and (2) how
`read_memory` renders `Created:`/`Modified:` lines when timestamps are absent. Both claims are
undermined on inspection.

**The critical problem**: The `cleanup_abandoned_sessions` implementation
(`src/sessions/cleanup.rs`) is entirely DB-driven. Its WHERE clause queries `last_indexed` from
`file_content`; it also checks filesystem `mtime` via `should_preserve_recent_filesystem_edit`.
It **never reads the `modified` frontmatter field** for staleness determination. This makes `ac1`
a test with a false premise, and the "filesystem fallback" framing in `ac2`/`ac3` inaccurate.

**The second problem**: `ac4` asserts `output.contains("Created:")` and
`output.contains("Modified:")`. The `read_memory` implementation always emits those labels in
Markdown mode — rendering `"N/A"` when the frontmatter fields are absent (not filesystem
timestamps). These assertions can never fail for any reasonable code change and are therefore
non-falsifiable. The test name is also wrong: no filesystem timestamps are used.

**Overlap with `session_cleanup_ttl.rs` and `session_cleanup_sweep.rs`**: `ac2` and `ac3` test
the same cleanup scenarios already covered by `active_session_within_default_ttl_is_not_marked_abandoned`
and `active_session_beyond_default_ttl_is_marked_abandoned`, but with weaker assertions (neither
`read_db_status` nor body content are checked) and without the extra coverage value of
`write_raw_markdown` being meaningfully different from `write_session_markdown` for these paths.

---

## Verdicts

| Test Function | Load-Bearing? | Unique? | Assertions Correct? | Name Accurate? | Parameterizable? | Verdict |
|---|---|---|---|---|---|---|
| `fr24_6_ac1_session_cleanup_with_modified_existing_behavior` | Marginally | No | No — premise wrong | No | Yes (age, FM fields) | **REWRITE** |
| `fr24_6_ac2_session_cleanup_without_modified_filesystem_fallback` | Partially | No | Partially | No | Yes (age, FM presence) | **MERGE** |
| `fr24_6_ac3_session_cleanup_abandoned_detected_regardless_of_frontmatter` | Partially | No | Partially | Yes | Yes (age, FM presence) | **MERGE** |
| `fr24_6_ac4_read_memory_displays_filesystem_timestamps_when_frontmatter_missing` | No | No | No — non-falsifiable | No | N/A | **REWRITE** |

---

## Per-Test Analysis

### `fr24_6_ac1_session_cleanup_with_modified_existing_behavior`

**Setup**: Writes a raw markdown file with `modified: <130_min_ago>` in frontmatter. Seeds DB
`file_content` with `last_indexed: <130_min_ago>`, `status: "active"`. Calls cleanup. Expects
`abandoned == 1` and frontmatter `status == "abandoned"`.

**What it claims to test**: That the `modified` frontmatter field is read and used as the
activity timestamp for cleanup decisions (implied by the test name and FR-24.6 comment).

**What it actually tests**: That the DB-driven cleanup query correctly identifies a session whose
`last_indexed` is 130 minutes old as a candidate. The `modified: <stale>` frontmatter field is
never read by the cleanup code — it is cosmetic noise in this test setup.

**Assertion gaps**: No assertion that the `modified` frontmatter field was consulted. No assertion
on `## Session Abandoned` body content. No assertion on DB `status` post-cleanup (`read_db_status`
exists in `session_cleanup_ttl.rs`'s support but isn't used here).

**Risk**: A refactor that removed the `modified`-field read path (if it existed) would not break
this test. A refactor that *added* reliance on `modified` frontmatter would also not be caught
here. The test documents a behavioral claim it cannot enforce.

**Verdict**: REWRITE. Remove the misleading `modified: {stale_modified}` from the raw frontmatter
(it isn't tested). Rename to `session_with_stale_db_timestamp_is_abandoned`. Add
`read_db_status` assertion to match `session_cleanup_ttl.rs` depth. Or, if the *intent* was to
verify that the `modified` field in frontmatter is *not* used to override the DB timestamp,
write an explicit test for that: a session with `modified: <5_min_ago>` in frontmatter but
`last_indexed: <130_min_ago>` in the DB — cleanup should still abandon it.

---

### `fr24_6_ac2_session_cleanup_without_modified_filesystem_fallback`

**Setup**: Writes a raw markdown file with no `modified` field. Seeds DB with
`last_indexed: <5_min_ago>`, `status: "active"`. Calls cleanup. Expects `abandoned == 0` and
frontmatter `status == "active"`.

**What it claims to test**: That when `modified` frontmatter is absent, the system "falls back
to the filesystem" to determine activity (per the name).

**What it actually tests**: That cleanup correctly leaves alone a session with a fresh
`last_indexed` DB timestamp (5 minutes old). No filesystem fallback occurs for *fresh* sessions
— the DB WHERE clause simply doesn't select them as candidates.

**Overlap**: `active_session_within_default_ttl_is_not_marked_abandoned` in
`session_cleanup_ttl.rs` covers the identical behavioral claim with a 30-minute-old session,
using `write_session_markdown` instead of `write_raw_markdown`. The difference in setup path
(`write_raw_markdown` vs `write_session_markdown`) adds no meaningful coverage here because
neither path writes a `modified` frontmatter field in this scenario, and the cleanup code doesn't
read it anyway.

**Assertion gaps**: No `read_db_status` assertion. The only assertion is `abandoned == 0` and the
frontmatter label — weaker than the `session_cleanup_ttl.rs` equivalents.

**Verdict**: MERGE into `session_cleanup_ttl.rs` or replace with a genuinely distinct test that
exercises `write_raw_markdown` in a scenario where that setup path matters (e.g., verifying
cleanup tolerates malformed frontmatter, or tests the actual filesystem-mtime override path via
explicit `filetime` manipulation).

---

### `fr24_6_ac3_session_cleanup_abandoned_detected_regardless_of_frontmatter`

**Setup**: Writes a raw markdown file with no `modified` field. Seeds DB with
`last_indexed: <150_min_ago>`, `status: "active"`. Calls cleanup. Expects `abandoned == 1` and
frontmatter `status == "abandoned"`.

**What it claims to test**: That stale sessions are abandoned even when `modified` frontmatter is
absent. The name is accurate for what it tests.

**Overlap**: `active_session_beyond_default_ttl_is_marked_abandoned` in
`session_cleanup_ttl.rs` covers the same claim (180 minutes, no `modified` FM). That test
additionally asserts `read_db_status`, body content contains `## Session Abandoned`, and that DB
`content` is updated — all absent here.

**Value**: Marginally adds coverage in the sense that `write_raw_markdown` is used instead of
`write_session_markdown`. But the cleanup code reads and re-writes the file via `read_markdown` /
`write_markdown` regardless of how the file was created, so this distinction has no behavioral
meaning for the cleanup path.

**Verdict**: MERGE into `session_cleanup_ttl.rs` with the same setup and the stronger assertion
set already present there. If the `write_raw_markdown` fixture style is intentional (to simulate
non-MarkdownWriter-written files), document that explicitly and add the missing body+DB assertions.

---

### `fr24_6_ac4_read_memory_displays_filesystem_timestamps_when_frontmatter_missing`

**Setup**: Writes a raw markdown file with `type: memory` and `title` but no `created` or
`modified` fields. Calls `memory::read_memory` with `includeChecksum: true`. Asserts the output
doesn't start with `"ERROR:"`, contains `"Created:"`, and contains `"Modified:"`.

**What it claims to test**: That `read_memory` displays *filesystem timestamps* as a fallback
when frontmatter timestamps are missing.

**What it actually tests**: That `read_memory` emits the `Created:` and `Modified:` labels. It
does — always, unconditionally, in Markdown output mode. The labels render `"N/A"` when
frontmatter fields are absent. **No filesystem timestamps are consulted.** See
`src/memory/access.rs` lines ~240–250:
```rust
let created_display = created
    .as_deref()
    .map(format_timestamp_local)
    .unwrap_or_else(|| "N/A".to_string());
let modified_display = modified
    .as_deref()
    .map(format_timestamp_local)
    .unwrap_or_else(|| "N/A".to_string());
```

**Non-falsifiability**: `output.contains("Created:")` can never be false under any code
change that preserves the Markdown output format. This assertion adds zero regression protection.
It would survive even if the date rendering were silently broken (e.g., always emitting "N/A"
when real dates are expected).

**Missing assertions**: If the intent is to verify graceful handling of missing timestamps, the
correct assertion is `output.contains("Created: N/A")` and `output.contains("Modified: N/A")`.
If the intent is to verify that filesystem timestamps *are* used as fallback, that feature doesn't
exist and the test name documents a capability the code doesn't have.

**Uniqueness**: The `read_memory_handles_empty_frontmatter` and
`read_memory_handles_missing_title_in_frontmatter` tests in `memory_tools_move_read_write.rs` also
test graceful `read_memory` behavior on raw files — but neither asserts on `Created:`/`Modified:`
lines. So ac4 is the only test touching timestamp label rendering, but it does so with
non-falsifiable assertions.

**Verdict**: REWRITE. Correct the test name to `read_memory_renders_na_when_timestamps_missing`.
Replace `contains("Created:")` with `contains("Created: N/A")` and similarly for `Modified:`.
This makes the assertion falsifiable — a regression that changes "N/A" to an empty string or
removes the label would be caught. Separately, if filesystem timestamp fallback is a desired
capability (it sounds useful), file a requirement, implement it, and write a real test for it.

---

## Recommendations

1. **REWRITE `ac1`**: The `modified: {stale_modified}` frontmatter field in the test setup is
   noise — the implementation ignores it. Rename the test to reflect what is actually exercised
   (DB-driven cleanup). Either remove the misleading raw `modified` field, or pivot: make ac1 the
   *explicit contract test* that `modified` frontmatter does **not** override a stale DB
   timestamp (i.e., set `modified: <2_min_ago>` but `last_indexed: <130_min_ago>` — cleanup
   should still abandon). That would be a genuinely unique and load-bearing test.

2. **MERGE ac2 + ac3 into `session_cleanup_ttl.rs`**: Move them there, use the
   `session_cleanup_support.rs` helpers (which include `read_db_status` and `read_body`), and
   add the missing body/DB assertions. If the `write_raw_markdown` fixture path is intentional,
   add a `create_raw_session` helper in that support module and document the distinction.

3. **REWRITE ac4**: Correct the name (`read_memory_renders_na_when_timestamps_missing`) and the
   assertions (`contains("Created: N/A")`). This turns a permanently-passing test into a real
   regression guard for the "N/A" fallback path. Consider adding a companion test that verifies
   `Created:` renders a formatted timestamp *when* `created` frontmatter is present, rounding
   out the coverage.

4. **Consider a true `modified`-frontmatter-vs-DB-timestamp contract test**: None exists in this
   file or in `session_cleanup_ttl.rs`. The question "does `modified` in frontmatter affect when
   a session is marked abandoned?" has no explicit test answer. The answer is currently "no" —
   but that's invisible. Document it.

5. **Align with `session_cleanup_ttl.rs` assertion depth**: The `session_cleanup_ttl.rs` tests
   check frontmatter status, DB status, and body content. These tests only check frontmatter
   status. The shallower assertions mean a regression in DB sync or body appending would go
   undetected here.

---

## Load-Bearing Score: 1/4

Only `ac3` carries any marginal load-bearing weight not already covered by `session_cleanup_ttl.rs`
(it exercises a `write_raw_markdown`-created file through the cleanup path). All three cleanup
tests (`ac1`–`ac3`) together do not collectively catch a bug that the existing `session_cleanup_ttl.rs`
tests would miss. `ac4` is non-falsifiable and load-bearing for no regression scenario. The sole
defensible load-bearing claim is that `ac3` would catch a cleanup regression on files written via
raw disk write rather than `write_markdown` — but that claim requires the `write_raw_markdown`
fixture path to be intentional and documented, which it currently is not.
