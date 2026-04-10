import type {
  ReadTextFileRequest,
  ReadTextFileResponse,
  WriteTextFileRequest,
  WriteTextFileResponse,
} from '@agentclientprotocol/sdk';
import {
  mkdir,
  readTextFile as tauriReadTextFile,
  writeTextFile as tauriWriteTextFile,
} from '@tauri-apps/plugin-fs';
import { dirname } from '@tauri-apps/api/path';
import { assertAbsolutePath } from './acp-meta';

export async function handleWriteTextFileRequest(
  params: WriteTextFileRequest
): Promise<WriteTextFileResponse> {
  try {
    assertAbsolutePath(params.path);
    const dir = await dirname(params.path);
    await mkdir(dir, { recursive: true });
    await tauriWriteTextFile(params.path, params.content);
    if (import.meta.env.DEV) {
      console.log('writeTextFile completed:', params.path);
    }
    return null as unknown as WriteTextFileResponse;
  } catch (e) {
    console.error('writeTextFile failed:', params.path, e);
    throw e;
  }
}

export async function handleReadTextFileRequest(
  params: ReadTextFileRequest
): Promise<ReadTextFileResponse> {
  try {
    assertAbsolutePath(params.path);
    let content = await tauriReadTextFile(params.path);
    if (params.line !== undefined || params.limit !== undefined) {
      const lines = content.split('\n');
      const startLine = params.line ? params.line - 1 : 0;
      const endLine = params.limit ? startLine + params.limit : lines.length;
      content = lines.slice(startLine, endLine).join('\n');
    }
    if (import.meta.env.DEV) {
      console.log('readTextFile completed:', params.path);
    }
    return { content, _meta: null };
  } catch (e) {
    console.error('readTextFile failed:', params.path, e);
    throw e;
  }
}
