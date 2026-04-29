const alwaysAllowed = new Set<string>();

export function isAlwaysAllowed(path: string): boolean {
  return alwaysAllowed.has(path);
}

export function rememberAlwaysAllowed(path: string): void {
  alwaysAllowed.add(path);
}

export function resetWritePermissionAllowlistForTest(): void {
  alwaysAllowed.clear();
}
