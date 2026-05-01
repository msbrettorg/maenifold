#!/usr/bin/env node
import assert from 'node:assert/strict';
import { lstatSync, mkdirSync, mkdtempSync, realpathSync, rmSync, symlinkSync } from 'node:fs';
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

rmSync(root, { recursive: true, force: true });
console.log('gate-write-task IPC weakness repro passed');
