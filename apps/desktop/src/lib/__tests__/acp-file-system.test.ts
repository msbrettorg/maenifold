import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { WriteTextFileRequest } from '@agentclientprotocol/sdk';
import type { WritePermissionPolicy } from '../acp-file-system';

function installElectronApi(
  writeTextFile = vi.fn().mockResolvedValue({}),
) {
  Object.defineProperty(window, 'electronAPI', {
    configurable: true,
    value: {
      readTextFile: vi.fn(),
      writeTextFile,
    },
  });
  return { writeTextFile };
}

function createPolicy(
  overrides: Partial<WritePermissionPolicy> = {}
): WritePermissionPolicy & {
  notifyPlanModeBlocked: ReturnType<typeof vi.fn>;
} {
  return {
    currentModeId: () => null,
    notifyPlanModeBlocked: vi.fn(),
    ...overrides,
  } as WritePermissionPolicy & {
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

  it('delegates non-plan writes to main-owned IPC permission flow', async () => {
    const { writeTextFile } = installElectronApi();
    const fileSystem = await import('../acp-file-system');
    const policy = createPolicy();

    await expect(
      fileSystem.handleWriteTextFileRequest(writeRequest('/tmp/foo', 'x'), policy)
    ).resolves.toEqual({});

    expect(writeTextFile).toHaveBeenCalledWith({
      path: '/tmp/foo',
      content: 'x',
      sessionId: 'session-1',
    });
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

    expect(writeTextFile).toHaveBeenCalledTimes(1);
    expect(writeTextFile).toHaveBeenCalledWith({
      path: '/tmp/normal',
      content: 'x',
      sessionId: 'session-1',
    });
  });
});
