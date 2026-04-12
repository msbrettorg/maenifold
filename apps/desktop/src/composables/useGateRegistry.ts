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
    dialog.className = 'native-path-dialog';
    const form = document.createElement('form');
    form.method = 'dialog';
    form.className = 'native-path-dialog__form';

    const heading = document.createElement('h3');
    heading.textContent = 'Register Gate Root';
    heading.className = 'native-path-dialog__title';

    const input = document.createElement('input');
    input.type = 'text';
    input.value = initialValue;
    input.placeholder = '/absolute/path';
    input.autofocus = true;
    input.className = 'native-path-dialog__input';

    const actions = document.createElement('div');
    actions.className = 'native-path-dialog__actions';

    const cancel = document.createElement('button');
    cancel.type = 'button';
    cancel.textContent = 'Cancel';
    cancel.className = 'native-path-dialog__button native-path-dialog__button--cancel';
    cancel.addEventListener('click', () => dialog.close('cancel'));

    const submit = document.createElement('button');
    submit.type = 'submit';
    submit.textContent = 'Use Path';
    submit.className = 'native-path-dialog__button native-path-dialog__button--submit';

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
