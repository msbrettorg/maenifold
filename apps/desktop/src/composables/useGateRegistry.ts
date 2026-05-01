import { onMounted, ref, toValue, watch, type MaybeRefOrGetter } from 'vue';
import { isAbsolutePath } from '../lib/acp-meta';
import { PREFS_STORE_PATH } from '../lib/constants';
import { pickAbsoluteDirectory } from './folder-picker';

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

async function setSelectedRootInMain(root: string): Promise<void> {
  if (typeof window === 'undefined' || typeof window.electronAPI?.gateSetRoot !== 'function') return;
  await window.electronAPI.gateSetRoot({ root: normalizeRoot(root) || null });
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
      await setSelectedRootInMain(selectedGateRoot.value);
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
    const selectionResult = await pickAbsoluteDirectory(selectedGateRoot.value);

    if (selectionResult.status === 'cancelled') {
      return '';
    }
    if (selectionResult.status === 'error') {
      error.value = selectionResult.message || 'Failed to select Gate root';
      return '';
    }
    if (selectionResult.status !== 'picked') {
      error.value = 'Directory selection is unavailable in this environment.';
      return '';
    }

    const folder = normalizeRoot(selectionResult.path);
    if (!isAbsolutePath(folder)) {
      error.value = 'Gate root must be an absolute path.';
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
      await setSelectedRootInMain(selectedGateRoot.value);
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
