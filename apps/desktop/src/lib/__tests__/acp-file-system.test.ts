import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

function installElectronApi(writeTextFile = vi.fn().mockResolvedValue({})) {
  Object.defineProperty(window, 'electronAPI', {
    configurable: true,
    value: {
      readTextFile: vi.fn(),
      writeTextFile,
    },
  });
  return { writeTextFile };
}

describe('handleWriteTextFileRequest', () => {
  beforeEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('routes fs/write through a permission gate instead of writing directly', async () => {
    const { writeTextFile } = installElectronApi();
    const fileSystem = await import('../acp-file-system') as Record<string, any>;

    expect(fileSystem).toHaveProperty('requestWritePermission');
    const requestWritePermission = vi
      .spyOn(fileSystem, 'requestWritePermission')
      .mockResolvedValueOnce('reject');

    await expect(
      fileSystem.handleWriteTextFileRequest({ path: '/tmp/foo', content: 'x' })
    ).rejects.toMatchObject({
      name: 'WritePermissionRejectedError',
    });

    expect(requestWritePermission).toHaveBeenCalledWith({
      path: '/tmp/foo',
      content: 'x',
    });
    expect(writeTextFile).not.toHaveBeenCalled();
  });

  it('proceeds with the write after an explicit allow decision', async () => {
    const { writeTextFile } = installElectronApi();
    const fileSystem = await import('../acp-file-system') as Record<string, any>;

    expect(fileSystem).toHaveProperty('requestWritePermission');
    vi.spyOn(fileSystem, 'requestWritePermission').mockResolvedValueOnce('allow');

    await expect(
      fileSystem.handleWriteTextFileRequest({ path: '/tmp/foo', content: 'x' })
    ).resolves.toEqual({});

    expect(writeTextFile).toHaveBeenCalledWith({ path: '/tmp/foo', content: 'x' });
  });

  it('throws a clear rejection error when the user denies the write', async () => {
    const { writeTextFile } = installElectronApi();
    const fileSystem = await import('../acp-file-system') as Record<string, any>;

    expect(fileSystem).toHaveProperty('requestWritePermission');
    vi.spyOn(fileSystem, 'requestWritePermission').mockResolvedValueOnce('reject');

    await expect(
      fileSystem.handleWriteTextFileRequest({ path: '/tmp/foo', content: 'denied' })
    ).rejects.toMatchObject({
      name: 'WritePermissionRejectedError',
      message: expect.stringContaining('/tmp/foo'),
    });

    expect(writeTextFile).not.toHaveBeenCalled();
  });

  it('persists always-allow-this-path so the second write skips the dialog', async () => {
    const { writeTextFile } = installElectronApi();
    const fileSystem = await import('../acp-file-system') as Record<string, any>;

    expect(fileSystem).toHaveProperty('requestWritePermission');
    expect(fileSystem).toHaveProperty('isWritePathAlwaysAllowed');
    expect(fileSystem).toHaveProperty('rememberWritePathAlwaysAllowed');

    const rememberedPaths = new Set<string>();
    const isWritePathAlwaysAllowed = vi
      .spyOn(fileSystem, 'isWritePathAlwaysAllowed')
      .mockImplementation((path: string) => rememberedPaths.has(path));
    const rememberWritePathAlwaysAllowed = vi
      .spyOn(fileSystem, 'rememberWritePathAlwaysAllowed')
      .mockImplementation((path: string) => {
        rememberedPaths.add(path);
      });
    const requestWritePermission = vi
      .spyOn(fileSystem, 'requestWritePermission')
      .mockResolvedValueOnce('always-allow-path');

    await fileSystem.handleWriteTextFileRequest({ path: '/tmp/foo', content: 'first' });
    await fileSystem.handleWriteTextFileRequest({ path: '/tmp/foo', content: 'second' });

    expect(requestWritePermission).toHaveBeenCalledTimes(1);
    expect(rememberWritePathAlwaysAllowed).toHaveBeenCalledWith('/tmp/foo');
    expect(isWritePathAlwaysAllowed).toHaveBeenNthCalledWith(2, '/tmp/foo');
    expect(writeTextFile).toHaveBeenCalledTimes(2);
  });

  it('auto-rejects plan mode writes without prompting the gate', async () => {
    const { writeTextFile } = installElectronApi();
    const fileSystem = await import('../acp-file-system') as Record<string, any>;

    expect(fileSystem).toHaveProperty('currentModeId');
    expect(fileSystem).toHaveProperty('requestWritePermission');
    vi.spyOn(fileSystem, 'currentModeId').mockReturnValue('plan');
    const requestWritePermission = vi
      .spyOn(fileSystem, 'requestWritePermission')
      .mockResolvedValueOnce('allow');

    await expect(
      fileSystem.handleWriteTextFileRequest({ path: '/tmp/foo', content: 'x' })
    ).rejects.toMatchObject({
      name: 'WritePermissionRejectedError',
      message: expect.stringContaining('plan mode'),
    });

    expect(requestWritePermission).not.toHaveBeenCalled();
    expect(writeTextFile).not.toHaveBeenCalled();
  });
});
