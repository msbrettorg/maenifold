import type { WebContents } from 'electron';
import { statSync } from 'node:fs';
import { isAbsolute } from 'node:path';
import * as pty from 'node-pty';

export const MAX_ACTIVE_TERMINALS = 64;
export const DEFAULT_OUTPUT_BYTE_LIMIT = 1024 * 1024;
export const MAX_OUTPUT_BYTE_LIMIT = 100 * 1024 * 1024;

type EnvVariable = { name: string; value: string };

export type TerminalExitStatus = {
  exitCode?: number | null;
  signal?: string | null;
};

export type CreateTerminalParams = {
  command: string;
  args?: string[];
  env?: EnvVariable[];
  cwd?: string | null;
  outputByteLimit?: number | null;
};

export type TerminalOperationParams = {
  sessionId: string;
  terminalId: string;
};

export type TerminalOutput = {
  output: string;
  truncated: boolean;
  exitStatus?: TerminalExitStatus | null;
};

export type CreateTerminalResult = {
  terminalId: string;
  outputByteLimit: number;
};

type TerminalEntry = {
  pty: pty.IPty;
  output: string;
  truncated: boolean;
  outputByteLimit?: number;
  exitStatus: TerminalExitStatus | null;
  released: boolean;
  exitPromise: Promise<TerminalExitStatus>;
  resolveExit: (status: TerminalExitStatus) => void;
};

const encoder = new TextEncoder();

export class TerminalLimitError extends Error {
  readonly code = 'TERMINAL_LIMIT_EXCEEDED';

  constructor() {
    super(`Terminal limit exceeded: maximum ${MAX_ACTIVE_TERMINALS} active terminals`);
  }
}

export class InvalidTerminalParamsError extends Error {
  readonly code = 'INVALID_TERMINAL_PARAMS';

  constructor(
    readonly field: string,
    readonly reason: string
  ) {
    super(`Invalid terminal params: ${field} ${reason}`);
  }
}

function invalid(field: string, reason: string): never {
  throw new InvalidTerminalParamsError(field, reason);
}

function normalizeByteLimit(limit: number | null | undefined): number {
  if (limit == null) return DEFAULT_OUTPUT_BYTE_LIMIT;
  if (!Number.isFinite(limit) || !Number.isInteger(limit) || limit <= 0) {
    invalid('outputByteLimit', 'must be a finite positive integer');
  }
  if (limit > MAX_OUTPUT_BYTE_LIMIT) {
    invalid('outputByteLimit', `must be <= ${MAX_OUTPUT_BYTE_LIMIT}`);
  }
  return limit;
}

function validateSessionId(params: Record<string, unknown>): string {
  if (typeof params.sessionId !== 'string' || params.sessionId.trim().length === 0) {
    invalid('sessionId', 'must be a non-empty string');
  }
  return params.sessionId.trim();
}

function validateTerminalId(params: Record<string, unknown>): string {
  if (typeof params.terminalId !== 'string' || params.terminalId.trim().length === 0) {
    invalid('terminalId', 'must be a non-empty string');
  }
  return params.terminalId.trim();
}

function asRecord(params: unknown): Record<string, unknown> {
  if (typeof params !== 'object' || params === null || Array.isArray(params)) {
    invalid('params', 'must be an object');
  }
  return params as Record<string, unknown>;
}

export function validateCreateTerminalParams(params: unknown): CreateTerminalParams & { sessionId: string } {
  const record = asRecord(params);
  const sessionId = validateSessionId(record);
  if (typeof record.command !== 'string' || record.command.trim().length === 0) {
    invalid('command', 'must be a non-empty string');
  }

  const args = record.args;
  if (args != null && (!Array.isArray(args) || args.some((arg) => typeof arg !== 'string'))) {
    invalid('args', 'must be an array of strings');
  }

  const env = record.env;
  if (
    env != null &&
    (!Array.isArray(env) ||
      env.some(
        (entry) =>
          typeof entry !== 'object' ||
          entry === null ||
          Array.isArray(entry) ||
          typeof (entry as { name?: unknown }).name !== 'string' ||
          typeof (entry as { value?: unknown }).value !== 'string'
      ))
  ) {
    invalid('env', 'must be an array of { name: string, value: string } entries');
  }

  let cwd: string | null | undefined;
  if (record.cwd != null) {
    if (typeof record.cwd !== 'string' || !isAbsolute(record.cwd)) {
      invalid('cwd', 'must be an absolute path string');
    }
    try {
      if (!statSync(record.cwd).isDirectory()) {
        invalid('cwd', 'must exist and be a directory');
      }
    } catch {
      invalid('cwd', 'must exist and be a directory');
    }
    cwd = record.cwd;
  }

  const outputByteLimit = normalizeByteLimit(record.outputByteLimit as number | null | undefined);

  return {
    sessionId,
    command: record.command.trim(),
    args: args == null ? [] : args,
    env: env == null ? [] : env as EnvVariable[],
    cwd,
    outputByteLimit,
  };
}

export function validateTerminalOperationParams(params: unknown): TerminalOperationParams {
  const record = asRecord(params);
  return {
    sessionId: validateSessionId(record),
    terminalId: validateTerminalId(record),
  };
}

function truncateUtf8FromFront(output: string, limit: number): { output: string; truncated: boolean } {
  const bytes = encoder.encode(output);
  if (bytes.byteLength <= limit) return { output, truncated: false };
  if (limit === 0) return { output: '', truncated: true };

  const firstCandidate = Math.max(0, bytes.byteLength - limit);
  for (let start = firstCandidate; start < bytes.byteLength; start += 1) {
    try {
      return {
        output: new TextDecoder('utf-8', { fatal: true }).decode(bytes.slice(start)),
        truncated: true,
      };
    } catch {
      // Keep moving forward until the suffix starts at a valid UTF-8 boundary.
    }
  }

  return { output: '', truncated: true };
}

function buildEnv(env: CreateTerminalParams['env']): Record<string, string> {
  const base: Record<string, string> = {};
  for (const [key, value] of Object.entries(process.env)) {
    if (value != null) base[key] = value;
  }
  // Trust model: a connected local ACP agent is trusted to inherit and override
  // environment variables. Do not sanitize PATH or allowlist env keys here.
  if (env) {
    for (const item of env) {
      base[item.name] = item.value;
    }
  }
  return base;
}

export class TerminalManager {
  private readonly terminals = new Map<string, TerminalEntry>();
  private nextTerminalId = 0;

  constructor(private readonly getWebContents: () => WebContents | null) {}

  createTerminal({ command, args = [], env, cwd, outputByteLimit }: CreateTerminalParams): CreateTerminalResult {
    if (typeof command !== 'string' || command.trim().length === 0) {
      throw new Error('terminal/create requires a command');
    }
    if (this.terminals.size >= MAX_ACTIVE_TERMINALS) {
      throw new TerminalLimitError();
    }

    const normalizedOutputByteLimit = normalizeByteLimit(outputByteLimit);

    const terminalId = `term-${Date.now().toString(36)}-${(this.nextTerminalId += 1).toString(36)}`;
    let resolveExit: (status: TerminalExitStatus) => void = () => {};
    const exitPromise = new Promise<TerminalExitStatus>((resolve) => {
      resolveExit = resolve;
    });
    const ptyProcess = pty.spawn(command, args, {
      cwd: cwd || process.cwd(),
      env: buildEnv(env),
    });
    const entry: TerminalEntry = {
      pty: ptyProcess,
      output: '',
      truncated: false,
      outputByteLimit: normalizedOutputByteLimit,
      exitStatus: null,
      released: false,
      exitPromise,
      resolveExit,
    };

    this.terminals.set(terminalId, entry);
    ptyProcess.onData((chunk) => {
      this.appendOutput(terminalId, chunk);
      this.getWebContents()?.send('terminal:output', terminalId, chunk);
    });
    ptyProcess.onExit(({ exitCode, signal }) => {
      const exitStatus = {
        exitCode,
        signal: signal == null || signal === 0 ? null : String(signal),
      };
      entry.exitStatus = exitStatus;
      entry.resolveExit(exitStatus);
    });

    return { terminalId, outputByteLimit: entry.outputByteLimit };
  }

  getOutput(terminalId: string): TerminalOutput {
    const entry = this.requireTerminal(terminalId);
    return {
      output: entry.output,
      truncated: entry.truncated,
      exitStatus: entry.exitStatus,
    };
  }

  kill(terminalId: string): Record<string, never> {
    const entry = this.requireTerminal(terminalId);
    if (!entry.exitStatus) {
      entry.pty.kill();
    }
    return {};
  }

  release(terminalId: string): Record<string, never> {
    const entry = this.requireTerminal(terminalId);
    if (!entry.exitStatus) {
      entry.pty.kill();
    }
    entry.released = true;
    this.terminals.delete(terminalId);
    return {};
  }

  async waitForExit(terminalId: string): Promise<TerminalExitStatus> {
    const entry = this.requireTerminal(terminalId);
    if (entry.exitStatus) return entry.exitStatus;
    return entry.exitPromise;
  }

  dispose(): void {
    for (const entry of this.terminals.values()) {
      try {
        entry.pty.kill();
      } catch {
        // Ignore races where the PTY has already exited.
      }
      entry.released = true;
    }
    this.terminals.clear();
  }

  private appendOutput(terminalId: string, chunk: string): void {
    const entry = this.terminals.get(terminalId);
    if (!entry) return;
    const appended = `${entry.output}${chunk}`;
    const truncated = truncateUtf8FromFront(appended, entry.outputByteLimit);
    entry.output = truncated.output;
    entry.truncated ||= truncated.truncated;
  }

  private requireTerminal(terminalId: string): TerminalEntry {
    const entry = this.terminals.get(terminalId);
    if (!entry) {
      throw new Error(`Unknown terminalId: ${terminalId}`);
    }
    return entry;
  }
}
