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
