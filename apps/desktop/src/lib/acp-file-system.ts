import type {
  ReadTextFileRequest,
  ReadTextFileResponse,
  WriteTextFileRequest,
  WriteTextFileResponse,
} from '@agentclientprotocol/sdk';
import { assertAbsolutePath } from './acp-meta';

type ElectronReadFileSystemApi = {
  readTextFile: (params: { path: string; line?: number; limit?: number }) => Promise<{ content: string }>;
};

type ElectronWriteFileSystemApi = {
  writeTextFile: (params: { path: string; content: string; sessionId?: string }) => Promise<Record<string, never>>;
};

export type WritePermissionPolicy = {
  currentModeId: () => string | null;
  notifyPlanModeBlocked: (path: string) => void;
};

export class WritePermissionRejectedError extends Error {
  constructor(public readonly path: string, reason?: string) {
    super(reason ? `Write to ${path} rejected: ${reason}` : `Write to ${path} rejected`);
    this.name = 'WritePermissionRejectedError';
  }
}

function getElectronReadFileSystemApi(): ElectronReadFileSystemApi {
  const api = typeof window === 'undefined' ? undefined : window.electronAPI;
  if (!api || typeof api.readTextFile !== 'function') {
    throw new Error('Electron ACP file access is unavailable in this environment');
  }
  return {
    readTextFile: api.readTextFile,
  };
}

function getElectronWriteFileSystemApi(): ElectronWriteFileSystemApi {
  const api = typeof window === 'undefined' ? undefined : window.electronAPI;
  if (
    !api ||
    typeof api.writeTextFile !== 'function'
  ) {
    throw new Error('Electron ACP file access is unavailable in this environment');
  }
  return {
    writeTextFile: api.writeTextFile,
  };
}

export async function handleWriteTextFileRequest(
  params: WriteTextFileRequest,
  policy: WritePermissionPolicy,
): Promise<WriteTextFileResponse> {
  assertAbsolutePath(params.path);
  if (policy.currentModeId() === 'plan') {
    policy.notifyPlanModeBlocked(params.path);
    throw new WritePermissionRejectedError(params.path, 'plan mode');
  }
  const electronFileSystem = getElectronWriteFileSystemApi();
  await electronFileSystem.writeTextFile({
    path: params.path,
    content: params.content,
    sessionId: params.sessionId,
  });
  return {};
}

export async function handleReadTextFileRequest(
  params: ReadTextFileRequest,
): Promise<ReadTextFileResponse> {
  assertAbsolutePath(params.path);
  const request: { path: string; line?: number; limit?: number } = { path: params.path };
  if (typeof params.line === 'number') request.line = params.line;
  if (typeof params.limit === 'number') request.limit = params.limit;
  return getElectronReadFileSystemApi().readTextFile({
    ...request,
  });
}
