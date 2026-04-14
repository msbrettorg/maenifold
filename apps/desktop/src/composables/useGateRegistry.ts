import { onMounted, ref, toValue, watch, type MaybeRefOrGetter } from 'vue';
import { PREFS_STORE_PATH } from '../lib/constants';

const GATE_ROOTS_KEY = 'gateRoots';
const SELECTED_GATE_ROOT_KEY = 'selectedGateRoot';

function normalizeRoot(value: string | null | undefined) {
  return value?.trim() ?? '';
}

function uniqueRoots(values: string[]) {
  return [...new Set(values.map(normalizeRoot).filter(Boolean))];
}

function storageKey(key: string) {
  return `${PREFS_STORE_PATH}:${key}`;
}

function browserStorageAvailable() {
  return typeof window !== 'undefined' && typeof window.localStorage !== 'undefined';
}

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

async function readStoredValue<T>(key: string): Promise<T | null> {
  if (!browserStorageAvailable()) {
    return null;
  }

  const raw = window.localStorage.getItem(storageKey(key));
  if (raw === null) {
    return null;
  }

  try {
    return JSON.parse(raw) as T;
  } catch {
    return raw as T;
  }
}

async function writeStoredValue(key: string, value: unknown): Promise<void> {
  if (!browserStorageAvailable()) {
    return;
  }

  window.localStorage.setItem(storageKey(key), JSON.stringify(value));
}

async function promptForRoot(initialValue = ''): Promise<string> {
  if (typeof document === 'undefined') {
    return '';
  }

  if (typeof HTMLDialogElement === 'undefined') {
    return normalizeRoot(window.prompt('Enter the absolute Gate root path', initialValue) ?? '');
  }

  return new Promise((resolve) => {
    const dialog = document.createElement('dialog');
    dialog.className = nativePathDialogClass;
    const form = document.createElement('form');
    form.method = 'dialog';
    form.className = nativePathDialogFormClass;

    const heading = document.createElement('h3');
    heading.textContent = 'Register Gate Root';
    heading.className = nativePathDialogTitleClass;

    const input = document.createElement('input');
    input.type = 'text';
    input.value = initialValue;
    input.placeholder = '/absolute/path';
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
      const value = dialog.returnValue === 'cancel' ? '' : normalizeRoot(input.value);
      dialog.remove();
      resolve(value);
    }, { once: true });

    dialog.showModal();
  });
}

export function useGateRegistry(seedRoot: MaybeRefOrGetter<string>) {
  const gateRoots = ref<string[]>([]);
  const selectedGateRoot = ref('');
  const isReady = ref(false);
  const error = ref('');

  async function persist() {
    error.value = '';

    try {
      await writeStoredValue(GATE_ROOTS_KEY, gateRoots.value);
      await writeStoredValue(SELECTED_GATE_ROOT_KEY, selectedGateRoot.value);
    } catch (persistError) {
      error.value = persistError instanceof Error ? persistError.message : String(persistError);
    }
  }

  async function addAndSelectRoot(root: string) {
    const normalized = normalizeRoot(root);
    if (!normalized) return '';

    gateRoots.value = uniqueRoots([...gateRoots.value, normalized]);
    selectedGateRoot.value = normalized;
    await persist();
    return normalized;
  }

  async function selectRoot(root: string) {
    selectedGateRoot.value = normalizeRoot(root);
    await persist();
  }

  async function browseForRoot() {
    error.value = '';
    const folder = await promptForRoot(selectedGateRoot.value);

    if (!folder) {
      if (!browserStorageAvailable()) {
        error.value = 'Gate root selection requires browser localStorage support.';
      }
      return '';
    }

    return addAndSelectRoot(folder);
  }

  async function removeSelectedRoot() {
    if (!selectedGateRoot.value) return '';

    const removed = selectedGateRoot.value;
    gateRoots.value = gateRoots.value.filter((root) => root !== removed);
    selectedGateRoot.value = gateRoots.value[0] ?? '';
    await persist();
    return removed;
  }

  function seedFromWorkspace() {
    const workspaceRoot = normalizeRoot(toValue(seedRoot));
    if (!workspaceRoot || gateRoots.value.length || selectedGateRoot.value) return;
    void addAndSelectRoot(workspaceRoot);
  }

  onMounted(async () => {
    try {
      gateRoots.value = uniqueRoots((await readStoredValue<string[]>(GATE_ROOTS_KEY)) ?? []);
      selectedGateRoot.value = normalizeRoot(
        (await readStoredValue<string>(SELECTED_GATE_ROOT_KEY)) ?? gateRoots.value[0] ?? '',
      );
      gateRoots.value = uniqueRoots([...gateRoots.value, selectedGateRoot.value]);
    } catch (loadError) {
      error.value = loadError instanceof Error ? loadError.message : String(loadError);
    } finally {
      isReady.value = true;
      seedFromWorkspace();
    }
  });

  watch(
    () => normalizeRoot(toValue(seedRoot)),
    () => {
      if (!isReady.value) return;
      seedFromWorkspace();
    },
  );

  return {
    gateRoots,
    selectedGateRoot,
    isReady,
    error,
    browseForRoot,
    addAndSelectRoot,
    selectRoot,
    removeSelectedRoot,
  };
}
