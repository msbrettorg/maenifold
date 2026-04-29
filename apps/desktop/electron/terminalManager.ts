import type { WebContents } from 'electron';
import * as pty from 'node-pty';

type EnvVariable = { name: string; value: string };

export type TerminalExitStatus = {
  exitCode?: number | null;
  signal?: string | null;
};

export type CreateTerminalParams = {
  command: string;
  args?: string[];
  env?: EnvVariable[] | Record<string, string | undefined>;
  cwd?: string | null;
  outputByteLimit?: number | null;
};

export type TerminalOutput = {
  output: string;
  truncated: boolean;
  exitStatus?: TerminalExitStatus | null;
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

function normalizeByteLimit(limit: number | null | undefined): number | undefined {
  return typeof limit === 'number' && Number.isFinite(limit) && limit >= 0
    ? Math.floor(limit)
    : undefined;
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
  if (Array.isArray(env)) {
    for (const item of env) {
      base[item.name] = item.value;
    }
    return base;
  }
  if (env) {
    for (const [key, value] of Object.entries(env)) {
      if (value == null) delete base[key];
      else base[key] = value;
    }
  }
  return base;
}

export class TerminalManager {
  private readonly terminals = new Map<string, TerminalEntry>();
  private nextTerminalId = 0;

  constructor(private readonly getWebContents: () => WebContents | null) {}

  createTerminal({ command, args = [], env, cwd, outputByteLimit }: CreateTerminalParams): string {
    if (typeof command !== 'string' || command.trim().length === 0) {
      throw new Error('terminal/create requires a command');
    }

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
      outputByteLimit: normalizeByteLimit(outputByteLimit),
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

    return terminalId;
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

  private appendOutput(terminalId: string, chunk: string): void {
    const entry = this.terminals.get(terminalId);
    if (!entry) return;
    const appended = `${entry.output}${chunk}`;
    if (entry.outputByteLimit == null) {
      entry.output = appended;
      return;
    }
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
