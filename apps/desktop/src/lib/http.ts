export function getHttpFetch(): typeof globalThis.fetch {
  return globalThis.fetch.bind(globalThis);
}
