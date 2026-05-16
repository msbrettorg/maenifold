export function storageKey(namespace: string, key: string): string {
  return `${namespace}:${key}`;
}
