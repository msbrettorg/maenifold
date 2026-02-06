# Product Requirements Document: OpenCode Plugin for Maenifold

## Document Control

| Version | Date | Author | Description |
|---------|------|--------|-------------|
| 1.0 | 2026-01-27 | PM Agent | Initial PRD |

---

## 1. Executive Summary

**Product Name**: maenifold-opencode-plugin
**Product Type**: TypeScript Plugin for OpenCode CLI
**Target Release**: Q1 2026

This plugin integrates maenifold's knowledge graph and reasoning infrastructure with OpenCode, providing the same capabilities currently available in Claude Code. The plugin enables persistent context engineering through graph-based retrieval, automatic context injection, and session compaction preservation.

### Vision Statement

Enable OpenCode users to leverage maenifold's knowledge graph for:
- Automatic context injection from the knowledge graph at session start
- Concept-based retrieval when tools are invoked with `[[WikiLinks]]`
- Preservation of session knowledge before context compaction

### Feature Parity Goal

Achieve functional parity with the Claude Code `plugin-maenifold` integration, translating bash hook scripts to TypeScript event handlers.

---

## 2. Problem Statement

### Current Situation

OpenCode is an emerging AI coding assistant that supports plugins via TypeScript/JavaScript modules. Maenifold currently integrates with Claude Code via:
- Bash hook scripts for event handling
- Skills for documentation injection

OpenCode users cannot currently benefit from maenifold's knowledge graph capabilities.

### Opportunity

By creating an OpenCode plugin, we:
1. Expand maenifold's reach to OpenCode users
2. Validate the portability of our integration patterns
3. Establish a TypeScript-native implementation that may inform future Claude Code improvements

### Constraints

1. OpenCode plugins must be TypeScript/JavaScript (no bash scripts)
2. Event model differs from Claude Code hooks
3. Plugin API is still maturing (some hooks are `experimental.*`)
4. Must call `maenifold` CLI for graph operations (same as Claude Code)

---

## 3. User Personas

### 3.1 OpenCode Power User

**Name**: Riley
**Role**: Senior Developer using OpenCode for daily coding
**Goals**:
- Build persistent knowledge that compounds across sessions
- Have relevant context automatically available when starting work
- Tag concepts in responses for future retrieval

**Pain Points**:
- Loses context between OpenCode sessions
- Manually rebuilds understanding each session
- No structured way to persist learnings

---

## 4. Functional Requirements

### 4.1 Core Plugin Infrastructure

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR-1.1 | Plugin SHALL export a default plugin function compatible with OpenCode's plugin API | P0 | TypeScript module |
| FR-1.2 | Plugin SHALL locate `maenifold` CLI binary via PATH or MAENIFOLD_ROOT env var | P0 | Same logic as bash hooks |
| FR-1.3 | Plugin SHALL gracefully degrade when maenifold CLI is unavailable | P0 | No errors, empty context |
| FR-1.4 | Plugin SHALL execute CLI commands with configurable timeout (default 5s) | P0 | Prevent hanging |

### 4.2 Session Start Hook

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR-2.1 | Plugin SHALL subscribe to `session.created` event | P0 | OpenCode equivalent of SessionStart |
| FR-2.2 | Plugin SHALL search memories for current repo/project name | P0 | `searchmemories` |
| FR-2.3 | Plugin SHALL retrieve recent activity concepts (last 24h) | P0 | `recentactivity` |
| FR-2.4 | Plugin SHALL build context for merged concept list | P0 | `buildcontext` |
| FR-2.5 | Plugin SHALL inject context into session via `event` hook response | P0 | OpenCode event pattern |
| FR-2.6 | Plugin SHALL respect TOKEN_LIMIT config (default 4000) | P1 | Prevent context bloat |

### 4.3 Compaction Hook

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR-3.1 | Plugin SHALL subscribe to `experimental.session.compacting` event | P0 | Pre-compaction hook |
| FR-3.2 | Plugin SHALL extract `[[WikiLinks]]` from conversation | P0 | WikiLink regex |
| FR-3.3 | Plugin SHALL extract decision patterns from conversation | P1 | Heuristic grep |
| FR-3.4 | Plugin SHALL write compaction summary to `memory://sessions/compaction/` | P0 | `writememory` |
| FR-3.5 | Plugin SHALL inject custom context into compaction prompt via `output.context` | P1 | Preserve domain knowledge |

### 4.4 Tool Execution Hook (Concept Augmentation)

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR-4.1 | Plugin SHALL subscribe to `tool.execute.before` event | P0 | Pre-tool hook |
| FR-4.2 | Plugin SHALL detect `[[WikiLinks]]` in tool arguments | P0 | Any tool, not just Task |
| FR-4.3 | Plugin SHALL build context for detected concepts | P0 | `buildcontext` |
| FR-4.4 | Plugin SHALL augment tool input with graph context | P0 | Modify output.args |
| FR-4.5 | Plugin SHALL skip augmentation for tools with no text arguments | P1 | Performance |

---

## 5. Non-Functional Requirements

### 5.1 Performance

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-1.1 | CLI command timeout | 5 seconds max |
| NFR-1.2 | Session start hook latency | < 2 seconds total |
| NFR-1.3 | Tool augmentation latency | < 1 second per concept |

### 5.2 Reliability

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-2.1 | Graceful degradation on CLI unavailable | Required |
| NFR-2.2 | No unhandled exceptions from plugin | Required |
| NFR-2.3 | JSON parsing error handling | Required |

### 5.3 Compatibility

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-3.1 | OpenCode version | 1.0+ |
| NFR-3.2 | maenifold CLI version | 1.0.4+ |
| NFR-3.3 | Node.js/Bun runtime | Bun (OpenCode's runtime) |

---

## 6. Technical Architecture

### 6.1 Plugin Structure

```
~/.config/opencode/plugins/
└── maenifold.ts          # Main plugin file (TypeScript)
```

### 6.2 Event Mapping

| Claude Code Hook | OpenCode Event | Purpose |
|------------------|----------------|---------|
| `SessionStart` | `session.created` | Inject graph context |
| `PreCompact` | `experimental.session.compacting` | Preserve knowledge |
| `PreToolUse` (Task) | `tool.execute.before` | Concept augmentation |
| `SubagentStop` | N/A | Not available in OpenCode |

### 6.3 CLI Interaction Pattern

```typescript
async function runMaenifoldTool(tool: string, payload: object): Promise<string> {
  const cli = findMaenifoldCli();
  if (!cli) return "";
  
  const proc = Bun.spawn([cli, "--tool", tool, "--payload", JSON.stringify(payload)]);
  const output = await Promise.race([
    new Response(proc.stdout).text(),
    new Promise<string>((_, reject) => 
      setTimeout(() => reject(new Error("timeout")), 5000)
    )
  ]);
  
  return output;
}
```

### 6.4 Data Flow

```
┌─────────────────────────────────────────────────────────────┐
│                      OpenCode Session                        │
├─────────────────────────────────────────────────────────────┤
│  1. session.created                                          │
│     └─► Plugin: SearchMemories + RecentActivity + BuildContext│
│         └─► Inject context into session                       │
│                                                               │
│  2. User types message with [[WikiLinks]]                      │
│     └─► tool.execute.before                                  │
│         └─► Plugin: Extract [[WikiLinks]], BuildContext        │
│             └─► Augment tool args with graph context          │
│                                                               │
│  3. Context approaching limit                                 │
│     └─► experimental.session.compacting                       │
│         └─► Plugin: Extract concepts + decisions              │
│             └─► WriteMemory to sessions/compaction/           │
│             └─► Inject summary into compaction prompt         │
└─────────────────────────────────────────────────────────────┘
```

---

## 7. Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Session start context injection | Working | Manual test |
| Compaction memory persistence | Working | Check `memory://sessions/compaction/` |
| Concept augmentation | Working | Verify `[[WikiLinks]]` trigger BuildContext |
| No errors in OpenCode logs | Zero | Console output |

---

## 8. Risks & Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| OpenCode plugin API changes | High | Medium | Pin to stable API, monitor releases |
| `experimental.*` hooks removed | High | Low | Use stable events where possible |
| CLI timeout causing hangs | Medium | Medium | Strict timeout enforcement |
| Bun-specific APIs not portable | Low | Low | Use standard Node.js patterns |

---

## 9. Out of Scope (v1)

1. SubagentStop hook (no equivalent in OpenCode)
2. Custom UI components
3. Plugin distribution via npm (local file only for v1)

---

## 10. Dependencies

| Dependency | Owner | Status |
|------------|-------|--------|
| OpenCode 1.0+ | Anomaly | Released |
| maenifold CLI | ma-collective | Required in PATH |
| Bun runtime | OpenCode | Included |

---

## 11. References

- [OpenCode Plugins Documentation](https://opencode.ai/docs/plugins/)
- [Claude Code plugin-maenifold](../integrations/claude-code/plugin-maenifold/)
- [Maenifold CLI Documentation](./SCRIPTING.md)
