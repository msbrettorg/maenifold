# Requirements Traceability Matrix: OpenCode Plugin

## FR-1.x: Core Plugin Infrastructure

| ID | Requirement | Component | Test | Status |
|----|-------------|-----------|------|--------|
| FR-1.1 | Plugin SHALL export a default plugin function compatible with OpenCode's plugin API | SPEC-1-core.md §2.1 | TC-1.1 | Specified |
| FR-1.2 | Plugin SHALL locate `maenifold` CLI binary via PATH or MAENIFOLD_ROOT env var | SPEC-1-core.md §2.2, §5.1 | TC-1.2 | Specified |
| FR-1.3 | Plugin SHALL gracefully degrade when maenifold CLI is unavailable | SPEC-1-core.md §2.3, §4.4 | TC-1.3 | Specified |
| FR-1.4 | Plugin SHALL execute CLI commands with configurable timeout (default 5s) | SPEC-1-core.md §2.4, §5.5 | TC-1.4 | Specified |

## FR-2.x: Session Start Hook

**Specification**: [SPEC-2-session.md](SPEC-2-session.md)

| ID | Requirement | Component | Test | Status |
|----|-------------|-----------|------|--------|
| FR-2.1 | Plugin SHALL subscribe to `session.created` event | SPEC-2-session.md §2 handleSessionStart | T-2.1.1 | Specified |
| FR-2.2 | Plugin SHALL search memories for current repo/project name | SPEC-2-session.md §4.1 runMaenifoldTool | T-2.2.1 | Specified |
| FR-2.3 | Plugin SHALL retrieve recent activity concepts (last 24h) | SPEC-2-session.md §4.2 runMaenifoldTool | T-2.3.1 | Specified |
| FR-2.4 | Plugin SHALL build context for merged concept list | SPEC-2-session.md §4.3 runMaenifoldTool | T-2.4.1 | Specified |
| FR-2.5 | Plugin SHALL inject context into session via `client.session.prompt({ noReply: true })` | SPEC-2-session.md §5.1 injectContext | T-2.5.1 | Specified |
| FR-2.6 | Plugin SHALL respect TOKEN_LIMIT config (default 4000) | SPEC-2-session.md §5.3 enforceTokenLimit | T-2.6.1 | Specified |

## FR-3.x: Compaction Hook

| ID | Requirement | Component | Test | Status |
|----|-------------|-----------|------|--------|
| FR-3.1 | Plugin SHALL subscribe to `experimental.session.compacting` event | SPEC-3-compaction.md | | Specified |
| FR-3.2 | Plugin SHALL extract WikiLinks like `[[authentication]]`, `[[database]]` from conversation | SPEC-3-compaction.md | | Specified |
| FR-3.3 | Plugin SHALL extract decision patterns from conversation | SPEC-3-compaction.md | | Specified |
| FR-3.4 | Plugin SHALL write compaction summary to `memory://sessions/compaction/` | SPEC-3-compaction.md | | Specified |
| FR-3.5 | Plugin SHALL inject custom context into compaction prompt via `output.context` | SPEC-3-compaction.md | | Specified |

## FR-4.x: Tool Execution Hook

| ID | Requirement | Component | Test | Status |
|----|-------------|-----------|------|--------|
| FR-4.1 | Plugin SHALL subscribe to `tool.execute.before` event | SPEC-4-tools.md | | Specified |
| FR-4.2 | Plugin SHALL detect WikiLinks like `[[API-design]]`, `[[error-handling]]` in tool arguments | SPEC-4-tools.md | | Specified |
| FR-4.3 | Plugin SHALL build context for detected concepts | SPEC-4-tools.md | | Specified |
| FR-4.4 | Plugin SHALL augment tool input with graph context | SPEC-4-tools.md | | Specified |
| FR-4.5 | Plugin SHALL skip augmentation for tools with no text arguments | SPEC-4-tools.md | | Specified |

## NFR-1.x: Performance

| ID | Requirement | Target | Test | Status |
|----|-------------|--------|------|--------|
| NFR-1.1 | CLI command timeout | 5s max | | |
| NFR-1.2 | Session start hook latency | < 2s | | |
| NFR-1.3 | Tool augmentation latency | < 1s per concept | | |

## NFR-2.x: Reliability

| ID | Requirement | Target | Test | Status |
|----|-------------|--------|------|--------|
| NFR-2.1 | Graceful degradation on CLI unavailable | Required | | |
| NFR-2.2 | No unhandled exceptions from plugin | Required | | |
| NFR-2.3 | JSON parsing error handling | Required | | |
