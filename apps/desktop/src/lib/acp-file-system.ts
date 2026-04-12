import type {
  ReadTextFileRequest,
  ReadTextFileResponse,
  WriteTextFileRequest,
  WriteTextFileResponse,
} from '@agentclientprotocol/sdk';

function ensureBrowserFsAvailable() {
  const browserWindow = window as Window & {
    showSaveFilePicker?: unknown;
    showOpenFilePicker?: unknown;
  };

  if (
    typeof window === 'undefined' ||
    !browserWindow.showSaveFilePicker ||
    !browserWindow.showOpenFilePicker
  ) {
    throw new Error('Browser file access is unavailable in this environment');
  }
}

export async function handleWriteTextFileRequest(
  params: WriteTextFileRequest,
): Promise<WriteTextFileResponse> {
  ensureBrowserFsAvailable();
  const picker = (window as Window & {
    showSaveFilePicker?: unknown;
  }).showSaveFilePicker as unknown as (options?: Record<string, unknown>) => Promise<{
    createWritable: () => Promise<{ write: (content: string) => Promise<void>; close: () => Promise<void> }>;
  }>;
  const handle = await picker({
    suggestedName: params.path?.split('/').pop() ?? 'untitled.txt',
  });
  const writable = await handle.createWritable();
  await writable.write(params.content);
  await writable.close();
  return {};
}

export async function handleReadTextFileRequest(
  _params: ReadTextFileRequest,
): Promise<ReadTextFileResponse> {
  ensureBrowserFsAvailable();
  const picker = (window as Window & {
    showOpenFilePicker?: unknown;
  }).showOpenFilePicker as unknown as () => Promise<Array<{
    getFile: () => Promise<{ text: () => Promise<string> }>;
  }>>;
  const [handle] = await picker();
  if (!handle) {
    throw new Error('No file selected');
  }
  const file = await handle.getFile();
  return { content: await file.text() };
}
