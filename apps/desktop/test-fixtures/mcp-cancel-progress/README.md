# MCP cancel/progress fixture

This directory contains the local TypeScript MCP fixture for T-303.16. It is
used by Vitest to pin the behavior needed by desktop cancellation and progress
tests before the fixture implementation is added.

## Expected fixture API

The SWE implementation should add `index.ts` next to this README and satisfy
the tests under `__tests__/`.

Required exports:

- `listTools()` returns MCP-shaped tool descriptors for `long_running` and
  `failing`.
- `longRunning(seconds, options)` runs for the requested number of seconds,
  emits `notifications/progress` events about every 500 ms through
  `options.onProgress`, and honors `options.signal` cancellation.
- `failing(message)` returns an MCP tool result with `isError: true` and a text
  content block that includes the supplied message.

Acceptance criteria:

- `longRunning(2)` emits at least three progress notifications and resolves.
- An abort around 750 ms stops new progress events and settles within 1.5 s.
- `failing("oops")` reports an MCP error result containing `oops`.
- Tool descriptors and result content blocks match the MCP object shape used in
  the tests.
- Do not add the task-augmented tool in this iteration; it is not part of this
  fixture contract yet.
