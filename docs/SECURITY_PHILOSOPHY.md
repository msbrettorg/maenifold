# Security Philosophy: NO FAKE SECURITY

## Core Principle

**NO FAKE AI extends to NO FAKE SECURITY**

Just as we refuse to create "smart" code that pretends to make decisions and tests that pretend to verify behavior, we refuse to create security measures that pretend to provide protection.

For strange-loop, this means securing what actually matters: **the integrity of your personal knowledge system**.

## What This Means

### ❌ NO SECURITY THEATER

```typescript
// BAD - Web security for a local stdio MCP server
const server = new McpServer({
  name: "strange-loop-server",
  version: "1.0.0",
  auth: {
    oauth: {
      authorizationServer: "https://auth.example.com",
      clientId: "strange-loop-client",
    },
  },
});
// This is security theater - wrong threat model entirely

// BAD - Generic input validation
function validateInput(input: any): boolean {
  return typeof input === "object"; // "We validate inputs"
}
```

### ✅ REAL SECURITY MEASURES

```typescript
// GOOD - Use prepared statements for SQL safety
const stmt = db.prepare("INSERT INTO entities (title, content) VALUES (?, ?)");
stmt.run(title, content);

// GOOD - Let errors be errors - don't hide them or "fix" them
try {
  await writeMarkdownFile(path, content);
} catch (error) {
  // Don't catch and retry - let the user handle it
  throw error;
}

// GOOD - Trust the user to manage their own system
// No artificial limits, no path restrictions, no resource monitoring
// It's their knowledge, their system, their choice
```

## Security Rules for Personal Knowledge Systems

### 1. **Use Prepared Statements**

- Always use prepared statements for SQL operations
- Prevents accidental SQL syntax errors
- SQLite handles the rest

### 2. **Let Errors Be Errors**

- Don't catch and retry operations
- Don't provide fallbacks or "smart" recovery
- Let the user see what actually happened

### 3. **Trust the User**

- It's their system, their data, their choices
- No artificial limits on file sizes, recursion depth, or resource usage
- The OS and SQLite provide the real boundaries

## The One Pillar of Personal Knowledge System Security

For strange-loop's actual threat model:

### **Use Prepared Statements**

```typescript
// ❌ FAKE: String concatenation
const query = `INSERT INTO entities (title, content) VALUES ('${title}', '${content}')`;
db.exec(query); // SQL injection risk

// ✅ REAL: Prepared statements
const stmt = db.prepare("INSERT INTO entities (title, content) VALUES (?, ?)");
stmt.run(title, content); // Safe from SQL syntax errors
```

That's it. SQLite handles integrity, the OS handles permissions, the user handles resource management.

## Why This Matters for strange-loop

Security theater creates false confidence:

- Checkboxes are ticked but no real protection is provided
- Resources wasted on protections against non-threats
- Complexity added where simplicity serves better
- User autonomy replaced with paternalistic restrictions

Real security for personal systems:

- Prepared statements prevent accidental SQL errors
- Clear error messages help users understand what happened
- Trust users to manage their own resources and data
- Let SQLite and the OS do what they're designed to do

## Practical Examples for strange-loop

### Real Security: Prepared Statements

```typescript
describe("strange-loop Security", () => {
  it("uses prepared statements for SQL operations", async () => {
    const db = new Database(":memory:");

    // This is the only security measure we need
    const stmt = db.prepare(
      "INSERT INTO entities (title, content) VALUES (?, ?)"
    );

    // Even with "malicious" content, prepared statements handle it safely
    const result = stmt.run("Test'; DROP TABLE entities; --", "Content");
    expect(result.changes).toBe(1);

    // Verify the table still exists and data was inserted correctly
    const rows = db.prepare("SELECT * FROM entities").all();
    expect(rows).toHaveLength(1);
    expect(rows[0].title).toBe("Test'; DROP TABLE entities; --");
  });

  it("lets errors be errors", async () => {
    // Don't catch and retry - let the user handle failures
    await expect(writeToNonExistentDirectory()).rejects.toThrow();
  });
});
```

## Security Anti-Patterns to Avoid for strange-loop

### 1. **Any Guard Classes**

```typescript
// ❌ FAKE: Creating "protection" for a personal system
class KnowledgeSystemGuard {
  validateInput() {
    /* making decisions for the user */
  }
}

// ✅ REAL: Trust the user to manage their own system
// No guard classes needed
```

### 2. **Artificial Limits**

```typescript
// ❌ FAKE: Deciding what's "too big" for the user
if (content.length > 10 * 1024 * 1024) {
  throw new Error("Content too large");
}

// ✅ REAL: Let the user decide what's appropriate
// Their disk, their choice
```

### 3. **Path Restrictions**

```typescript
// ❌ FAKE: Restricting where users can organize their own knowledge
if (!fullPath.startsWith(BASE_PATH)) {
  throw new Error("Path traversal detected");
}

// ✅ REAL: Let users organize their files however they want
// It's their filesystem
```

## Remember for strange-loop

- If you're restricting the user, you're not providing security
- Real security is invisible and protects against actual threats
- The user is the admin of their personal knowledge system
- NO FAKE AI, NO FAKE TESTS, NO FAKE SECURITY

## Implementation for strange-loop

1. **Use Prepared Statements**: Already implemented ✓
2. **Let Errors Be Errors**: Already implemented ✓
3. **Trust the User**: This is the philosophy

That's it. The system already follows real security principles by not implementing fake security.

This philosophy ensures our security actually secures our personal knowledge system, not just compliance checkboxes.
