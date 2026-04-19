import { isAbsolutePath } from '../lib/acp-meta';
import { useFolderPickerStore } from '../stores/folder-picker';

type ElectronDirectoryApi = {
  openDirectory?: () => Promise<string | null>;
  showOpenDialog?: () => Promise<string | null>;
};

type DirectoryPickerWindow = Window & {
  showDirectoryPicker?: () => Promise<FileSystemDirectoryHandle>;
  electronAPI?: ElectronDirectoryApi;
};

export type DirectoryPickResult =
  | { status: 'picked'; path: string }
  | { status: 'cancelled' }
  | { status: 'unavailable' }
  | { status: 'error'; message: string };

function joinWithDetectedSeparator(basePath: string, directoryName: string): string {
  const trimmed = basePath.replace(/[\\/]+$/, '');
  if (!trimmed) {
    return directoryName;
  }

  const separator = trimmed.includes('\\') && !trimmed.includes('/') ? '\\' : '/';
  return `${trimmed}${separator}${directoryName}`;
}

function suggestAbsolutePathFromCurrent(currentPath: string, directoryName: string): string {
  const trimmedCurrent = currentPath.trim();
  if (!isAbsolutePath(trimmedCurrent)) {
    return directoryName;
  }

  const lastSegment = trimmedCurrent.replace(/[\\/]+$/, '').split(/[\\/]/).pop();
  if (lastSegment === directoryName) {
    return trimmedCurrent;
  }

  return joinWithDetectedSeparator(trimmedCurrent, directoryName);
}

async function requestAbsolutePath(initialValue = ''): Promise<string> {
  if (typeof window === 'undefined') {
    return '';
  }

  const folderPickerStore = useFolderPickerStore();
  return (await folderPickerStore.requestPath(initialValue) ?? '').trim();
}

function shouldBypassFileSystemAccess(windowRef: Window): boolean {
  const inIframe = (() => {
    try {
      return windowRef !== windowRef.top;
    } catch {
      return true;
    }
  })();

  return inIframe || windowRef.crossOriginIsolated === false;
}

async function pickWithFileSystemAccess(currentPath: string): Promise<DirectoryPickResult> {
  const browserWindow = window as DirectoryPickerWindow;
  if (shouldBypassFileSystemAccess(browserWindow)) {
    return { status: 'unavailable' };
  }
  if (typeof browserWindow.showDirectoryPicker !== 'function') {
    return { status: 'unavailable' };
  }

  try {
    const handle = await browserWindow.showDirectoryPicker();
    const suggestedPath = suggestAbsolutePathFromCurrent(currentPath, handle.name);
    const confirmedPath = (await requestAbsolutePath(suggestedPath)).trim();
    if (!confirmedPath) {
      return { status: 'cancelled' };
    }

    return { status: 'picked', path: confirmedPath };
  } catch (error) {
    if (
      (error instanceof DOMException && error.name === 'AbortError')
      || (typeof error === 'object' && error !== null && 'name' in error && error.name === 'AbortError')
    ) {
      return { status: 'cancelled' };
    }

    const message = error instanceof Error ? error.message : 'Directory picker failed';
    return { status: 'error', message };
  }
}

async function pickWithElectronBridge(): Promise<DirectoryPickResult> {
  const api = (window as DirectoryPickerWindow).electronAPI;
  if (!api) {
    return { status: 'unavailable' };
  }

  try {
    if (typeof api.showOpenDialog === 'function') {
      const selectedPath = (await api.showOpenDialog() ?? '').trim();
      if (!selectedPath) {
        return { status: 'cancelled' };
      }

      return { status: 'picked', path: selectedPath };
    }

    if (typeof api.openDirectory === 'function') {
      const selectedPath = (await api.openDirectory() ?? '').trim();
      if (!selectedPath) {
        return { status: 'cancelled' };
      }

      return { status: 'picked', path: selectedPath };
    }

    return { status: 'unavailable' };
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Electron directory picker failed';
    return { status: 'error', message };
  }
}

export async function pickAbsoluteDirectory(currentPath = ''): Promise<DirectoryPickResult> {
  if (typeof window === 'undefined') {
    return { status: 'unavailable' };
  }

  const fileSystemAccessResult = await pickWithFileSystemAccess(currentPath);
  if (fileSystemAccessResult.status !== 'unavailable') {
    return fileSystemAccessResult;
  }

  const electronBridgeResult = await pickWithElectronBridge();
  if (electronBridgeResult.status !== 'unavailable') {
    return electronBridgeResult;
  }

  const enteredPath = (await requestAbsolutePath(currentPath)).trim();
  if (!enteredPath) {
    return { status: 'cancelled' };
  }

  return { status: 'picked', path: enteredPath };
}
