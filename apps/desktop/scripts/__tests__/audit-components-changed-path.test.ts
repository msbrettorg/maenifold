import { afterEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  spawnSync: vi.fn(),
  readFileSync: vi.fn(),
  readdirSync: vi.fn(),
}));

vi.mock('node:child_process', () => ({
  default: { spawnSync: mocks.spawnSync },
  spawnSync: mocks.spawnSync,
}));
vi.mock('node:fs', () => ({
  default: {
    readFileSync: mocks.readFileSync,
    readdirSync: mocks.readdirSync,
  },
  readFileSync: mocks.readFileSync,
  readdirSync: mocks.readdirSync,
}));

describe('audit-components changed-mode path handling', () => {
  afterEach(() => {
    vi.restoreAllMocks();
    vi.resetModules();
    mocks.spawnSync.mockReset();
    mocks.readFileSync.mockReset();
    mocks.readdirSync.mockReset();
  });

  it('audits git paths that include the apps/desktop prefix', async () => {
    const appRoot = process.cwd();
    const changedFile = 'src/components/ChangedTooLarge.vue';
    const correctAbsolutePath = `${appRoot}/${changedFile}`;
    const oversizedComponent = `${'<template>\n<div />\n</template>\n'}${'const x = 1;\n'.repeat(299)}`;
    const errors: string[] = [];
    const exit = vi.spyOn(process, 'exit').mockImplementation(((code?: string | number | null) => {
      throw new Error(`process.exit(${code})`);
    }) as typeof process.exit);
    vi.spyOn(console, 'error').mockImplementation((message?: unknown) => errors.push(String(message)));
    vi.spyOn(console, 'log').mockImplementation(() => undefined);

    mocks.spawnSync.mockReturnValue({ status: 0, stdout: `apps/desktop/${changedFile}\n` });
    mocks.readdirSync.mockReturnValue([]);
    mocks.readFileSync.mockImplementation((filePath: unknown) => {
      if (String(filePath) === correctAbsolutePath) return oversizedComponent;
      throw new Error(`unexpected read: ${String(filePath)}`);
    });

    await expect(import('../audit-components.mjs?changed-path')).rejects.toThrow('process.exit(1)');

    expect(mocks.spawnSync).toHaveBeenCalledWith(
      'git',
      expect.arrayContaining(['diff', '--name-only', '--relative', 'HEAD~1...HEAD']),
      expect.objectContaining({ cwd: appRoot })
    );
    expect(exit).toHaveBeenCalledWith(1);
    expect(errors.join('\n')).toContain(`${changedFile}:302`);
  });
});
