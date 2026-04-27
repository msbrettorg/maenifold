import type {
  ReadTextFileRequest,
  ReadTextFileResponse,
  WriteTextFileRequest,
  WriteTextFileResponse,
} from '@agentclientprotocol/sdk';
import { assertAbsolutePath } from './acp-meta';

type ElectronFileSystemApi = {
  readTextFile: (params: { path: string; line?: number; limit?: number }) => Promise<{ content: string }>;
  writeTextFile: (params: { path: string; content: string }) => Promise<Record<string, never>>;
};

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
