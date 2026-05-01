#!/usr/bin/env node
import assert from 'node:assert/strict';
import { lstatSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, symlinkSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const CURRENT_REGEX = /^T-\d+\.\d+-[a-z0-9-]+\.md$/;
const TIGHTENED_REGEX = /^T-\d+\.\d+-[a-z0-9]+(?:-[a-z0-9]+)*\.md$/;
const root = mkdtempSync(join(tmpdir(), 'gate-write-task-'));
const target = join(root, 'escaped');
const todo = join(root, 'todo');

mkdirSync(target);
symlinkSync(target, todo, 'dir');
assert.equal(join(todo, 'T-1.1-proof.md').startsWith(`${todo}/`), true, 'lexical path check stays inside todo/');
assert.equal(lstatSync(todo).isSymbolicLink(), true, 'todo/ can be a symlink today');
assert.equal(realpathSync(todo), realpathSync(target), 'symlinked todo/ resolves somewhere else');

for (const name of ['T-1.1--.md', 'T-1.1-a-.md', 'T-1.1--foo.md']) {
  assert.equal(CURRENT_REGEX.test(name), true, `current regex should allow ${name}`);
  assert.equal(TIGHTENED_REGEX.test(name), false, `tightened regex should reject ${name}`);
}

for (const name of ['T-1.1-a.md', 'T-1.1-a-b2.md']) {
  assert.equal(TIGHTENED_REGEX.test(name), true, `tightened regex should allow ${name}`);
}

const mainSource = readFileSync(new URL('../electron/main.ts', import.meta.url), 'utf8');
const gateRegistrySource = readFileSync(new URL('../src/composables/useGateRegistry.ts', import.meta.url), 'utf8');
const handlerMatch = mainSource.match(/ipcMain\.handle\('gate:write-task-file'[\s\S]*?return \{ path: taskPath \};\n  }\);/);
assert.ok(handlerMatch, 'gate:write-task-file handler should exist');
assert.match(gateRegistrySource, /selectedGateRoot/, 'renderer should track a selected Gate root');
assert.match(mainSource, /let selectedGateRoot: string \| null = null;/, 'main process should track selected Gate root');
assert.match(mainSource, /ipcMain\.handle\('gate:set-root'/, 'renderer should be able to set selected Gate root in main');
assert.match(gateRegistrySource, /gateSetRoot/, 'renderer registry should sync selected Gate root to main');
assert.match(handlerMatch[0], /realpathSync\(root\)/, 'handler should canonicalize the caller-provided root');
assert.match(handlerMatch[0], /realpathSync\(selectedGateRoot\)/, 'handler should canonicalize selected Gate root');
assert.match(
  handlerMatch[0],
  /Root mismatch: provided root does not match selected gate root/,
  'handler should reject roots that do not match selected Gate root',
);

rmSync(root, { recursive: true, force: true });
console.log('gate-write-task IPC root validation passed');
