# WorkIQ Integration

Bridges Microsoft 365 Copilot (via the [WorkIQ](https://workiq.dev) CLI) with maenifold's SequentialThinking tool. Queries M365 data and feeds the response through a multi-thought analysis session, producing a persistent reasoning chain in the knowledge graph.

## Usage

```bash
./think.sh "What meetings do I have this week about cost optimization?"
```

With custom thought count:

```bash
./think.sh -t 5 "Summarize my recent emails about the Azure migration"
```

The script outputs the session ID to stdout and progress/results to stderr:

```
session-1771302756428-12345
---
M365: <response from WorkIQ>
View: maenifold --tool ReadMemory --payload '{"identifier":"memory://thinking/sequential/session-1771302756428-12345"}'
```

## How It Works

1. Queries M365 via `workiq ask` with your question.
2. Injects the M365 response into thought 0 of a new SequentialThinking session with `[[WikiLink]]` tags for graph integration.
3. Runs N-2 middle analysis thoughts (configurable), each referencing the WorkIQ data and tagging `[[M365-Copilot]]`, `[[knowledge-graph]]`.
4. Produces a final conclusion thought that closes the session.
5. Prints the session ID for follow-up via `ReadMemory`.

## Prerequisites

- **workiq CLI** installed and authenticated (`workiq ask` must work).
- **maenifold binary** in `PATH` (`which maenifold` should resolve).
- **python3** in `PATH` (used for JSON escaping).

## Environment Variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `WORKIQ_THINK_THOUGHTS` | `3` | Total number of thoughts in the analysis session (minimum 2: initial + conclusion). Override with `-t` flag or this env var. |
