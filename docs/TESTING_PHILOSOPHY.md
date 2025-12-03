# Testing Philosophy: NO FAKE TESTS

## Core Principle

**NO FAKE AI extends to NO FAKE TESTS**

Just as we refuse to create "smart" code that pretends to make decisions, we refuse to create tests that pretend to verify behavior.

## What This Means

### ❌ NO MOCKS
```typescript
// BAD - This is a lie
const mockDB = {
  save: vi.fn().mockResolvedValue({ id: 1 })
};
```

### ✅ REAL SYSTEMS
```typescript
// GOOD - This is truth
const db = new Database(':memory:');
await db.save(entity);
```

## Testing Rules

1. **Use Real SQLite**
   - In-memory databases for speed
   - Real constraints, real SQL, real behavior
   - If SQLite is "too slow", your code is too slow

2. **Use Real Directories**
   - NO temp directories - use actual project test directories
   - `/workspace/test-outputs/` for all test artifacts
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
```typescript
describe('MemoryDatabase', () => {
  let db: MemoryDatabase;
  
  beforeEach(() => {
    // Real database, in memory
    db = new MemoryDatabase(':memory:');
  });
  
  it('saves and retrieves entities', async () => {
    const entity = createValidEntity(); // Real, complete entity
    await db.saveEntity(entity);
    const retrieved = await db.getEntity(entity.permalink);
    expect(retrieved).toEqual(entity);
  });
});
```

### Testing File Operations
```typescript
describe('MarkdownManager', () => {
  const baseTestDir = join(process.cwd(), 'test-outputs', 'markdown');
  const testRunDir = join(baseTestDir, `run-${Date.now()}`);
  
  beforeAll(async () => {
    await mkdir(testRunDir, { recursive: true });
  });
  
  // NO immediate cleanup - keep artifacts for debugging
  
  it('writes and reads markdown files', async () => {
    const manager = new MarkdownManager(testRunDir);
    await manager.writeMarkdownFile(entity);
    
    // Check the ACTUAL file in ACTUAL directory
    const filePath = join(testRunDir, 'notes', 'test.md');
    const exists = existsSync(filePath);
    expect(exists).toBe(true);
    
    // Test outputs remain in test-outputs/markdown/run-[timestamp]/
    // for inspection after test runs
  });
});

// Deterministic Test IDs
// Use generateTestId() instead of Date.now() for reproducible test runs
// Format: test-${TEST_ID || 'run'}-${sequentialCounter}
// Example: TEST_ID=debug-issue-123 npm test
// This allows reproducing specific test failures

// Periodic cleanup in separate maintenance script
async function cleanOldTestRuns() {
  const runs = await readdir(baseTestDir);
  const sorted = runs.sort().reverse();
  // Keep last 20 runs
  for (const old of sorted.slice(20)) {
    await rm(join(baseTestDir, old), { recursive: true });
  }
}
```

### Testing MCP Tools
```typescript
describe('Fugue Tool', () => {
  it('handles real tool input/output', async () => {
    const input = {
      thought: 'Real thought',
      thoughtNumber: 1,
      totalThoughts: 3,
      nextThoughtNeeded: true
    };
    
    const result = await fugueHandler(input);
    // Test the CONTRACT, not the implementation
    expect(JSON.parse(result.content[0].text)).toMatchObject({
      status: 'thinking',
      thoughtNumber: 1
    });
  });
});
```

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