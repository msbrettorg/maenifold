import type { ResourceContent } from '../composables/mcpTransport';

export function gateResourceUri(path: string, root: string): string {
  if (!root) {
    return `gate://${path}`;
  }
  return `gate://${path}?root=${encodeURIComponent(root)}`;
}

export function parseResourcePayload<T>(content: ResourceContent): T {
  const text = content.text;
  if (!text) {
    throw new Error(`Resource ${content.uri} returned no text payload.`);
  }
  return JSON.parse(text) as T;
}
