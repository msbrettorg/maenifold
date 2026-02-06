using NUnit.Framework;

// Disable parallel test execution for the entire assembly.
// Rationale: All tests share a SQLite database via Config static state.
// Parallel execution causes SQLite Error 16 (file locking) on CI runners.
// This is the proper fix - not a workaround - because:
// 1. Tests legitimately share database state (that's what we're testing)
// 2. SQLite in WAL mode still has write locking constraints
// 3. Test isolation at the database level would require significant refactoring
[assembly: NonParallelizable]
