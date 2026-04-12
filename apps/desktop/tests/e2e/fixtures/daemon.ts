import { expect, test as base } from '@playwright/test';
import { once } from 'node:events';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawn, type ChildProcess } from 'node:child_process';
import { setTimeout as delay } from 'node:timers/promises';
import { waitForDaemon } from '../helpers/wait-for-daemon';

const fixtureDirectory = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(fixtureDirectory, '../../../../../');
const binaryName = process.platform === 'win32' ? 'maenifold.exe' : 'maenifold';

export const daemonBaseUrl =
  process.env.PLAYWRIGHT_DAEMON_URL ??
  process.env.VITE_DAEMON_URL ??
  'http://127.0.0.1:19761';

type DaemonFixtures = {
  daemonBaseUrl: string;
};

function daemonPort(): string {
  const parsed = new URL(daemonBaseUrl);
  if (parsed.port) {
    return parsed.port;
  }

  return parsed.protocol === 'https:' ? '443' : '80';
}

function resolveDaemonCommand(): { command: string; args: string[] } {
  const configuredBinary =
    process.env.PLAYWRIGHT_MAENIFOLD_BIN ??
    process.env.MAENIFOLD_BIN;

  if (configuredBinary) {
    return {
      command: configuredBinary,
      args: ['--http', '--port', daemonPort()],
    };
  }

  const localBinary = path.join(repoRoot, 'target', 'debug', binaryName);
  if (existsSync(localBinary)) {
    return {
      command: localBinary,
      args: ['--http', '--port', daemonPort()],
    };
  }

  return {
    command: 'maenifold',
    args: ['--http', '--port', daemonPort()],
  };
}

function captureOutput(stream: NodeJS.ReadableStream | null, target: string[]): void {
  if (!stream) {
    return;
  }

  stream.setEncoding('utf8');
  stream.on('data', (chunk: string) => {
    target.push(chunk);
    if (target.length > 200) {
      target.splice(0, target.length - 200);
    }
  });
}

async function stopDaemonProcess(child: ChildProcess): Promise<void> {
  if (child.exitCode !== null) {
    return;
  }

  const exitPromise = once(child, 'exit').catch(() => undefined);
  child.kill('SIGTERM');
  const result = await Promise.race([
    exitPromise.then(() => 'exited' as const),
    delay(10_000).then(() => 'timeout' as const),
  ]);

  if (result === 'timeout' && child.exitCode === null) {
    child.kill('SIGKILL');
    await exitPromise;
  }
}

async function startDaemonProcess(): Promise<ChildProcess> {
  const { command, args } = resolveDaemonCommand();
  const stdout: string[] = [];
  const stderr: string[] = [];
  const child = spawn(command, args, {
    cwd: repoRoot,
    stdio: ['ignore', 'pipe', 'pipe'],
    env: {
      ...process.env,
      RUST_LOG: process.env.RUST_LOG ?? 'warn',
    },
  });

  captureOutput(child.stdout, stdout);
  captureOutput(child.stderr, stderr);

  const exitPromise = Promise.race([
    once(child, 'error').then(([error]) => {
      throw error;
    }),
    once(child, 'exit').then(([code, signal]) => {
      throw new Error(
        `maenifold exited before readiness check completed (code=${String(code)}, signal=${String(signal)})`,
      );
    }),
  ]);

  try {
    await Promise.race([
      waitForDaemon(daemonBaseUrl),
      exitPromise,
    ]);
    return child;
  } catch (error) {
    await stopDaemonProcess(child);
    const logs = [...stdout, ...stderr].join('').trim();
    const detail = logs ? `\n\nRecent daemon output:\n${logs}` : '';
    throw new Error(
      `Failed to start maenifold daemon using "${command} ${args.join(' ')}": ${error instanceof Error ? error.message : String(error)}${detail}`,
    );
  }
}

export const test = base.extend<DaemonFixtures>({
  daemonBaseUrl: [
    async ({}, use) => {
      const child = await startDaemonProcess();
      try {
        await use(daemonBaseUrl);
      } finally {
        await stopDaemonProcess(child);
      }
    },
    { scope: 'worker', auto: true, timeout: 120_000 },
  ],
});

export { expect };
