# Testing Philosophy: NO FAKE TESTS

## Core Principle

**NO FAKE AI extends to NO FAKE TESTS**

Just as we refuse to create "smart" code that pretends to make decisions, we refuse to create tests that pretend to verify behavior.

## What This Means

### No Mocks

BAD — This is a lie:
```
// Illustrative pseudo-code (actual tests use C#/NUnit)
var mockDB = { save: fn().mockResolvedValue({ id: 1 }) };
```

### Real Systems

GOOD — This is truth:
```
// Illustrative pseudo-code (actual tests use C#/NUnit)
var db = new SqliteConnection(":memory:");
db.SaveEntity(entity);
```

## Testing Rules

1. **Use Real SQLite**
   - In-memory databases for speed
   - Real constraints, real SQL, real behavior
   - If SQLite is "too slow", your code is too slow

2. **Use Real Directories**
   - NO temp directories - use actual project test directories
   - `test-outputs/` (relative to repo root) for all test artifacts
   - Keep test outputs for debugging failed tests
   - Real file permissions, real disk I/O, real behavior
   - Clean up old runs periodically, not immediately

3. **Use Real Data Structures**
   - Complete, valid entities matching schemas
   - No shortcuts, no partial objects
   - If it's hard to create, your schema is too complex

4. **Test Real Behavior**
   - Integration over unit tests
   - Test what users actually do
   - If you can't test it for real, don't build it

5. **Keep Test Evidence**
   - Failed tests should leave artifacts for debugging
   - Performance tests need real I/O measurements
   - Git-ignore test outputs but keep them locally
   - Time-stamped test runs for comparison

## Why This Matters

Mocks create false confidence:
- Tests pass but production fails
- Mocks drift from reality
- Developers test their mocks, not their code

Real tests find real bugs:
- Permission errors
- SQL constraint violations  
- Race conditions
- Actual behavior mismatches

## Practical Examples

### Testing Database Operations

Use a real SQLite in-memory connection (via `Microsoft.Data.Sqlite`). Open the connection, create schema, insert a complete and valid entity, retrieve it, and assert equality. No mocks; no fakes.

Actual tests use C#/NUnit — see `tests/Maenifold.Tests/` for working examples.

### Testing File Operations

Use a real subdirectory under `test-outputs/` (relative to repo root). Create the directory in `[SetUp]`, write actual files, and assert existence and content against the real filesystem. Do not clean up immediately — keep artifacts for post-run debugging.

Use a timestamped run folder (e.g., `test-outputs/markdown/run-{timestamp}/`) so multiple runs do not collide. Periodic cleanup can be done in a separate maintenance step, keeping the last N runs.

Actual tests use C#/NUnit — see `tests/Maenifold.Tests/` for working examples.

### Testing MCP Tools (SequentialThinking)

Invoke the real `SequentialThinkingTools` handler with a complete, valid input payload. Assert on the returned JSON contract — `status`, `thoughtNumber`, etc. — not on internal implementation details.

Actual tests use C#/NUnit — see `tests/Maenifold.Tests/` for working examples.

## The Only Exceptions

Test doubles are acceptable ONLY for:
1. External services you don't control (and we have none)
2. Non-deterministic behavior that can't be controlled (and we have none)
3. Dangerous operations (format disk, delete user data - and we have none)

## Remember

- If you're mocking it, you're not testing it
- Real tests are fast enough with modern tools
- Complexity in tests reveals complexity in code
- NO FAKE AI, NO FAKE TESTS

This philosophy ensures our tests actually test our code, not our imagination.