# Convert maenifold from C# to Python

## Source
`/Users/brett/src/ma-collective/maenifold`

## Target
`/Users/brett/src/ma-collective/maenifold-py`

## Requirements

1. **100% CLI parity**: `maenifold --help` and `python -m maenifold.main --help` must produce identical output. Same for all error cases. Same exit codes.

2. **100% tool parity**: All 28 tools must work identically to C# version.

3. **100% database compatibility**: Python must read/write the same SQLite database as C#. Both CLIs must be able to use the same DB simultaneously.

4. **All 153 tests pass**: `/Users/brett/src/ma-collective/maenifold/docs/TEST-MATRIX.md`

## Success Criteria

Run these commands. Output must be identical between C# and Python:

```bash
# C# binary location
CS="/Users/brett/src/ma-collective/maenifold/src/bin/Debug/net9.0/maenifold"

# Python module
PY="python -m maenifold.main"

# Test 1: Help
$CS --help
$PY --help

# Test 2: Invalid tool
$CS --tool FakeTool --payload '{}'
$PY --tool FakeTool --payload '{}'

# Test 3: Bad JSON
$CS --tool WriteMemory --payload 'not json'
$PY --tool WriteMemory --payload 'not json'

# Test 4: Missing param
$CS --tool WriteMemory --payload '{"title":"x"}'
$PY --tool WriteMemory --payload '{"title":"x"}'

# Test 5: Valid operation
$CS --tool MemoryStatus --payload '{}'
$PY --tool MemoryStatus --payload '{}'
```

Exit codes must match. Output format must match. Error messages must match.

## Completion

Output `<promise>ALLCOMPLETE</promise>` when:
- All 5 CLI parity tests pass (identical output)
- All 153 TEST-MATRIX.md tests pass
- Database cross-compatibility verified

Do not claim completion until verified.
Do not leave tasks running in the background.

You are the product manager. You are responsible for for the success of this project, not your agents. They will lie to you to try to finish faster. They will misconstrue your instructions. The work items in TODO.md which you assign to your agents must be unambiguous and complete - including instructions, goals, non-goals and success criteria. You MUST spend the time to create detailed and complete todo.md items for your agents to avoid re-work - they have NO context beyond their session. Decisions made by one agent will not flow to another agent. You are the glue that holds this project together. 

researcher -> swe -> red-team -> blue-team -> ... until done.

Output <promise>ALLCOMPLETE</promise> only when all items have been verified and all tests pass.
