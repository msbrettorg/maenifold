import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

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

const allowedPath = '/home/test/maenifold/memory/note.md';

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
    resolve: mocks.handlers.get('acp:write-permission:resolve')!,
    sender: { send: vi.fn() },
  };
}

async function startPromptedWrite() {
  const { write, resolve, sender } = await loadHandlers();
  const pending = write(
    { sender },
    { path: allowedPath, content: 'hello' }
  ) as Promise<Record<string, never>>;
  await vi.waitFor(() => expect(sender.send).toHaveBeenCalledTimes(1));
  const [channel, payload] = sender.send.mock.calls[0] as [string, { requestId: string }];
  expect(channel).toBe('acp:write-permission:prompt');
  return { pending, resolve, payload };
}

describe('ACP write permission prompt IPC', () => {
  beforeEach(() => {
    vi.useRealTimers();
  });

  afterEach(async () => {
    const prompts = await import('../write-permission-prompts');
    prompts.resetWritePermissionPromptsForTest();
    vi.restoreAllMocks();
  });

  it('stores a pending request with a main-issued requestId for non-allowlisted paths', async () => {
    const { pending, resolve, payload } = await startPromptedWrite();

    expect(payload).toMatchObject({
      requestId: expect.any(String),
      path: allowedPath,
      contentPreview: 'hello',
    });

    const rejected = expect(pending).rejects.toMatchObject({ name: 'WritePermissionRejectedError' });
    await resolve({}, { requestId: payload.requestId, decision: 'reject' });
    await rejected;
  });

  it('resolve allow_once triggers the write', async () => {
    const { pending, resolve, payload } = await startPromptedWrite();

    await resolve({}, { requestId: payload.requestId, decision: 'allow_once' });

    await expect(pending).resolves.toEqual({});
    expect(mocks.writeFile).toHaveBeenCalledWith(allowedPath, 'hello', 'utf8');
  });

  it('resolve reject rejects the write with WritePermissionRejectedError', async () => {
    const { pending, resolve, payload } = await startPromptedWrite();

    const rejected = expect(pending).rejects.toMatchObject({ name: 'WritePermissionRejectedError' });
    await resolve({}, { requestId: payload.requestId, decision: 'reject' });

    await rejected;
    expect(mocks.writeFile).not.toHaveBeenCalled();
  });

  it('resolve allow_always writes and remembers the path', async () => {
    const { write, resolve, sender } = await loadHandlers();
    const pending = write(
      { sender },
      { path: allowedPath, content: 'hello' }
    ) as Promise<Record<string, never>>;
    await vi.waitFor(() => expect(sender.send).toHaveBeenCalledTimes(1));
    const payload = sender.send.mock.calls[0][1] as { requestId: string };

    await resolve({}, { requestId: payload.requestId, decision: 'allow_always' });
    await expect(pending).resolves.toEqual({});

    await expect(write({ sender }, { path: allowedPath, content: 'again' })).resolves.toEqual({});
    expect(sender.send).toHaveBeenCalledTimes(1);
    expect(mocks.writeFile).toHaveBeenCalledWith(allowedPath, 'again', 'utf8');
  });

  it('unknown requestId in resolve throws', async () => {
    const { resolve } = await loadHandlers();

    await expect(resolve({}, { requestId: 'missing', decision: 'allow_once' })).rejects.toThrow(
      'unknown write permission request'
    );
  });

  it('times out unresolved prompts after 60 seconds', async () => {
    vi.useFakeTimers();
    const { write, sender } = await loadHandlers();
    const pending = write({ sender }, { path: allowedPath, content: 'late' }) as Promise<unknown>;
    const rejected = expect(pending).rejects.toMatchObject({ name: 'WritePermissionRejectedError' });
    await vi.waitFor(() => expect(sender.send).toHaveBeenCalledTimes(1));

    await vi.advanceTimersByTimeAsync(60_000);

    await rejected;
    expect(mocks.writeFile).not.toHaveBeenCalled();
  });
});
