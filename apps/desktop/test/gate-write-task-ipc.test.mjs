#!/usr/bin/env node
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

const canonicalTaskFilenamePattern = /^T-\d+\.\d+-[a-z0-9]+(?:-[a-z0-9]+)*\.md$/;
const canonicalTaskFilenameSource =
  'const gateTaskFilenamePattern = /^T-\\d+\\.\\d+-[a-z0-9]+(?:-[a-z0-9]+)*\\.md$/;';

function readMainSource() {
  return readFileSync(join(process.cwd(), 'electron/main.ts'), 'utf8');
}

function readGateRegistrySource() {
  return readFileSync(join(process.cwd(), 'src/composables/useGateRegistry.ts'), 'utf8');
}

function gateWriteTaskHandler(source) {
  const handlerMatch = source.match(/ipcMain\.handle\('gate:write-task-file'[\s\S]*?return \{ path: taskPath \};\n  }\);/);
  expect(handlerMatch).not.toBeNull();
  return handlerMatch[0];
}

describe('gate:write-task-file IPC contract', () => {
  it('uses the canonical tightened task filename schema', () => {
    for (const name of ['T-1.1--.md', 'T-1.1-a-.md', 'T-1.1--foo.md']) {
      expect(canonicalTaskFilenamePattern.test(name), `${name} should be rejected`).toBe(false);
    }

    for (const name of ['T-1.1-a.md', 'T-1.1-a-b2.md']) {
      expect(canonicalTaskFilenamePattern.test(name), `${name} should be accepted`).toBe(true);
    }

    expect(readMainSource()).toContain(canonicalTaskFilenameSource);
  });

  it('requires the renderer-selected Gate root before writing', () => {
    const mainSource = readMainSource();
    const gateRegistrySource = readGateRegistrySource();
    const handlerSource = gateWriteTaskHandler(mainSource);

    expect(gateRegistrySource).toMatch(/selectedGateRoot/);
    expect(mainSource).toMatch(/let selectedGateRoot: string \| null = null;/);
    expect(mainSource).toMatch(/ipcMain\.handle\('gate:set-root'/);
    expect(gateRegistrySource).toMatch(/gateSetRoot/);
    expect(handlerSource).toMatch(/realpathSync\(root\)/);
    expect(handlerSource).toMatch(/realpathSync\(selectedGateRoot\)/);
    expect(handlerSource).toMatch(/Root mismatch: provided root does not match selected gate root/);
  });

  it('keeps path validation and exclusive-create write checks in place', () => {
    const handlerSource = gateWriteTaskHandler(readMainSource());

    expect(handlerSource).toMatch(/lstatSync\(todoDir\)\.isSymbolicLink\(\)/);
    expect(handlerSource).toMatch(/Refusing to write: todo\/ is a symbolic link/);
    expect(handlerSource).toMatch(/realpathSync\(todoDir\)/);
    expect(handlerSource).toContain("if (!resolvedTodo.startsWith(`${resolvedRoot}/`)) throw new Error('Refusing to write: todo/ resolves outside root');");
    expect(handlerSource).toMatch(/Refusing to write: todo\/ resolves outside root/);
    expect(handlerSource).toContain("writeFileSync(taskPath, content, { flag: 'wx' });");
  });
});
