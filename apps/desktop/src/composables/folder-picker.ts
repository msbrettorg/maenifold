import { isAbsolutePath } from '../lib/acp-meta';

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

const nativePathDialogClass =
  'fixed inset-0 z-[1000] m-0 flex h-full w-full max-h-none max-w-none items-center justify-center border-0 bg-black/60 p-0 text-inherit';
const nativePathDialogFormClass =
  'grid min-w-80 max-w-md gap-3 rounded-lg border border-border bg-card p-6 text-foreground shadow-xl';
const nativePathDialogTitleClass = 'm-0 mb-4 text-lg font-semibold text-foreground';
const nativePathDialogInputClass =
  'mb-4 w-full rounded-md border border-border bg-input px-3 py-2 text-sm text-foreground outline-none focus:border-ring focus:ring-2 focus:ring-ring/20';
const nativePathDialogActionsClass = 'flex justify-end gap-2';
const nativePathDialogButtonClass =
  'inline-flex items-center justify-center rounded-md px-4 py-2 text-sm font-medium transition-colors';
const nativePathDialogCancelButtonClass =
  `${nativePathDialogButtonClass} bg-muted text-foreground hover:bg-muted/80`;
const nativePathDialogSubmitButtonClass =
  `${nativePathDialogButtonClass} bg-primary text-primary-foreground hover:bg-primary/90`;

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

async function promptForAbsolutePath(title: string, initialValue = ''): Promise<string> {
  if (typeof document === 'undefined') {
    return '';
  }

  if (typeof HTMLDialogElement === 'undefined') {
    return (window.prompt(title, initialValue) ?? '').trim();
  }

  return new Promise((resolve) => {
    const dialog = document.createElement('dialog');
    dialog.className = nativePathDialogClass;
    const form = document.createElement('form');
    form.method = 'dialog';
    form.className = nativePathDialogFormClass;

    const heading = document.createElement('h3');
    heading.textContent = title;
    heading.className = nativePathDialogTitleClass;

    const input = document.createElement('input');
    input.type = 'text';
    input.placeholder = '/absolute/path';
    input.value = initialValue;
    input.autofocus = true;
    input.className = nativePathDialogInputClass;

    const actions = document.createElement('div');
    actions.className = nativePathDialogActionsClass;

    const cancel = document.createElement('button');
    cancel.type = 'button';
    cancel.textContent = 'Cancel';
    cancel.className = nativePathDialogCancelButtonClass;
    cancel.addEventListener('click', () => dialog.close('cancel'));

    const submit = document.createElement('button');
    submit.type = 'submit';
    submit.textContent = 'Use Path';
    submit.className = nativePathDialogSubmitButtonClass;

    actions.append(cancel, submit);
    form.append(heading, input, actions);
    dialog.append(form);
    document.body.append(dialog);

    dialog.addEventListener('close', () => {
      const value = dialog.returnValue === 'cancel' ? '' : input.value.trim();
      dialog.remove();
      resolve(value);
    }, { once: true });

    dialog.showModal();
  });
}

async function pickWithFileSystemAccess(currentPath: string): Promise<DirectoryPickResult> {
  const browserWindow = window as DirectoryPickerWindow;
  if (typeof browserWindow.showDirectoryPicker !== 'function') {
    return { status: 'unavailable' };
  }

  try {
    const handle = await browserWindow.showDirectoryPicker();
    const suggestedPath = suggestAbsolutePathFromCurrent(currentPath, handle.name);
    const confirmedPath = (await promptForAbsolutePath('Confirm working directory path', suggestedPath)).trim();
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

  const enteredPath = (await promptForAbsolutePath('Enter Working Directory', currentPath)).trim();
  if (!enteredPath) {
    return { status: 'cancelled' };
  }

  return { status: 'picked', path: enteredPath };
}
