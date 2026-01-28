# Technical Specification: Compaction Handler (FR-3.x)

## Document Control

| Version | Date | Author | Description |
|---------|------|--------|-------------|
| 1.0 | 2026-01-27 | SWE Agent | Initial specification |

---

## 1. Overview

This specification details the implementation of the [[compaction]] handler for the [[OpenCode]] [[maenifold]] plugin, addressing requirements FR-3.1 through FR-3.5. The handler preserves session knowledge before [[OpenCode]] compacts the context window.

**Traceability**: T-SPEC-3 (RTM FR-3.x)

---

## 2. Hook Handler Signature

### 2.1 Event Subscription (FR-3.1)

The plugin SHALL subscribe to the `experimental.session.compacting` hook. This hook fires immediately before OpenCode invokes the LLM to generate a compaction summary.

```typescript
// T-SPEC-3: RTM FR-3.1
"experimental.session.compacting": async (input, output) => {
  // input: { sessionID: string }
  // output: { context: string[], prompt?: string }
}
```

**Hook Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `input.sessionID` | `string` | UUID of the session being compacted |
| `output.context` | `string[]` | Mutable array for context injection |
| `output.prompt` | `string \| undefined` | Optional replacement for entire compaction prompt |

**Implementation Notes:**
- This hook is **pre-compaction**: the summary has not yet been generated
- The `output.context` array can be mutated to inject additional context
- Setting `output.prompt` replaces OpenCode's default compaction prompt entirely (not recommended)

---

## 3. Concept Extraction Pattern (FR-3.2)

### 3.1 WikiLink Regex

Extract `[[WikiLink]]` concepts from conversation content using the following regex pattern:

```typescript
// T-SPEC-3: RTM FR-3.2
const WIKILINK_PATTERN = /\[\[([^\]]+)\]\]/g;

function extractConcepts(text: string): string[] {
  const matches: string[] = [];
  let match: RegExpExecArray | null;
  
  while ((match = WIKILINK_PATTERN.exec(text)) !== null) {
    const concept = match[1]
      .toLowerCase()
      .replace(/\s+/g, '-')      // Spaces to hyphens
      .replace(/[^a-z0-9-]/g, '') // Remove invalid chars
      .replace(/-+/g, '-')        // Collapse multiple hyphens
      .replace(/^-|-$/g, '');     // Trim leading/trailing hyphens
    
    if (concept.length > 0 && concept.length <= 100) {
      matches.push(concept);
    }
  }
  
  // Deduplicate and return
  return [...new Set(matches)];
}
```

**Normalization Rules:**
- Convert to lowercase
- Replace spaces with hyphens
- Remove non-alphanumeric characters (except hyphens)
- Collapse multiple hyphens
- Limit concept length to 100 characters

### 3.2 Conversation Access

Retrieve conversation messages via the SDK client:

```typescript
// T-SPEC-3: RTM FR-3.2
async function getConversationText(
  client: OpencodeClient,
  sessionID: string
): Promise<string> {
  const result = await client.session.messages({
    path: { id: sessionID }
  });
  
  const messages = result.data ?? [];
  
  return messages
    .flatMap(m => m.parts)
    .filter(p => p.type === "text")
    .map(p => (p as { text: string }).text)
    .join("\n");
}
```

---

## 4. Decision Extraction Heuristics (FR-3.3)

### 4.1 Decision Pattern Matching

Extract decisions using heuristic patterns that identify architectural choices, implementation decisions, and explicit declarations:

```typescript
// T-SPEC-3: RTM FR-3.3
const DECISION_PATTERNS: RegExp[] = [
  // Explicit decision statements
  /(?:we|I)\s+(?:decided|chose|selected|picked)\s+(?:to\s+)?(.{10,150})/gi,
  
  // Architectural choices
  /(?:using|chose|selected)\s+(\w+)\s+(?:for|as|instead of)\s+(.{5,100})/gi,
  
  // Explicit because/reason statements
  /(?:because|since|the reason is)\s+(.{10,150})/gi,
  
  // Implementation approach declarations
  /(?:approach|strategy|solution)[:\s]+(.{10,150})/gi,
  
  // Rejection patterns (what NOT to do)
  /(?:we|I)\s+(?:won't|will not|shouldn't|decided against)\s+(.{10,100})/gi,
];

interface Decision {
  pattern: string;    // Which pattern matched
  text: string;       // The extracted decision text
  context: string;    // Surrounding context (sentence)
}

function extractDecisions(text: string): Decision[] {
  const decisions: Decision[] = [];
  
  // Split into sentences for context extraction
  const sentences = text.split(/(?<=[.!?])\s+/);
  
  for (const sentence of sentences) {
    for (const pattern of DECISION_PATTERNS) {
      // Reset regex state
      pattern.lastIndex = 0;
      const match = pattern.exec(sentence);
      
      if (match) {
        decisions.push({
          pattern: pattern.source.slice(0, 50),
          text: match[1]?.trim() || match[0].trim(),
          context: sentence.trim().slice(0, 200)
        });
      }
    }
  }
  
  // Deduplicate by text content
  const seen = new Set<string>();
  return decisions.filter(d => {
    const key = d.text.toLowerCase();
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}
```

### 4.2 Decision Categorization

Decisions SHOULD be categorized when written to memory:

| Category | Patterns | Example |
|----------|----------|---------|
| `architecture` | "using X for Y", "approach:" | "using SQLite for local storage" |
| `rejection` | "won't", "decided against" | "won't use Redis for this" |
| `implementation` | "decided to", "chose to" | "decided to use async/await" |
| `rationale` | "because", "since", "reason" | "because it's simpler" |

---

## 5. WriteMemory Payload Structure (FR-3.4)

### 5.1 Memory Location

Compaction summaries SHALL be written to `memory://sessions/compaction/` with the following path structure:

```
memory://sessions/compaction/{year}/{month}/{day}/session-{sessionID-prefix}.md
```

Example: `memory://sessions/compaction/2026/01/27/session-abc123.md`

### 5.2 Payload Structure

```typescript
// T-SPEC-3: RTM FR-3.4
interface CompactionSummaryPayload {
  title: string;      // "Compaction: {project} {timestamp}"
  content: string;    // Markdown with [[concepts]]
  folder: string;     // "sessions/compaction/YYYY/MM/DD"
  tags: string[];     // ["compaction", "session", project-name]
}

function buildCompactionPayload(
  projectName: string,
  sessionID: string,
  concepts: string[],
  decisions: Decision[]
): CompactionSummaryPayload {
  const now = new Date();
  const dateFolder = `${now.getFullYear()}/${String(now.getMonth() + 1).padStart(2, '0')}/${String(now.getDate()).padStart(2, '0')}`;
  
  // Build concept list with WikiLinks
  const conceptList = concepts
    .slice(0, 20) // Limit to top 20 concepts
    .map(c => `- [[${c}]]`)
    .join('\n');
  
  // Build decision list
  const decisionList = decisions
    .slice(0, 10) // Limit to top 10 decisions
    .map(d => `- ${d.text} _(${d.pattern.slice(0, 20)}...)_`)
    .join('\n');
  
  const content = `# Compaction Summary

**Session**: ${sessionID.slice(0, 8)}
**Project**: [[${projectName}]]
**Timestamp**: ${now.toISOString()}

## Key Concepts

${conceptList || '_No concepts extracted_'}

## Decisions Made

${decisionList || '_No decisions extracted_'}

## Session Context

This [[compaction]] summary preserves key knowledge from the session before context truncation. Use [[BuildContext]] on the concepts above to restore working memory.
`;

  return {
    title: `Compaction: ${projectName} ${now.toISOString().slice(0, 10)}`,
    content,
    folder: `sessions/compaction/${dateFolder}`,
    tags: ['compaction', 'session', projectName.toLowerCase()]
  };
}
```

### 5.3 CLI Invocation

Write to memory using the maenifold CLI:

```typescript
// T-SPEC-3: RTM FR-3.4
async function writeCompactionSummary(
  cli: string,
  payload: CompactionSummaryPayload
): Promise<boolean> {
  const toolPayload = {
    title: payload.title,
    content: payload.content,
    folder: payload.folder,
    tags: payload.tags
  };
  
  const proc = Bun.spawn([
    cli,
    "--tool", "WriteMemory",
    "--payload", JSON.stringify(toolPayload)
  ], {
    stdout: "pipe",
    stderr: "pipe"
  });
  
  const exitCode = await proc.exited;
  return exitCode === 0;
}
```

---

## 6. Context Injection (FR-3.5)

### 6.1 output.context.push() Pattern

Inject context into the compaction prompt by pushing to the mutable `output.context` array:

```typescript
// T-SPEC-3: RTM FR-3.5
"experimental.session.compacting": async (input, output) => {
  const sessionID = input.sessionID;
  
  // Step 1: Get conversation text
  const conversationText = await getConversationText(client, sessionID);
  
  // Step 2: Extract concepts (FR-3.2)
  const concepts = extractConcepts(conversationText);
  
  // Step 3: Extract decisions (FR-3.3)
  const decisions = extractDecisions(conversationText);
  
  // Step 4: Write to memory (FR-3.4)
  const projectName = path.basename(directory);
  const payload = buildCompactionPayload(projectName, sessionID, concepts, decisions);
  await writeCompactionSummary(cli, payload);
  
  // Step 5: Inject context (FR-3.5)
  if (concepts.length > 0 || decisions.length > 0) {
    output.context.push(`## Knowledge Graph Preservation

**Preserved Concepts** (${concepts.length}): ${concepts.slice(0, 10).map(c => `[[${c}]]`).join(', ')}

**Key Decisions** (${decisions.length}):
${decisions.slice(0, 5).map(d => `- ${d.text}`).join('\n')}

When summarizing this session, preserve these [[WikiLink]] concept references to maintain knowledge graph connectivity.`);
  }
}
```

### 6.2 Context Injection Guidelines

| Guideline | Rationale |
|-----------|-----------|
| Keep injected context under 500 tokens | Avoid bloating compaction prompt |
| Use WikiLink format for concepts | Maintains graph connectivity post-compaction |
| Prioritize decisions over raw concepts | Decisions carry more semantic weight |
| Do NOT replace `output.prompt` | Let OpenCode handle compaction logic |

---

## 7. Complete Handler Implementation

```typescript
// T-SPEC-3: RTM FR-3.1, FR-3.2, FR-3.3, FR-3.4, FR-3.5
"experimental.session.compacting": async (input, output) => {
  const sessionID = input.sessionID;
  
  try {
    // Locate CLI (graceful degradation if unavailable)
    const cli = findMaenifoldCli();
    if (!cli) {
      await log("warn", "maenifold CLI not found, skipping compaction preservation");
      return;
    }
    
    // Get conversation content
    const conversationText = await getConversationText(client, sessionID);
    if (!conversationText || conversationText.length < 100) {
      await log("debug", "Insufficient conversation content for compaction preservation");
      return;
    }
    
    // Extract concepts (FR-3.2)
    const concepts = extractConcepts(conversationText);
    await log("info", `Extracted ${concepts.length} concepts from compacting session`);
    
    // Extract decisions (FR-3.3)
    const decisions = extractDecisions(conversationText);
    await log("info", `Extracted ${decisions.length} decisions from compacting session`);
    
    // Write to memory (FR-3.4)
    const projectName = path.basename(directory);
    const payload = buildCompactionPayload(projectName, sessionID, concepts, decisions);
    const written = await writeCompactionSummary(cli, payload);
    
    if (written) {
      await log("info", `Compaction summary written to ${payload.folder}`);
    } else {
      await log("warn", "Failed to write compaction summary");
    }
    
    // Inject context (FR-3.5)
    if (concepts.length > 0 || decisions.length > 0) {
      output.context.push(buildContextInjection(concepts, decisions));
      await log("info", "Injected preservation context into compaction prompt");
    }
    
  } catch (error) {
    // Graceful degradation - don't break compaction
    await log("error", `Compaction handler error: ${error instanceof Error ? error.message : 'unknown'}`);
  }
}
```

---

## 8. Error Handling

### 8.1 Graceful Degradation

The handler SHALL NOT throw exceptions that interrupt OpenCode's compaction process:

| Failure Mode | Behavior |
|--------------|----------|
| CLI not found | Log warning, skip preservation |
| CLI timeout (>5s) | Log error, continue without write |
| JSON parse error | Log error, skip affected operation |
| Empty conversation | Log debug, exit early |
| WriteMemory failure | Log warning, still inject context |

### 8.2 Logging

Use structured logging via the OpenCode client:

```typescript
async function log(level: "debug" | "info" | "warn" | "error", message: string): Promise<void> {
  await client.app.log({
    body: {
      service: "maenifold-plugin",
      level,
      message,
      extra: { hook: "compacting" }
    }
  });
}
```

---

## 9. Testing Considerations

### 9.1 Unit Test Cases

| Test Case | Input | Expected Output |
|-----------|-------|-----------------|
| `extractConcepts` with valid WikiLinks | `"Using [[React]] and [[TypeScript]]"` | `["react", "typescript"]` |
| `extractConcepts` with nested brackets | `"See [[foo [[bar]]]]"` | `["foo [[bar"]` (partial) |
| `extractConcepts` with empty input | `""` | `[]` |
| `extractDecisions` with decision | `"We decided to use SQLite."` | `[{text: "use SQLite", ...}]` |
| `extractDecisions` with no decisions | `"Hello world"` | `[]` |

### 9.2 Integration Test Cases

| Test Case | Setup | Verification |
|-----------|-------|--------------|
| Full compaction flow | Trigger compaction with concepts | Check `memory://sessions/compaction/` |
| CLI unavailable | Unset PATH | Logs warning, no crash |
| Large conversation | 100KB+ text | Completes under 5s |

---

## 10. References

- PRD Requirements: FR-3.1 through FR-3.5
- RTM: `docs/opencode-plugin/RTM.md`
- API Reference: `memory://tech/integrations/opencode-plugin-api-reference`
- Summary Capture Pattern: `memory://tech/integrations/opencode-sessioncompacted-event-summary-capture-pattern`
