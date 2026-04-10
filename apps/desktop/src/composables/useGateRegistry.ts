import { onMounted, ref, toValue, watch, type MaybeRefOrGetter } from 'vue';
import { open } from '@tauri-apps/plugin-dialog';
import { load, type Store } from '@tauri-apps/plugin-store';
import { PREFS_STORE_PATH } from '../lib/constants';

const GATE_ROOTS_KEY = 'gateRoots';
const SELECTED_GATE_ROOT_KEY = 'selectedGateRoot';

function normalizeRoot(value: string | null | undefined) {
  return value?.trim() ?? '';
}

function uniqueRoots(values: string[]) {
  return [...new Set(values.map(normalizeRoot).filter(Boolean))];
}

export function useGateRegistry(seedRoot: MaybeRefOrGetter<string>) {
  const gateRoots = ref<string[]>([]);
  const selectedGateRoot = ref('');
  const isReady = ref(false);
  const error = ref('');

  let prefsStore: Store | null = null;

  async function persist() {
    if (!prefsStore) return;

    error.value = '';

    try {
      await prefsStore.set(GATE_ROOTS_KEY, gateRoots.value);
      await prefsStore.set(SELECTED_GATE_ROOT_KEY, selectedGateRoot.value);
      await prefsStore.save();
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
    const folder = await open({
      directory: true,
      multiple: false,
      title: 'Register Gate Root',
    });

    return typeof folder === 'string' ? addAndSelectRoot(folder) : '';
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
      prefsStore = await load(PREFS_STORE_PATH);
      gateRoots.value = uniqueRoots((await prefsStore.get<string[]>(GATE_ROOTS_KEY)) ?? []);
      selectedGateRoot.value = normalizeRoot(
        (await prefsStore.get<string>(SELECTED_GATE_ROOT_KEY)) ?? gateRoots.value[0] ?? '',
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
