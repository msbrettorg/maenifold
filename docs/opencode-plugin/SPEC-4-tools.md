# SPEC-4: Tool Execution Hook (Concept Augmentation)

## Document Control

| Version | Date | Author | Description |
|---------|------|--------|-------------|
| 1.0 | 2026-01-27 | SWE Agent | Initial specification for FR-4.x |

---

## 1. Overview

This specification defines the implementation of the tool execution hook that detects WikiLinks like `[[REST-API]]`, `[[database-migrations]]`, `[[logging]]` in tool arguments and augments tool input with knowledge graph context. This enables automatic context injection for any tool invocation containing WikiLink-style concept references.

**Traced Requirements**:
- FR-4.1: Plugin SHALL subscribe to `tool.execute.before` event
- FR-4.2: Plugin SHALL detect WikiLinks like `[[authentication]]`, `[[caching]]` in tool arguments
- FR-4.3: Plugin SHALL build context for detected concepts
- FR-4.4: Plugin SHALL augment tool input with graph context
- FR-4.5: Plugin SHALL skip augmentation for tools with no text arguments

---

## 2. Hook Handler Signature

### 2.1 OpenCode API Pattern

The `tool.execute.before` hook follows OpenCode's standard hook pattern with input/output parameters:

```typescript
// T-4.1: RTM FR-4.1
"tool.execute.before": async (input, output) => {
  // input: { tool: string, sessionID: string, callID: string }
  // output: { args: Record<string, unknown> }
  
  // Augmentation logic here
}
```

### 2.2 Input Object Structure

| Property | Type | Description |
|----------|------|-------------|
| `tool` | `string` | Name of the tool being executed (e.g., "bash", "write", "read") |
| `sessionID` | `string` | Unique identifier for the current session |
| `callID` | `string` | Unique identifier for this specific tool call |

### 2.3 Output Object Structure

| Property | Type | Description |
|----------|------|-------------|
| `args` | `Record<string, unknown>` | Mutable object containing tool arguments |

The `output.args` object can be modified in-place. Changes persist to the actual tool execution.

---

## 3. Concept Detection Algorithm

### 3.1 WikiLink Pattern

Concepts are detected using the standard WikiLink regex pattern:

```typescript
// T-4.2: RTM FR-4.2
const WIKILINK_PATTERN = /\[\[([^\]]+)\]\]/g;
```

This pattern captures:
- Opening `[[` bracket pair
- One or more non-bracket characters (the concept name)
- Closing `]]` bracket pair

### 3.2 Recursive String Search

Tool arguments can be nested objects, arrays, or primitive values. Concept detection must recursively traverse the entire argument structure to find all text content.

```typescript
// T-4.2: RTM FR-4.2
function extractConceptsFromArgs(args: unknown): string[] {
  const concepts: Set<string> = new Set();
  
  function traverse(value: unknown): void {
    if (typeof value === 'string') {
      // Extract WikiLinks from string values
      const matches = value.matchAll(WIKILINK_PATTERN);
      for (const match of matches) {
        concepts.add(match[1].trim());
      }
    } else if (Array.isArray(value)) {
      // Recurse into arrays
      for (const item of value) {
        traverse(item);
      }
    } else if (value !== null && typeof value === 'object') {
      // Recurse into object properties
      for (const key of Object.keys(value)) {
        traverse((value as Record<string, unknown>)[key]);
      }
    }
    // Skip null, undefined, numbers, booleans
  }
  
  traverse(args);
  return Array.from(concepts);
}
```

### 3.3 Edge Cases

| Case | Behavior |
|------|----------|
| Empty args `{}` | Return empty array, skip augmentation |
| Nested objects | Recurse into all properties |
| Arrays of strings | Extract from each string element |
| Null/undefined values | Skip gracefully |
| Numeric/boolean values | Skip (no string content) |
| Malformed WikiLinks `[single]` | Ignore (pattern requires double brackets) |
| Empty WikiLinks `[[]]` | Ignore (no concept name) |

---

## 4. Context Building and Injection Pattern

### 4.1 Context Building

For each detected concept, invoke `maenifold` CLI to build context:

```typescript
// T-4.3: RTM FR-4.3
async function buildContextForConcepts(concepts: string[]): Promise<string> {
  if (concepts.length === 0) return "";
  
  const contextParts: string[] = [];
  
  for (const concept of concepts) {
    try {
      // Call maenifold buildcontext for each concept
      const result = await runMaenifoldTool("buildcontext", {
        conceptName: concept,
        depth: 1,
        includeContent: true,
        maxEntities: 5
      });
      
      if (result) {
        contextParts.push(result);
      }
    } catch (error) {
      // Log but don't fail - graceful degradation per NFR-2.1
      await logError(`BuildContext failed for [[${concept}]]`, error);
    }
  }
  
  return contextParts.join("\n\n---\n\n");
}
```

### 4.2 Context Injection Strategy

The augmentation strategy depends on the tool type and argument structure:

```typescript
// T-4.4: RTM FR-4.4
function augmentArgsWithContext(
  args: Record<string, unknown>,
  context: string,
  toolName: string
): void {
  // Strategy 1: Tool has known text field (command, content, prompt, query)
  const TEXT_FIELDS = ['command', 'content', 'prompt', 'query', 'message', 'text', 'input'];
  
  for (const field of TEXT_FIELDS) {
    if (field in args && typeof args[field] === 'string') {
      // Prepend context as a comment block
      const contextBlock = formatContextBlock(context);
      args[field] = `${contextBlock}\n\n${args[field]}`;
      return;
    }
  }
  
  // Strategy 2: No known text field - add as new 'context' property
  // This allows tools to optionally consume the context
  args['_maenifold_context'] = context;
}

function formatContextBlock(context: string): string {
  return `<!-- Knowledge Graph Context (auto-injected by maenifold) -->\n${context}\n<!-- End Knowledge Graph Context -->`;
}
```

### 4.3 Injection Format

The injected context follows this structure:

```
<!-- Knowledge Graph Context (auto-injected by maenifold) -->
## [[concept-name]]

[BuildContext output including related concepts and file excerpts]

---

## [[another-concept]]

[BuildContext output for second concept]
<!-- End Knowledge Graph Context -->
```

This format:
- Is clearly delineated with HTML comments (works in markdown, code comments)
- Preserves WikiLinks for downstream processing
- Separates multiple concept contexts with horizontal rules

---

## 5. Skip Conditions

### 5.1 No Text Arguments

```typescript
// T-4.5: RTM FR-4.5
function hasTextArguments(args: Record<string, unknown>): boolean {
  function containsString(value: unknown): boolean {
    if (typeof value === 'string') return true;
    if (Array.isArray(value)) return value.some(containsString);
    if (value !== null && typeof value === 'object') {
      return Object.values(value as Record<string, unknown>).some(containsString);
    }
    return false;
  }
  
  return containsString(args);
}
```

### 5.2 No Concepts Detected

If `extractConceptsFromArgs()` returns an empty array, skip augmentation entirely.

### 5.3 CLI Unavailable

If `maenifold` CLI is not found (per FR-1.2, FR-1.3), skip augmentation and log at debug level.

### 5.4 Timeout

If context building exceeds the configured timeout (per FR-1.4, NFR-1.3 < 1s per concept), abort and continue with unaugmented args.

### 5.5 Complete Skip Logic

```typescript
// T-4.5: RTM FR-4.5
async function shouldSkipAugmentation(
  args: Record<string, unknown>
): Promise<{ skip: boolean; reason?: string }> {
  // Check 1: No text arguments
  if (!hasTextArguments(args)) {
    return { skip: true, reason: "no_text_args" };
  }
  
  // Check 2: No concepts detected
  const concepts = extractConceptsFromArgs(args);
  if (concepts.length === 0) {
    return { skip: true, reason: "no_concepts" };
  }
  
  return { skip: false };
}
```

---

## 6. Complete Handler Implementation

```typescript
// T-4.1, T-4.2, T-4.3, T-4.4, T-4.5: RTM FR-4.1, FR-4.2, FR-4.3, FR-4.4, FR-4.5
"tool.execute.before": async (input, output) => {
  const { tool, sessionID } = input;
  const args = output.args;
  
  // Skip check
  const skipResult = await shouldSkipAugmentation(args);
  if (skipResult.skip) {
    await client.app.log({
      body: {
        service: "maenifold",
        level: "debug",
        message: `Skipping augmentation for ${tool}`,
        extra: { reason: skipResult.reason, sessionID }
      }
    });
    return;
  }
  
  // Extract concepts
  const concepts = extractConceptsFromArgs(args);
  await client.app.log({
    body: {
      service: "maenifold",
      level: "info",
      message: `Detected concepts in ${tool}: ${concepts.join(", ")}`,
      extra: { tool, concepts, sessionID }
    }
  });
  
  // Build context with timeout protection
  const startTime = Date.now();
  const TIMEOUT_PER_CONCEPT_MS = 1000; // NFR-1.3
  const MAX_TOTAL_TIMEOUT_MS = 5000;   // NFR-1.1
  
  try {
    const context = await Promise.race([
      buildContextForConcepts(concepts),
      new Promise<string>((_, reject) =>
        setTimeout(() => reject(new Error("context_timeout")), MAX_TOTAL_TIMEOUT_MS)
      )
    ]);
    
    if (context) {
      augmentArgsWithContext(args, context, tool);
      
      const elapsed = Date.now() - startTime;
      await client.app.log({
        body: {
          service: "maenifold",
          level: "info",
          message: `Augmented ${tool} with ${concepts.length} concept(s) in ${elapsed}ms`,
          extra: { tool, conceptCount: concepts.length, elapsed, sessionID }
        }
      });
    }
  } catch (error) {
    // Graceful degradation - continue without augmentation
    await client.app.log({
      body: {
        service: "maenifold",
        level: "warn",
        message: `Augmentation failed for ${tool}: ${error}`,
        extra: { tool, error: String(error), sessionID }
      }
    });
  }
}
```

---

## 7. Performance Considerations

### 7.1 Latency Budget

Per NFR-1.3, tool augmentation must complete in < 1 second per concept:

| Operation | Target | Notes |
|-----------|--------|-------|
| Concept extraction | < 10ms | In-memory regex + traversal |
| CLI invocation | < 800ms | Network/process overhead |
| Context formatting | < 10ms | String manipulation |
| Arg mutation | < 1ms | Object property assignment |

### 7.2 Parallel vs Sequential

For multiple concepts, consider parallel BuildContext calls with collective timeout:

```typescript
// Alternative: parallel context building
const contextPromises = concepts.map(concept =>
  runMaenifoldTool("buildcontext", { conceptName: concept, depth: 1, includeContent: true })
    .catch(() => "") // Individual failures don't block others
);

const contexts = await Promise.all(contextPromises);
const context = contexts.filter(Boolean).join("\n\n---\n\n");
```

This trades increased CLI process overhead for reduced total latency when multiple concepts are detected.

---

## 8. Testing Guidance

### 8.1 Unit Test Cases

| Test | Input | Expected |
|------|-------|----------|
| Single concept in string arg | `{ command: "test [[auth]]" }` | Augmented with auth context |
| Multiple concepts | `{ prompt: "[[JWT]] and [[OAuth]]" }` | Both contexts included |
| Nested concept | `{ options: { query: "[[search]]" } }` | Concept detected in nested object |
| Array with concepts | `{ parts: ["[[A]]", "[[B]]"] }` | Both concepts extracted |
| No concepts | `{ command: "ls -la" }` | No augmentation |
| Empty args | `{}` | Skipped |
| Numeric args only | `{ count: 5, limit: 10 }` | Skipped |
| Malformed WikiLink | `{ text: "[single bracket]" }` | No concept extracted |

### 8.2 Integration Test Cases

| Test | Scenario | Validation |
|------|----------|------------|
| CLI unavailable | MAENIFOLD_ROOT invalid | Graceful skip, no error |
| CLI timeout | Mock slow response | Completes within 5s, no augmentation |
| Large concept graph | Concept with 50+ relations | Respects maxEntities=5 |

---

## 9. Source

- PRD: `/docs/opencode-plugin/PRD.md` (FR-4.x requirements)
- RTM: `/docs/opencode-plugin/RTM.md` (traceability matrix)
- API Reference: `memory://tech/integrations/opencode-plugin-api-reference` (hook signatures)
