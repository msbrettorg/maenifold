# Test Evaluation Remediation

```
pending → red → blue → done
    ↑      |      |
    └──────┘──────┘  (reject → pending, attempts++)
```

## Database

`pipeline.db` is the source of truth. Eval specs live in `todo/` and never move.

```sql
-- Dashboard
SELECT status, count(*) FROM eval_queue GROUP BY status;

-- Get SWE batch
SELECT file FROM eval_queue WHERE status = 'pending' ORDER BY attempts, file LIMIT 8;

-- Advance: SWE done → red-team
UPDATE eval_queue SET status = 'red', updated_at = datetime('now') WHERE file = ?;

-- Advance: red-team passed → blue-team
UPDATE eval_queue SET status = 'blue', updated_at = datetime('now') WHERE file = ?;

-- Advance: blue-team passed → done
UPDATE eval_queue SET status = 'done', updated_at = datetime('now') WHERE file = ?;

-- Reject (from red or blue → back to pending)
UPDATE eval_queue SET status = 'pending', attempts = attempts + 1, updated_at = datetime('now') WHERE file = ?;

-- Auto-stuck (3+ failures)
UPDATE eval_queue SET status = 'stuck' WHERE attempts >= 3 AND status = 'pending';

-- Recovery (PM restart — reset in-flight items)
UPDATE eval_queue SET status = 'pending' WHERE status NOT IN ('pending', 'done', 'stuck');
```

## Dispatch

Query the DB. For each status with work, spin up matching agents (up to 8 total):

| Status    | Agent    | Prompt             | Task                                                  |
|-----------|----------|--------------------|-------------------------------------------------------|
| `pending` | SWE      | `swe-prompt.md`    | "Read `swe-prompt.md`. Process `todo/{file}`."        |
| `red`     | Red-team | `red-prompt.md`    | "Read `red-prompt.md`. Audit `todo/{file}`."          |
| `blue`    | Blue-team| `blue-prompt.md`   | "Read `blue-prompt.md`. Verify `todo/{file}`."        |

Mix stages to maximize pipeline throughput — while SWE works batch A, red-team audits batch B, blue-team verifies batch C.

On agent completion:
- SWE reports done → advance to `red`
- Red-team reports PASS → advance to `blue`
- Red-team reports REJECT → reject (back to `pending`, attempts++)
- Blue-team reports PASS → advance to `done`
- Blue-team reports FAIL → reject (back to `pending`, attempts++)

You don't read eval files. You don't know what's in them. You match status → agent type, pass the file path, update DB on completion.
