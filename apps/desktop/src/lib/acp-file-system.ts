import type {
  ReadTextFileRequest,
  ReadTextFileResponse,
  WriteTextFileRequest,
  WriteTextFileResponse,
} from '@agentclientprotocol/sdk';
import { assertAbsolutePath } from './acp-meta';
import { pushPlanModeWriteBlockedNotification } from './acp-chat-notifications';

type ElectronFileSystemApi = {
  readTextFile: (params: { path: string; line?: number; limit?: number }) => Promise<{ content: string }>;
  writeTextFile: (params: { path: string; content: string }) => Promise<Record<string, never>>;
};

export type WritePermissionDecision = 'allow' | 'reject' | 'always-allow-path';

type WritePermissionRequest = { path: string; content: string };
type WritePermissionGate = (request: WritePermissionRequest) => Promise<WritePermissionDecision>;

const alwaysAllowedWritePaths = new Set<string>();
let writePermissionGate: WritePermissionGate = async () => 'reject';
let modeProvider: () => string | null = () => null;

export class WritePermissionRejectedError extends Error {
  constructor(public readonly path: string, reason?: string) {
    super(reason ? `Write to ${path} rejected: ${reason}` : `Write to ${path} rejected`);
    this.name = 'WritePermissionRejectedError';
  }
}

export function setWritePermissionGate(gate: WritePermissionGate): void {
  writePermissionGate = gate;
}

export function setCurrentModeProvider(provider: () => string | null): void {
  modeProvider = provider;
}

export function currentModeId(): string | null {
  return modeProvider();
}

export function rememberWritePathAlwaysAllowed(path: string): void {
  alwaysAllowedWritePaths.add(path);
}

export function isWritePathAlwaysAllowed(path: string): boolean {
  return alwaysAllowedWritePaths.has(path);
}

export async function requestWritePermission(
  request: WritePermissionRequest,
): Promise<WritePermissionDecision> {
  return writePermissionGate(request);
}

function getElectronFileSystemApi(): ElectronFileSystemApi {
  const api = typeof window === 'undefined' ? undefined : window.electronAPI;
  if (
    !api ||
    typeof api.readTextFile !== 'function' ||
    typeof api.writeTextFile !== 'function'
  ) {
    throw new Error('Electron ACP file access is unavailable in this environment');
  }
  return {
    readTextFile: api.readTextFile,
    writeTextFile: api.writeTextFile,
  };
}

export async function handleWriteTextFileRequest(
  params: WriteTextFileRequest,
): Promise<WriteTextFileResponse> {
  assertAbsolutePath(params.path);
  const fileSystem = await import('./acp-file-system');
  if (fileSystem.currentModeId() === 'plan') {
    pushPlanModeWriteBlockedNotification(params.path);
    throw new WritePermissionRejectedError(params.path, 'plan mode');
  }
  if (!fileSystem.isWritePathAlwaysAllowed(params.path)) {
    const decision = await fileSystem.requestWritePermission({
      path: params.path,
      content: params.content,
    });
    if (decision === 'reject') {
      throw new WritePermissionRejectedError(params.path);
    }
    if (decision === 'always-allow-path') {
      fileSystem.rememberWritePathAlwaysAllowed(params.path);
    }
  }
  await getElectronFileSystemApi().writeTextFile({
    path: params.path,
    content: params.content,
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
  return getElectronFileSystemApi().readTextFile({
    ...request,
  });
}
