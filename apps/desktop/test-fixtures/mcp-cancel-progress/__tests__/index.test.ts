import { describe, expect, it, vi } from 'vitest';

import {
  failing,
  listTools,
  longRunning,
  type ProgressEvent,
} from '../index';

type McpContentBlock = {
  type: string;
  text?: string;
};

type McpToolResult = {
  content: McpContentBlock[];
  isError?: boolean;
};

type McpToolDescriptor = {
  name: string;
  description: string;
  inputSchema: {
    type: 'object';
    properties: Record<string, unknown>;
    required?: string[];
  };
};

const expectTextContent = (result: McpToolResult, expectedText: string) => {
  expect(result).toMatchObject({
    content: expect.arrayContaining([
      expect.objectContaining({
        type: 'text',
        text: expect.stringContaining(expectedText),
      }),
    ]),
  });
};

describe('mcp cancel/progress fixture', () => {
  it('describes the MCP tools with JSON-schema inputs', () => {
    const tools = listTools();

    expect(tools).toHaveLength(2);
    expect(tools).toEqual(
      expect.arrayContaining<McpToolDescriptor>([
        expect.objectContaining({
          name: 'long_running',
          description: expect.any(String),
          inputSchema: expect.objectContaining({
            type: 'object',
            properties: expect.objectContaining({
              seconds: expect.objectContaining({ type: 'number' }),
            }),
            required: ['seconds'],
          }),
        }),
        expect.objectContaining({
          name: 'failing',
          description: expect.any(String),
          inputSchema: expect.objectContaining({
            type: 'object',
            properties: expect.objectContaining({
              message: expect.objectContaining({ type: 'string' }),
            }),
            required: ['message'],
          }),
        }),
      ]),
    );
  });

  it('longRunning(2) emits at least three progress notifications then resolves', async () => {
    vi.useFakeTimers();
    const progress: ProgressEvent[] = [];

    try {
      const run = longRunning(2, {
        onProgress: (event: ProgressEvent) => progress.push(event),
      });

      await vi.advanceTimersByTimeAsync(2_100);
      const result = await run;

      expect(progress.length).toBeGreaterThanOrEqual(3);
      expect(progress.map((event) => event.method)).toEqual(
        expect.arrayContaining(['notifications/progress']),
      );
      expect(progress.every((event) => event.params.total === 2_000)).toBe(true);
      expect(progress.at(-1)?.params.completed).toBeGreaterThanOrEqual(2_000);
      expect(result).toMatchObject({ isError: false });
      expectTextContent(result, 'completed');
    } finally {
      vi.useRealTimers();
    }
  });

  it('longRunning(2) honors an AbortSignal aborted after about 750ms', async () => {
    vi.useFakeTimers();
    const abortController = new AbortController();
    const progress: ProgressEvent[] = [];

    try {
      const run = longRunning(2, {
        signal: abortController.signal,
        onProgress: (event: ProgressEvent) => progress.push(event),
      });

      await vi.advanceTimersByTimeAsync(750);
      abortController.abort();
      const emittedAtAbort = progress.length;

      const settled = Promise.race([
        run.then(
          (result) => ({ status: 'resolved' as const, result }),
          (error: unknown) => ({ status: 'rejected' as const, error }),
        ),
        new Promise<{ status: 'timeout' }>((resolve) => {
          setTimeout(() => resolve({ status: 'timeout' }), 1_500);
        }),
      ]);

      await vi.advanceTimersByTimeAsync(1_500);
      const outcome = await settled;

      expect(outcome.status).not.toBe('timeout');
      expect(progress).toHaveLength(emittedAtAbort);
      expect(emittedAtAbort).toBeGreaterThanOrEqual(1);
      expect(emittedAtAbort).toBeLessThan(3);
    } finally {
      vi.useRealTimers();
    }
  });

  it('failing("oops") returns an MCP error result with message content', async () => {
    const result = await failing('oops');

    expect(result).toMatchObject({
      isError: true,
      content: expect.arrayContaining([
        expect.objectContaining({
          type: 'text',
          text: expect.stringContaining('oops'),
        }),
      ]),
    });
  });
});
