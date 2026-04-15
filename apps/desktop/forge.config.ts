import { execSync, spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { chmod } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);
const packageJson = require('./package.json') as { version: string };
const rootDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(rootDir, '..', '..');

function resolveMaenifoldBinaryPath(): string {
  const binaryName = process.platform === 'win32' ? 'maenifold.exe' : 'maenifold';
  return join(repoRoot, 'target', 'release', binaryName);
}

function resolvePackagedAppExecutablePath(): string {
  return join(
    rootDir,
    'out',
    'maenifold-darwin-arm64',
    'maenifold.app',
    'Contents',
    'MacOS',
    'maenifold',
  );
}

async function runCommand(command: string, args: string[]): Promise<void> {
  await new Promise<void>((resolvePromise, reject) => {
    const child = spawn(command, args, {
      cwd: rootDir,
      shell: process.platform === 'win32',
      stdio: 'inherit',
    });

    child.on('error', reject);
    child.on('exit', (code) => {
      if (code === 0) resolvePromise();
      else reject(new Error(`${command} ${args.join(' ')} exited with code ${code ?? 'unknown'}`));
    });
  });
}

function findRunningPackagedAppPids(): number[] {
  if (process.platform !== 'darwin') return [];

  const executablePath = resolvePackagedAppExecutablePath();
  const processList = execSync('ps -axo pid=,command=', {
    cwd: rootDir,
    encoding: 'utf8',
  });

  return processList
    .split('\n')
    .flatMap((line) => {
      const match = line.trim().match(/^(\d+)\s+(.*)$/);
      if (!match) return [];
      const pid = Number.parseInt(match[1] ?? '', 10);
      const command = match[2] ?? '';
      return Number.isFinite(pid) && command.includes(executablePath) ? [pid] : [];
    });
}

async function stopRunningPackagedApp(): Promise<void> {
  const pids = findRunningPackagedAppPids();
  if (pids.length === 0) return;

  for (const pid of pids) {
    try {
      process.kill(pid, 'SIGTERM');
    } catch {
      // Ignore races where the process exits before we signal it.
    }
  }

  for (let attempt = 0; attempt < 20; attempt += 1) {
    await new Promise((resolvePromise) => setTimeout(resolvePromise, 250));
    if (findRunningPackagedAppPids().length === 0) return;
  }

  throw new Error(
    'Timed out waiting for the existing packaged maenifold.app process to exit before packaging.',
  );
}

async function markBundledBinaryExecutable(outputPaths: string[]): Promise<void> {
  if (process.platform === 'win32') return;

  await Promise.all(
    outputPaths.map(async (outputPath) => {
      const candidate = join(outputPath, 'Contents', 'Resources', 'maenifold');
      await chmod(candidate, 0o755).catch(() => undefined);
    }),
  );
}

const config = {
  packagerConfig: {
    appVersion: packageJson.version,
    asar: true,
    executableName: 'maenifold',
    extraResource: [resolveMaenifoldBinaryPath()],
    icon: join(rootDir, 'assets', 'icons', 'icon.icns'),
    name: 'maenifold',
    // Future CI/CD work: add macOS code signing via osxSign / notarization.
  },
  hooks: {
    prePackage: async () => {
      const binaryPath = resolveMaenifoldBinaryPath();

      await stopRunningPackagedApp();
      await runCommand('npm', ['run', 'build:electron']);

      if (!existsSync(binaryPath)) {
        throw new Error(`Expected maenifold binary at ${binaryPath}. Build it with cargo build --release before packaging.`);
      }

      if (process.platform !== 'win32') {
        await chmod(binaryPath, 0o755).catch(() => undefined);
      }
    },
    postPackage: async (_forgeConfig: unknown, options: { outputPaths: string[] }) => {
      await markBundledBinaryExecutable(options.outputPaths);
    },
  },
  makers: [
    {
      name: '@electron-forge/maker-zip',
      platforms: ['darwin'],
    },
  ],
};

export default config;
