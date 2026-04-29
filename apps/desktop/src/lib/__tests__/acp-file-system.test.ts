import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { WriteTextFileRequest } from '@agentclientprotocol/sdk';
import type { WritePermissionPolicy } from '../acp-file-system';

function installElectronApi(
  writeTextFile = vi.fn().mockResolvedValue({}),
  options: {
    issueWritePermissionToken?: (params: { path: string; allowsAlways?: boolean }) => Promise<string>;
    isWritePathAlwaysAllowed?: (path: string) => Promise<boolean>;
  } = {}
) {
  const issueWritePermissionToken = vi.fn(
    options.issueWritePermissionToken ?? (async () => 'token-1')
  );
  const isWritePathAlwaysAllowed = vi.fn(
    options.isWritePathAlwaysAllowed ?? (async () => false)
  );
  Object.defineProperty(window, 'electronAPI', {
    configurable: true,
    value: {
      readTextFile: vi.fn(),
      writeTextFile,
      issueWritePermissionToken,
      isWritePathAlwaysAllowed,
    },
  });
  return { writeTextFile, issueWritePermissionToken, isWritePathAlwaysAllowed };
}

function createPolicy(
  overrides: Partial<WritePermissionPolicy> = {}
): WritePermissionPolicy & {
  requestPermission: ReturnType<typeof vi.fn>;
  notifyPlanModeBlocked: ReturnType<typeof vi.fn>;
} {
  const pathAllowList = new Set<string>();
  return {
    currentModeId: () => null,
    requestPermission: vi.fn().mockResolvedValue('allow'),
    pathAllowList,
    notifyPlanModeBlocked: vi.fn(),
    ...overrides,
  } as WritePermissionPolicy & {
    requestPermission: ReturnType<typeof vi.fn>;
    notifyPlanModeBlocked: ReturnType<typeof vi.fn>;
  };
}

function writeRequest(path: string, content: string): WriteTextFileRequest {
  return { sessionId: 'session-1', path, content } as WriteTextFileRequest;
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
    const fileSystem = await import('../acp-file-system');
    const policy = createPolicy({
      requestPermission: vi.fn().mockResolvedValueOnce('reject'),
    });

    await expect(
      fileSystem.handleWriteTextFileRequest(writeRequest('/tmp/foo', 'x'), policy)
    ).rejects.toMatchObject({
      name: 'WritePermissionRejectedError',
    });

    expect(policy.requestPermission).toHaveBeenCalledWith({
      path: '/tmp/foo',
      content: 'x',
    });
    expect(writeTextFile).not.toHaveBeenCalled();
  });

  it('proceeds with the write after an explicit allow decision', async () => {
    const { writeTextFile, issueWritePermissionToken } = installElectronApi();
    const fileSystem = await import('../acp-file-system');
    const policy = createPolicy();

    await expect(
      fileSystem.handleWriteTextFileRequest(writeRequest('/tmp/foo', 'x'), policy)
    ).resolves.toEqual({});

    expect(issueWritePermissionToken).toHaveBeenCalledWith({ path: '/tmp/foo', allowsAlways: false });
    expect(writeTextFile).toHaveBeenCalledWith({ path: '/tmp/foo', content: 'x', token: 'token-1' });
  });

  it('throws a clear rejection error when the user denies the write', async () => {
    const { writeTextFile } = installElectronApi();
    const fileSystem = await import('../acp-file-system');
    const policy = createPolicy({
      requestPermission: vi.fn().mockResolvedValueOnce('reject'),
    });

    await expect(
      fileSystem.handleWriteTextFileRequest(writeRequest('/tmp/foo', 'denied'), policy)
    ).rejects.toMatchObject({
      name: 'WritePermissionRejectedError',
      message: expect.stringContaining('/tmp/foo'),
    });

    expect(writeTextFile).not.toHaveBeenCalled();
  });

  it('persists always-allow-this-path so the second write skips the dialog', async () => {
    const { writeTextFile, issueWritePermissionToken, isWritePathAlwaysAllowed } = installElectronApi();
    const fileSystem = await import('../acp-file-system');
    const policy = createPolicy({
      requestPermission: vi.fn().mockResolvedValueOnce('always-allow-path'),
    });

    await fileSystem.handleWriteTextFileRequest(writeRequest('/tmp/foo', 'first'), policy);
    await fileSystem.handleWriteTextFileRequest(writeRequest('/tmp/foo', 'second'), policy);

    expect(policy.requestPermission).toHaveBeenCalledTimes(1);
    expect(policy.pathAllowList.has('/tmp/foo')).toBe(true);
    expect(isWritePathAlwaysAllowed).toHaveBeenCalledTimes(1);
    expect(issueWritePermissionToken).toHaveBeenCalledWith({ path: '/tmp/foo', allowsAlways: true });
    expect(writeTextFile).toHaveBeenCalledTimes(2);
  });

  it('auto-rejects plan mode writes without prompting the gate', async () => {
    const { writeTextFile } = installElectronApi();
    const fileSystem = await import('../acp-file-system');
    const policy = createPolicy({ currentModeId: () => 'plan' });

    await expect(
      fileSystem.handleWriteTextFileRequest(writeRequest('/tmp/foo', 'x'), policy)
    ).rejects.toMatchObject({
      name: 'WritePermissionRejectedError',
      message: expect.stringContaining('plan mode'),
    });

    expect(policy.requestPermission).not.toHaveBeenCalled();
    expect(policy.notifyPlanModeBlocked).toHaveBeenCalledWith('/tmp/foo');
    expect(writeTextFile).not.toHaveBeenCalled();
  });

  it('keeps mode and allow-list policy isolated across sessions', async () => {
    const { writeTextFile } = installElectronApi();
    const fileSystem = await import('../acp-file-system');
    const planPolicy = createPolicy({ currentModeId: () => 'plan' });
    const normalPolicy = createPolicy({ currentModeId: () => 'architect' });

    await expect(
      fileSystem.handleWriteTextFileRequest(writeRequest('/tmp/plan', 'x'), planPolicy)
    ).rejects.toMatchObject({ name: 'WritePermissionRejectedError' });
    await expect(
      fileSystem.handleWriteTextFileRequest(writeRequest('/tmp/normal', 'x'), normalPolicy)
    ).resolves.toEqual({});

    expect(planPolicy.requestPermission).not.toHaveBeenCalled();
    expect(normalPolicy.requestPermission).toHaveBeenCalledTimes(1);
    expect(writeTextFile).toHaveBeenCalledTimes(1);
    expect(writeTextFile).toHaveBeenCalledWith({
      path: '/tmp/normal',
      content: 'x',
      token: 'token-1',
    });
  });

  it('uses a minted single-use token for the IPC write', async () => {
    const usedTokens = new Set<string>();
    const writeTextFile = vi.fn(async ({ token }: { token?: string }) => {
      if (!token || usedTokens.has(token)) throw new Error('invalid token');
      usedTokens.add(token);
      return {};
    });
    installElectronApi(writeTextFile);
    const fileSystem = await import('../acp-file-system');
    const policy = createPolicy();

    await expect(
      fileSystem.handleWriteTextFileRequest(writeRequest('/tmp/foo', 'x'), policy)
    ).resolves.toEqual({});
    await expect(writeTextFile.mock.calls[0][0] && writeTextFile(writeTextFile.mock.calls[0][0]))
      .rejects.toThrow('invalid token');
  });
});
