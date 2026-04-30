#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync, rmSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const APP_ROOT = path.resolve(SCRIPT_DIR, '..');
const DIST_ELECTRON_DIR = path.join(APP_ROOT, 'dist-electron');
const MAIN_BUNDLE_PATH = path.join(DIST_ELECTRON_DIR, 'main.js');

const EXTERNALIZED_NODE_PTY_PATTERNS = [
  /\brequire\(["']node-pty["']\)/,
  /\b__require\(["']node-pty["']\)/,
  /\bfrom\s+["']node-pty["']/,
  /\bimport\(["']node-pty["']\)/,
];

const BUNDLED_NATIVE_PATTERNS = [
  /build\/Release\/pty\.node/,
  /build\/Debug\/pty\.node/,
  /conpty\.node/,
  /winpty\.node/,
  /pty\.node/,
];

rmSync(DIST_ELECTRON_DIR, { recursive: true, force: true });

const build = spawnSync('npm', ['run', 'build:electron'], {
  cwd: APP_ROOT,
  encoding: 'utf8',
  env: {
    ...process.env,
    FORCE_COLOR: '0',
    NO_COLOR: '1',
  },
  maxBuffer: 20 * 1024 * 1024,
});

const output = `${build.stdout ?? ''}${build.stderr ?? ''}`.trim();

if (output.length > 0) {
  console.log(output);
}

assert(
  build.status === 0,
  `Expected "npm run build:electron" to exit 0, got ${build.status ?? 'unknown'}.`,
);

assert(existsSync(MAIN_BUNDLE_PATH), `Expected electron main bundle at ${MAIN_BUNDLE_PATH}.`);

const mainBundle = readFileSync(MAIN_BUNDLE_PATH, 'utf8');

assert(
  EXTERNALIZED_NODE_PTY_PATTERNS.some((pattern) => pattern.test(mainBundle)),
  'Expected dist-electron/main.js to preserve an external node-pty import/require.',
);

assert(
  !BUNDLED_NATIVE_PATTERNS.some((pattern) => pattern.test(mainBundle)),
  'Expected dist-electron/main.js to avoid bundling node-pty native binding paths.',
);

console.log('✅ electron build audit passed.');

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}
