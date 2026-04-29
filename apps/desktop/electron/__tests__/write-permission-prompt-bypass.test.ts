import { afterEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  handlers: new Map<string, (event: unknown, payload: unknown) => unknown>(),
  realpath: vi.fn(async (filePath: string) => filePath),
  writeFile: vi.fn(async () => undefined),
}));

vi.mock('electron', () => ({
  ipcMain: {
    handle: vi.fn((channel: string, handler: (event: unknown, payload: unknown) => unknown) => {
      mocks.handlers.set(channel, handler);
    }),
  },
}));

vi.mock('node:fs/promises', () => ({
  default: {
    readFile: vi.fn(async () => ''),
    realpath: mocks.realpath,
    writeFile: mocks.writeFile,
  },
  readFile: vi.fn(async () => ''),
  realpath: mocks.realpath,
  writeFile: mocks.writeFile,
}));

vi.mock('node:os', () => ({
  default: { homedir: () => '/home/test' },
  homedir: () => '/home/test',
}));

async function loadHandlers() {
  vi.resetModules();
  mocks.handlers.clear();
  mocks.realpath.mockImplementation(async (filePath: string) => filePath);
  mocks.writeFile.mockClear();
  const prompts = await import('../write-permission-prompts');
  prompts.resetWritePermissionPromptsForTest();
  const permissions = await import('../write-permission-allowlist');
  permissions.resetWritePermissionAllowlistForTest();
  const fileSystem = await import('../acp-file-system');
  fileSystem.registerAcpFileSystemHandlers();
  return {
    write: mocks.handlers.get('acp:write-text-file')!,
    sender: { send: vi.fn() },
  };
}

describe('ACP write permission bypass regressions', () => {
  afterEach(async () => {
    const prompts = await import('../write-permission-prompts');
    prompts.resetWritePermissionPromptsForTest();
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('does not register a public IPC that lets the renderer mint write tokens', async () => {
    await loadHandlers();

    expect(mocks.handlers.has('acp:write-permission:issue-token')).toBe(false);
  });

  it('rejects a forged-token write that never receives main-owned prompt resolution', async () => {
    vi.useFakeTimers();
    const { write, sender } = await loadHandlers();

    const pending = write(
      { sender },
      {
        path: '/home/test/maenifold/memory/forged.md',
        content: 'pwn',
        token: 'forged-renderer-token',
      }
    ) as Promise<unknown>;
    const rejected = expect(pending).rejects.toMatchObject({ name: 'WritePermissionRejectedError' });
    await vi.waitFor(() => expect(sender.send).toHaveBeenCalledTimes(1));
    await vi.advanceTimersByTimeAsync(60_000);

    await rejected;
    expect(mocks.writeFile).not.toHaveBeenCalled();
  });
});
