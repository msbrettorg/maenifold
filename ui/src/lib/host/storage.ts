// Minimal KV-store shim with two backends:
//  - on Tauri, defers to `@tauri-apps/plugin-store` so behaviour matches
//    today's persistence (keyed JSON file in the app's data dir);
//  - on web, uses `localStorage` under a single `acp-ui:<name>` namespace.
//
// The exposed shape mirrors the subset of `plugin-store` we actually use
// (`get`, `set`, `save`) so we can swap backends without touching call
// sites in the session/preferences stores.

import { isTauriHost } from '../platform';

export interface KVStore {
  get<T>(key: string): Promise<T | null>;
  set(key: string, value: unknown): Promise<void>;
  save(): Promise<void>;
}

class WebKVStore implements KVStore {
  private readonly storageKey: string;
  private data: Record<string, unknown>;

  constructor(name: string) {
    this.storageKey = `acp-ui:${name}`;
    this.data = {};
    if (typeof localStorage !== 'undefined') {
      const raw = localStorage.getItem(this.storageKey);
      if (raw) {
        try {
          this.data = JSON.parse(raw) ?? {};
        } catch (e) {
          console.warn(`Failed to parse ${this.storageKey} from localStorage:`, e);
          this.data = {};
        }
      }
    }
  }

  async get<T>(key: string): Promise<T | null> {
    const v = this.data[key];
    return (v === undefined ? null : (v as T));
  }

  async set(key: string, value: unknown): Promise<void> {
    this.data[key] = value;
  }

  async save(): Promise<void> {
    if (typeof localStorage === 'undefined') return;
    try {
      localStorage.setItem(this.storageKey, JSON.stringify(this.data));
    } catch (e) {
      // Quota exceeded, private mode, etc. — surface to the console; the
      // session/preferences code already tolerates a missing store.
      console.warn(`Failed to persist ${this.storageKey} to localStorage:`, e);
    }
  }
}

/**
 * Open (or create) a persistent KV store. Returns an interface that matches
 * the slice of `@tauri-apps/plugin-store` we use today.
 */
export async function loadKvStore(name: string): Promise<KVStore> {
  if (isTauriHost()) {
    // Defer the import so the web bundle doesn't pay for it. Vite turns
    // this into a separate chunk that's only fetched on Tauri.
    const { load } = await import('@tauri-apps/plugin-store');
    const store = await load(name);
    return {
      async get<T>(key: string): Promise<T | null> {
        const v = await store.get<T>(key);
        return v === undefined ? null : v;
      },
      set: (key, value) => store.set(key, value),
      save: () => store.save(),
    };
  }
  return new WebKVStore(name);
}
