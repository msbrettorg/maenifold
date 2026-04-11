import { onMounted, ref, toValue, watch, type MaybeRefOrGetter } from 'vue';
import { PREFS_STORE_PATH } from '../lib/constants';

const GATE_ROOTS_KEY = 'gateRoots';
const SELECTED_GATE_ROOT_KEY = 'selectedGateRoot';

type GateRegistryStore = {
  get<T>(key: string): Promise<T | null>;
  set(key: string, value: unknown): Promise<void>;
  save(): Promise<void>;
};

function normalizeRoot(value: string | null | undefined) {
  return value?.trim() ?? '';
}

function uniqueRoots(values: string[]) {
  return [...new Set(values.map(normalizeRoot).filter(Boolean))];
}

function browserStorageAvailable() {
  return typeof window !== 'undefined' && typeof window.localStorage !== 'undefined';
}

function createBrowserStore(): GateRegistryStore | null {
  if (!browserStorageAvailable()) return null;

  return {
    async get<T>(key: string) {
      const raw = window.localStorage.getItem(`${PREFS_STORE_PATH}:${key}`);
      if (raw === null) return null;

      try {
        return JSON.parse(raw) as T;
      } catch {
        return raw as T;
      }
    },
    async set(key: string, value: unknown) {
      window.localStorage.setItem(`${PREFS_STORE_PATH}:${key}`, JSON.stringify(value));
    },
    async save() {
      // Browser persistence is written eagerly to localStorage.
    },
  };
}

async function loadPrefsStore(): Promise<GateRegistryStore | null> {
  try {
    const { load } = await import('@tauri-apps/plugin-store');
    const store = await load(PREFS_STORE_PATH);
    return {
      async get<T>(key: string) {
        return (await store.get<T>(key)) ?? null;
      },
      async set(key: string, value: unknown) {
        await store.set(key, value);
      },
      async save() {
        await store.save();
      },
    };
  } catch {
    return createBrowserStore();
  }
}

async function pickGateRoot(): Promise<string> {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const folder = await open({
      directory: true,
      multiple: false,
      title: 'Register Gate Root',
    });
    return typeof folder === 'string' ? folder : '';
  } catch {
    if (typeof window === 'undefined' || typeof window.prompt !== 'function') {
      return '';
    }

    return normalizeRoot(window.prompt('Enter the absolute Gate root path') ?? '');
  }
}

export function useGateRegistry(seedRoot: MaybeRefOrGetter<string>) {
  const gateRoots = ref<string[]>([]);
  const selectedGateRoot = ref('');
  const isReady = ref(false);
  const error = ref('');

  let prefsStore: GateRegistryStore | null = null;

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
    error.value = '';
    const folder = await pickGateRoot();

    if (!folder) {
      if (!browserStorageAvailable()) {
        error.value = 'Gate root selection requires a Tauri desktop runtime or browser prompt support.';
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
      prefsStore = await loadPrefsStore();
      gateRoots.value = uniqueRoots((await prefsStore?.get<string[]>(GATE_ROOTS_KEY)) ?? []);
      selectedGateRoot.value = normalizeRoot(
        (await prefsStore?.get<string>(SELECTED_GATE_ROOT_KEY)) ?? gateRoots.value[0] ?? '',
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
