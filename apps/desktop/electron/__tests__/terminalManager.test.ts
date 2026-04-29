import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  DEFAULT_OUTPUT_BYTE_LIMIT,
  MAX_ACTIVE_TERMINALS,
  MAX_OUTPUT_BYTE_LIMIT,
  TerminalManager,
  validateCreateTerminalParams,
} from '../terminalManager';

const ptyState = vi.hoisted(() => ({
  spawned: [] as Array<{
    kill: ReturnType<typeof vi.fn>;
    onData: (callback: (chunk: string) => void) => void;
    onExit: (callback: (event: { exitCode: number; signal?: number }) => void) => void;
    emitData: (chunk: string) => void;
    emitExit: (event: { exitCode: number; signal?: number }) => void;
  }>,
}));

vi.mock('node-pty', () => ({
  spawn: vi.fn(() => {
    let onDataCallback: (chunk: string) => void = () => {};
    let onExitCallback: (event: { exitCode: number; signal?: number }) => void = () => {};
    const mockPty = {
      kill: vi.fn(),
      onData: (callback: (chunk: string) => void) => {
        onDataCallback = callback;
      },
      onExit: (callback: (event: { exitCode: number; signal?: number }) => void) => {
        onExitCallback = callback;
      },
      emitData: (chunk: string) => onDataCallback(chunk),
      emitExit: (event: { exitCode: number; signal?: number }) => onExitCallback(event),
    };
    ptyState.spawned.push(mockPty);
    return mockPty;
  }),
}));

function createManager(): TerminalManager {
  return new TerminalManager(() => ({ send: vi.fn() } as never));
}

function createValidParams(overrides: Record<string, unknown> = {}) {
  return {
    sessionId: 'session-1',
    command: 'npm',
    args: ['test'],
    env: [{ name: 'NODE_ENV', value: 'test' }],
    cwd: process.cwd(),
    ...overrides,
  };
}

describe('TerminalManager security hardening', () => {
  beforeEach(() => {
    ptyState.spawned = [];
  });

  it('rejects the 65th active terminal with TERMINAL_LIMIT_EXCEEDED', () => {
    const manager = createManager();

    for (let index = 0; index < MAX_ACTIVE_TERMINALS; index += 1) {
      manager.createTerminal({ command: 'echo', args: ['ok'] });
    }

    expect(() => manager.createTerminal({ command: 'echo', args: ['overflow'] })).toThrow(
      expect.objectContaining({ code: 'TERMINAL_LIMIT_EXCEEDED' })
    );
  });

  it('defaults omitted outputByteLimit to 1 MiB', () => {
    const manager = createManager();
    const terminalId = manager.createTerminal({ command: 'echo' });
    ptyState.spawned[0]?.emitData('x'.repeat(DEFAULT_OUTPUT_BYTE_LIMIT + 1));

    expect(manager.getOutput(terminalId)).toMatchObject({
      output: 'x'.repeat(DEFAULT_OUTPUT_BYTE_LIMIT),
      truncated: true,
    });
  });

  it('rejects outputByteLimit values over 100 MiB', () => {
    const manager = createManager();

    expect(() =>
      manager.createTerminal({
        command: 'echo',
        outputByteLimit: MAX_OUTPUT_BYTE_LIMIT + 1,
      })
    ).toThrow(
      expect.objectContaining({
        code: 'INVALID_TERMINAL_PARAMS',
        field: 'outputByteLimit',
      })
    );
  });

  it('dispose kills active PTYs', () => {
    const manager = createManager();
    manager.createTerminal({ command: 'echo', args: ['one'] });
    manager.createTerminal({ command: 'echo', args: ['two'] });

    manager.dispose();

    expect(ptyState.spawned[0]?.kill).toHaveBeenCalledOnce();
    expect(ptyState.spawned[1]?.kill).toHaveBeenCalledOnce();
  });
});

describe('terminal IPC parameter validation', () => {
  it('rejects invalid command values', () => {
    for (const command of [undefined, '', '   ']) {
      expect(() => validateCreateTerminalParams(createValidParams({ command }))).toThrow(
        expect.objectContaining({
          code: 'INVALID_TERMINAL_PARAMS',
          field: 'command',
        })
      );
    }
  });

  it('rejects invalid args values', () => {
    for (const args of ['not-array', ['ok', 1]]) {
      expect(() => validateCreateTerminalParams(createValidParams({ args }))).toThrow(
        expect.objectContaining({
          code: 'INVALID_TERMINAL_PARAMS',
          field: 'args',
        })
      );
    }
  });

  it('rejects invalid env entries', () => {
    for (const env of ['not-array', [{ name: 'OK' }], [{ name: 42, value: 'x' }]]) {
      expect(() => validateCreateTerminalParams(createValidParams({ env }))).toThrow(
        expect.objectContaining({
          code: 'INVALID_TERMINAL_PARAMS',
          field: 'env',
        })
      );
    }
  });

  it('rejects non-absolute cwd values', () => {
    expect(() => validateCreateTerminalParams(createValidParams({ cwd: 'relative/path' }))).toThrow(
      expect.objectContaining({
        code: 'INVALID_TERMINAL_PARAMS',
        field: 'cwd',
      })
    );
  });

  it('rejects non-existent cwd values', () => {
    expect(() =>
      validateCreateTerminalParams(createValidParams({ cwd: '/definitely/not/a/real/maenifold/path' }))
    ).toThrow(
      expect.objectContaining({
        code: 'INVALID_TERMINAL_PARAMS',
        field: 'cwd',
      })
    );
  });

  it('rejects invalid outputByteLimit values', () => {
    for (const outputByteLimit of [Number.POSITIVE_INFINITY, Number.NaN, -1, 0, 1.5, MAX_OUTPUT_BYTE_LIMIT + 1]) {
      expect(() => validateCreateTerminalParams(createValidParams({ outputByteLimit }))).toThrow(
        expect.objectContaining({
          code: 'INVALID_TERMINAL_PARAMS',
          field: 'outputByteLimit',
        })
      );
    }
  });
});
