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

describe('audit-components max-loc enforcement', () => {
  afterEach(() => {
    vi.restoreAllMocks();
    vi.resetModules();
    mocks.spawnSync.mockReset();
    mocks.readFileSync.mockReset();
    mocks.readdirSync.mockReset();
  });

  it('keeps countLinesForMaxLoc wc -l semantics', async () => {
    mocks.spawnSync.mockReturnValue({ status: 0, stdout: '' });
    mocks.readdirSync.mockReturnValue([]);
    const log = vi.spyOn(console, 'log').mockImplementation(() => undefined);
    const error = vi.spyOn(console, 'error').mockImplementation(() => undefined);

    const { countLinesForMaxLoc } = await import('../audit-loc.mjs');

    expect(countLinesForMaxLoc('a\nb\nc\n')).toBe(3);
    expect(countLinesForMaxLoc('a\nb\nc')).toBe(2);

    log.mockRestore();
    error.mockRestore();
  });

  it('emits a max-loc violation for a 301-line ts source', async () => {
    mocks.spawnSync.mockReturnValue({ status: 0, stdout: '' });
    mocks.readdirSync.mockReturnValue([]);
    const log = vi.spyOn(console, 'log').mockImplementation(() => undefined);
    const error = vi.spyOn(console, 'error').mockImplementation(() => undefined);

    const { checkMaxLoc } = await import('../audit-components.mjs?max-loc-over');
    const source = 'line\n'.repeat(301);

    expect(checkMaxLoc(source, 'src/example.ts', false)).toEqual({
      file: 'src/example.ts',
      line: 301,
      rule: 'max-loc',
      message: '301 lines exceeds 300 limit — split this file.',
    });

    log.mockRestore();
    error.mockRestore();
  });

  it('does not emit a max-loc violation for a 299-line ts source', async () => {
    mocks.spawnSync.mockReturnValue({ status: 0, stdout: '' });
    mocks.readdirSync.mockReturnValue([]);
    const log = vi.spyOn(console, 'log').mockImplementation(() => undefined);
    const error = vi.spyOn(console, 'error').mockImplementation(() => undefined);

    const { checkMaxLoc } = await import('../audit-components.mjs?max-loc-under');
    const source = 'line\n'.repeat(299);

    expect(checkMaxLoc(source, 'src/example.ts', false)).toBeNull();

    log.mockRestore();
    error.mockRestore();
  });

  it('does not emit a max-loc violation at the exact 300-line boundary', async () => {
    mocks.spawnSync.mockReturnValue({ status: 0, stdout: '' });
    mocks.readdirSync.mockReturnValue([]);
    const log = vi.spyOn(console, 'log').mockImplementation(() => undefined);
    const error = vi.spyOn(console, 'error').mockImplementation(() => undefined);

    const { checkMaxLoc } = await import('../audit-components.mjs?max-loc-boundary');
    const source = 'line\n'.repeat(300);

    expect(checkMaxLoc(source, 'src/example.ts', false)).toBeNull();

    log.mockRestore();
    error.mockRestore();
  });

  it('counts CRLF line endings the same as LF', async () => {
    mocks.spawnSync.mockReturnValue({ status: 0, stdout: '' });
    mocks.readdirSync.mockReturnValue([]);
    const log = vi.spyOn(console, 'log').mockImplementation(() => undefined);
    const error = vi.spyOn(console, 'error').mockImplementation(() => undefined);

    const { checkMaxLoc } = await import('../audit-components.mjs?max-loc-crlf');
    const sourceCrlf = 'line\r\n'.repeat(301);
    const sourceLf = 'line\n'.repeat(301);

    const crlfResult = checkMaxLoc(sourceCrlf, 'src/example.ts', false);
    const lfResult = checkMaxLoc(sourceLf, 'src/example.ts', false);

    expect(crlfResult).not.toBeNull();
    expect(lfResult).not.toBeNull();
    expect(crlfResult.line).toBe(lfResult.line);
    expect(crlfResult.message).toBe(lfResult.message);

    log.mockRestore();
    error.mockRestore();
  });

  it('uses wc -l newline-count semantics via checkMaxLoc (trailing newline counts; missing trailing newline does not add a line)', async () => {
    mocks.spawnSync.mockReturnValue({ status: 0, stdout: '' });
    mocks.readdirSync.mockReturnValue([]);
    const log = vi.spyOn(console, 'log').mockImplementation(() => undefined);
    const error = vi.spyOn(console, 'error').mockImplementation(() => undefined);

    const { checkMaxLoc } = await import('../audit-components.mjs?max-loc-trailing');
    const withTrailing = 'line\n'.repeat(301);
    const withoutTrailing = 'line\n'.repeat(300) + 'line';

    expect(checkMaxLoc(withTrailing, 'src/example.ts', false)).not.toBeNull();
    expect(checkMaxLoc(withoutTrailing, 'src/example.ts', false)).toBeNull();

    log.mockRestore();
    error.mockRestore();
  });
});

describe('audit-components integration: auditCodeFile end-to-end', () => {
  afterEach(() => {
    vi.restoreAllMocks();
    vi.resetModules();
    mocks.spawnSync.mockReset();
    mocks.readFileSync.mockReset();
    mocks.readdirSync.mockReset();
  });

  async function runAudit({ relPath, source, loadAudit }) {
    mocks.spawnSync.mockReturnValue({ status: 0, stdout: `${relPath}\n` });
    mocks.readdirSync.mockReturnValue([]);
    mocks.readFileSync.mockImplementation((filePath) => {
      if (String(filePath).endsWith(relPath)) return source;
      throw new Error(`Unexpected read: ${filePath}`);
    });

    const log = vi.spyOn(console, 'log').mockImplementation(() => undefined);
    const error = vi.spyOn(console, 'error').mockImplementation(() => undefined);
    const exit = vi.spyOn(process, 'exit').mockImplementation((code) => {
      throw new Error(`process.exit:${code}`);
    });

    let importError = null;
    try {
      await loadAudit();
    } catch (err) {
      importError = err;
    }

    return { log, error, exit, importError };
  }

  it('emits exactly one violation when a synthetic .ts file is 301 lines', async () => {
    const result = await runAudit({
      relPath: 'src/example.ts',
      source: 'line\n'.repeat(301),
      loadAudit: () => import('../audit-components.mjs?audit-code-file-over'),
    });

    expect(result.importError).toBeInstanceOf(Error);
    expect(result.importError?.message).toBe('process.exit:1');
    expect(result.exit).toHaveBeenCalledWith(1);
    expect(result.error.mock.calls).toEqual([
      ['\n❌ Component audit: 1 violation(s) found:\n'],
      ['  max-loc (1):'],
      ['    src/example.ts:301  301 lines exceeds 300 limit — split this file.'],
      [''],
      ['Run with --all to scan every component.\n'],
    ]);
    expect(result.log).not.toHaveBeenCalled();
  });

  it('does NOT emit a violation when a synthetic .ts file is 300 lines', async () => {
    const result = await runAudit({
      relPath: 'src/example.ts',
      source: 'line\n'.repeat(300),
      loadAudit: () => import('../audit-components.mjs?audit-code-file-boundary'),
    });

    expect(result.importError).toBeNull();
    expect(result.exit).not.toHaveBeenCalled();
    expect(result.error).not.toHaveBeenCalled();
    expect(result.log).toHaveBeenCalledWith('✅ audit:components passed (0 changed components, 1 changed TS files).');
  });

  it('skips .vue files in the LOC check (their LOC is governed by the existing .vue rule)', async () => {
    const result = await runAudit({
      relPath: 'src/Example.vue',
      source: `<template>\n  <div />\n</template>\n<script setup lang="ts">\n${'const value = 1;\n'.repeat(496)}</script>\n`,
      loadAudit: () => import('../audit-components.mjs?audit-code-file-vue'),
    });

    expect(result.importError).toBeInstanceOf(Error);
    expect(result.importError?.message).toBe('process.exit:1');
    expect(result.exit).toHaveBeenCalledWith(1);
    expect(result.error).toHaveBeenCalledWith('  max-loc (1):');
    expect(result.error).toHaveBeenCalledWith(
      '    src/Example.vue:501  501 lines exceeds 300 limit — split this component.'
    );
    expect(result.error).not.toHaveBeenCalledWith(
      '    src/Example.vue:501  501 lines exceeds 300 limit — split this file.'
    );
  });
});
