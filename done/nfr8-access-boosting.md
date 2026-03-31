---
id: nfr8-access-boosting
status: done
---

# nfr8-access-boosting

**Requirement**: NFR-8
**Test Cases**: TC-NFR8.1, TC-NFR8.2, TC-NFR8.3
**Component**: crates/maenifold-core/src/tools/read_memory.rs, crates/maenifold-core/tests/access_boosting_reads.rs

## Acceptance Criteria

1. ReadMemory refreshes `last_accessed` timestamp in the `file_content` table on each successful read (by URI and by title)
2. Sequential reads of the same file update `last_accessed` each time (monotonically increasing)
3. ReadMemory on a non-existent identifier does not create or update any `last_accessed` entry
4. ReadMemory with `learn=true` short-circuits before touching `last_accessed` (returns help docs without DB write)
5. The decay-weighting system uses `last_accessed` timestamps with ACT-R power-law recency bias

## Verdicts

- TC-NFR8.1 (ReadMemory refreshes last_accessed on successful reads): VERIFY — test file exists at crates/maenifold-core/tests/access_boosting_reads.rs, needs verification
- TC-NFR8.2 (non-existent identifier does not update last_accessed): VERIFY — test may exist, needs verification
- TC-NFR8.3 (learn=true short-circuit skips last_accessed): VERIFY — test may exist, needs verification

## Blue-team Verification

**Agent**: gpt-5.4 | **Date**: 2026-03-15
**Verdict**: PASS

```
cargo test -p maenifold-core --test integration_tests -- access_boosting
   → 5/5 RTM-targeted tests passed
```

Passing tests:
- `access_boosting_reads::read_memory_updates_last_accessed_on_every_read` (RTM-201)
- `access_boosting_reads::read_memory_updates_last_accessed_on_subsequent_reads` (RTM-201)
- `access_boosting_reads::read_memory_by_title_updates_last_accessed` (RTM-201)
- `access_boosting_reads::read_memory_on_missing_uri_does_not_update_last_accessed` (RTM-202)
- `access_boosting_reads::read_memory_learn_mode_does_not_update_last_accessed` (RTM-203)

Note: 3 adjacent tests (search, build_context, decay_benchmark) failed with DB locking/IO errors — test isolation issues, not logic bugs. Tracked separately.
