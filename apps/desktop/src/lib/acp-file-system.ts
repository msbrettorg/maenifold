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
  writeTextFile: (params: { path: string; content: string; token?: string }) => Promise<Record<string, never>>;
  issueWritePermissionToken: (params: { path: string; allowsAlways?: boolean }) => Promise<string>;
  isWritePathAlwaysAllowed: (path: string) => Promise<boolean>;
};

export type WritePermissionDecision = 'allow' | 'reject' | 'always-allow-path';

type WritePermissionRequest = { path: string; content: string };
export type WritePermissionPolicy = {
  currentModeId: () => string | null;
  requestPermission: (request: WritePermissionRequest) => Promise<WritePermissionDecision>;
  pathAllowList: { has: (path: string) => boolean; add: (path: string) => void };
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
    typeof api.writeTextFile !== 'function' ||
    typeof api.issueWritePermissionToken !== 'function' ||
    typeof api.isWritePathAlwaysAllowed !== 'function'
  ) {
    throw new Error('Electron ACP file access is unavailable in this environment');
  }
  return {
    writeTextFile: api.writeTextFile,
    issueWritePermissionToken: api.issueWritePermissionToken,
    isWritePathAlwaysAllowed: api.isWritePathAlwaysAllowed,
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
  const isAlwaysAllowed =
    policy.pathAllowList.has(params.path) ||
    await electronFileSystem.isWritePathAlwaysAllowed(params.path);
  let token: string | undefined;
  if (!isAlwaysAllowed) {
    const decision = await policy.requestPermission({
      path: params.path,
      content: params.content,
    });
    if (decision === 'reject') {
      throw new WritePermissionRejectedError(params.path);
    }
    token = await electronFileSystem.issueWritePermissionToken({
      path: params.path,
      allowsAlways: decision === 'always-allow-path',
    });
    if (decision === 'always-allow-path') {
      policy.pathAllowList.add(params.path);
    }
  }
  await electronFileSystem.writeTextFile({
    path: params.path,
    content: params.content,
    token,
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
