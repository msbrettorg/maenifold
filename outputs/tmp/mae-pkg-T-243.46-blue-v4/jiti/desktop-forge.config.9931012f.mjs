"use strict";Object.defineProperty(exports, "__esModule", { value: true });exports.default = void 0;var _nodeChild_process = await jitiImport("node:child_process");
var _nodeFs = await jitiImport("node:fs");
var _promises = await jitiImport("node:fs/promises");
var _nodeModule = await jitiImport("node:module");
var _nodePath = await jitiImport("node:path");
var _nodeUrl = await jitiImport("node:url");

const _require = (0, _nodeModule.createRequire)("file:///Users/brett/src/ma-collective/maenifold-rs/apps/desktop/forge.config.ts");
const packageJson = _require('./package.json');
const rootDir = (0, _nodePath.dirname)((0, _nodeUrl.fileURLToPath)("file:///Users/brett/src/ma-collective/maenifold-rs/apps/desktop/forge.config.ts"));
const repoRoot = (0, _nodePath.resolve)(rootDir, '..', '..');

function resolveMaenifoldBinaryPath() {
  const binaryName = process.platform === 'win32' ? 'maenifold.exe' : 'maenifold';
  return (0, _nodePath.join)(repoRoot, 'target', 'release', binaryName);
}

function resolvePackagedAppExecutablePath() {
  return (0, _nodePath.join)(
    rootDir,
    'out',
    'maenifold-darwin-arm64',
    'maenifold.app',
    'Contents',
    'MacOS',
    'maenifold'
  );
}

async function runCommand(command, args) {
  await new Promise((resolvePromise, reject) => {
    const child = (0, _nodeChild_process.spawn)(command, args, {
      cwd: rootDir,
      shell: process.platform === 'win32',
      stdio: 'inherit'
    });

    child.on('error', reject);
    child.on('exit', (code) => {
      if (code === 0) resolvePromise();else
      reject(new Error(`${command} ${args.join(' ')} exited with code ${code ?? 'unknown'}`));
    });
  });
}

function findRunningPackagedAppPids() {
  if (process.platform !== 'darwin') return [];

  const executablePath = resolvePackagedAppExecutablePath();
  const processList = (0, _nodeChild_process.execSync)('ps -axo pid=,command=', {
    cwd: rootDir,
    encoding: 'utf8'
  });

  return processList.
  split('\n').
  flatMap((line) => {
    const match = line.trim().match(/^(\d+)\s+(.*)$/);
    if (!match) return [];
    const pid = Number.parseInt(match[1] ?? '', 10);
    const command = match[2] ?? '';
    return Number.isFinite(pid) && command.includes(executablePath) ? [pid] : [];
  });
}

async function stopRunningPackagedApp() {
  const pids = findRunningPackagedAppPids();
  if (pids.length === 0) return;

  for (const pid of pids) {
    try {
      process.kill(pid, 'SIGTERM');
    } catch {

      // Ignore races where the process exits before we signal it.
    }}

  for (let attempt = 0; attempt < 20; attempt += 1) {
    await new Promise((resolvePromise) => setTimeout(resolvePromise, 250));
    if (findRunningPackagedAppPids().length === 0) return;
  }

  throw new Error(
    'Timed out waiting for the existing packaged maenifold.app process to exit before packaging.'
  );
}

async function markBundledBinaryExecutable(outputPaths) {
  if (process.platform === 'win32') return;

  await Promise.all(
    outputPaths.map(async (outputPath) => {
      const candidate = (0, _nodePath.join)(outputPath, 'Contents', 'Resources', 'maenifold');
      await (0, _promises.chmod)(candidate, 0o755).catch(() => undefined);
    })
  );
}

const config = {
  packagerConfig: {
    appVersion: packageJson.version,
    asar: true,
    executableName: 'maenifold',
    extraResource: [resolveMaenifoldBinaryPath()],
    icon: (0, _nodePath.join)(rootDir, 'assets', 'icons', 'icon.icns'),
    name: 'maenifold'
    // Future CI/CD work: add macOS code signing via osxSign / notarization.
  },
  hooks: {
    prePackage: async () => {
      const binaryPath = resolveMaenifoldBinaryPath();

      await stopRunningPackagedApp();
      await runCommand('npm', ['run', 'build:electron']);

      if (!(0, _nodeFs.existsSync)(binaryPath)) {
        throw new Error(`Expected maenifold binary at ${binaryPath}. Build it with cargo build --release before packaging.`);
      }

      if (process.platform !== 'win32') {
        await (0, _promises.chmod)(binaryPath, 0o755).catch(() => undefined);
      }
    },
    postPackage: async (_forgeConfig, options) => {
      await markBundledBinaryExecutable(options.outputPaths);
    }
  },
  makers: [
  {
    name: '@electron-forge/maker-zip',
    platforms: ['darwin']
  }]

};var _default = exports.default =

config; /* v9-60c717a4ff664810 */
