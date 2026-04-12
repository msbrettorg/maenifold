export function storageKey(namespace: string, key: string): string {
  return `${namespace}:${key}`;
}

function getLocalStorage(): Storage | null {
  if (typeof window === 'undefined') {
    return null;
  }

  return window.localStorage;
}

export function readStorageValue<T>(namespace: string, key: string): T | null {
  const storage = getLocalStorage();
  if (!storage) {
    return null;
  }

  const raw = storage.getItem(storageKey(namespace, key));
  if (raw === null) {
    return null;
  }

  try {
    return JSON.parse(raw) as T;
  } catch {
    return raw as T;
  }
}

export function writeStorageValue(namespace: string, key: string, value: unknown): void {
  const storage = getLocalStorage();
  if (!storage) {
    return;
  }

  storage.setItem(storageKey(namespace, key), JSON.stringify(value));
}

export function removeStorageValue(namespace: string, key: string): void {
  const storage = getLocalStorage();
  if (!storage) {
    return;
  }

  storage.removeItem(storageKey(namespace, key));
}

export function hasLocalStorage(): boolean {
  return getLocalStorage() !== null;
}
