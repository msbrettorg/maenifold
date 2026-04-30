#!/usr/bin/env node

import { execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdtempSync, rmSync } from 'node:fs';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const APP_ROOT = path.resolve(SCRIPT_DIR, '..');
const BINARY_PATH = path.join(APP_ROOT, 'out', 'maenifold-darwin-arm64', 'maenifold.app', 'Contents', 'MacOS', 'maenifold');
const PAGE_URL_RE = /^file:\/\/.+\/dist\/index\.html(?:[?#].*)?$/;
const TIMEOUT_MS = 30_000;
const POLL_MS = 1_000;
const OUTPUT_LINE_LIMIT = 200;
const interrupted = { signal: null };
let child = null;
let userDataDir = null;
process.once('SIGINT', () => { interrupted.signal = 'SIGINT'; });
process.once('SIGTERM', () => { interrupted.signal = 'SIGTERM'; });
const outputLines = [];

try {
  assert(existsSync(BINARY_PATH), `Expected packaged binary at ${BINARY_PATH}.`);
  const port = await allocatePort();
  userDataDir = mkdtempSync(path.join(os.tmpdir(), 'maenifold-electron-launch-'));
  const startedAt = Date.now();
  child = spawn(BINARY_PATH, [
    `--remote-debugging-port=${port}`,
    '--remote-allow-origins=*',
    `--user-data-dir=${userDataDir}`,
  ], {
    cwd: APP_ROOT,
    stdio: ['ignore', 'pipe', 'pipe'],
    env: {
      ...process.env,
      FORCE_COLOR: '0',
      NO_COLOR: '1',
    },
  });
  captureStream(child.stdout, 'stdout');
  captureStream(child.stderr, 'stderr');
  let reachedWindow = false;
  while (Date.now() - startedAt < TIMEOUT_MS) {
    assert(!interrupted.signal, `Interrupted by ${interrupted.signal}.`);
    const targets = await readJsonTargets(port);
    if (Array.isArray(targets) && targets.some((entry) => entry?.type === 'page' && PAGE_URL_RE.test(String(entry?.url ?? '')))) {
      reachedWindow = true;
      break;
    }
    await sleep(POLL_MS);
  }
  const elapsedSeconds = ((Date.now() - startedAt) / 1000).toFixed(1);
  const helperInfo = getHelperInfo(child.pid);
  if (reachedWindow) {
    console.log(`✅ packaged app reached createWindow within ${elapsedSeconds}s`);
    process.exitCode = 0;
  } else {
    console.error('❌ packaged app did not reach createWindow within 30s');
    console.error(`Helper processes spawned: ${helperInfo.labels.join(', ')}`);
    if (helperInfo.helpers.length > 0) {
      for (const proc of helperInfo.helpers.slice(-10)) {
        console.error(`ps: ${proc.command}`);
      }
    }
    const tail = outputLines.slice(-30);
    if (tail.length > 0) { console.error('Last 30 output lines:'); for (const line of tail) console.error(line); }
    process.exitCode = 1;
  }
} catch (error) {
  console.error(`❌ ${error instanceof Error ? error.message : String(error)}`);
  const tail = outputLines.slice(-30);
  if (tail.length > 0) { console.error('Last 30 output lines:'); for (const line of tail) console.error(line); }
  process.exitCode = 1;
} finally {
  await cleanup();
}

function captureStream(stream, name) {
  let remainder = '';
  stream.on('data', (chunk) => {
    remainder += chunk.toString();
    const lines = remainder.split(/\r?\n/);
    remainder = lines.pop() ?? '';
    for (const line of lines) pushOutput(`[${name}] ${line}`);
  });
  stream.on('end', () => {
    if (remainder.length > 0) pushOutput(`[${name}] ${remainder}`);
  });
}
function pushOutput(line) {
  outputLines.push(line);
  if (outputLines.length > OUTPUT_LINE_LIMIT) outputLines.splice(0, outputLines.length - OUTPUT_LINE_LIMIT);
}
async function readJsonTargets(port) {
  try {
    const response = await fetch(`http://127.0.0.1:${port}/json`, {
      signal: AbortSignal.timeout(4_000),
    });
    if (!response.ok) return null;
    return await response.json();
  } catch {
    return null;
  }
}
async function allocatePort() {
  return await new Promise((resolve, reject) => {
    const server = net.createServer();
    server.once('error', reject);
    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      assert(address && typeof address === 'object', 'Failed to allocate debug port.');
      server.close((error) => error ? reject(error) : resolve(address.port));
    });
  });
}
function getHelperInfo(rootPid) {
  const descendants = listDescendants(rootPid);
  const helpers = descendants.filter((proc) => proc.command.includes('--type='));
  const labels = [];
  if (helpers.some((proc) => /network/i.test(proc.command))) labels.push('network');
  if (helpers.some((proc) => /--type=gpu-process\b/.test(proc.command))) labels.push('GPU');
  if (helpers.some((proc) => /--type=renderer\b/.test(proc.command))) labels.push('renderer');
  if (labels.length === 0) labels.push('none');
  return { labels, helpers };
}
function listDescendants(rootPid) {
  const snapshot = listProcesses();
  const byParent = new Map();
  for (const proc of snapshot) {
    const siblings = byParent.get(proc.ppid) ?? [];
    siblings.push(proc);
    byParent.set(proc.ppid, siblings);
  }

  const descendants = [];
  const stack = [rootPid];
  while (stack.length > 0) {
    const current = stack.pop();
    for (const childProc of byParent.get(current) ?? []) {
      descendants.push(childProc);
      stack.push(childProc.pid);
    }
  }
  return descendants;
}
function listProcesses() {
  try {
    return execFileSync('ps', ['-axo', 'pid=,ppid=,command='], { encoding: 'utf8' })
      .split('\n')
      .map((line) => line.match(/^\s*(\d+)\s+(\d+)\s+(.*)$/))
      .filter(Boolean)
      .map((match) => ({ pid: Number(match[1]), ppid: Number(match[2]), command: match[3] }));
  } catch {
    return [];
  }
}
async function cleanup() {
  if (child?.pid) await terminateProcessTree(child.pid);
  if (userDataDir) rmSync(userDataDir, { recursive: true, force: true });
}
async function terminateProcessTree(rootPid) {
  const descendants = listDescendants(rootPid).map((proc) => proc.pid).reverse();
  for (const pid of [...descendants, rootPid]) safeKill(pid, 'SIGTERM');
  await sleep(500);
  for (const pid of [...descendants, rootPid]) safeKill(pid, 'SIGKILL');
}
function safeKill(pid, signal) {
  try {
    process.kill(pid, signal);
  } catch {
  }
}
function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
function assert(condition, message) {
  if (!condition) throw new Error(message);
}
