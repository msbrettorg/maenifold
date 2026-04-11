import { DEFAULT_MCP_BASE_URL } from './constants';

/**
 * Derive a fetchable backend origin from the current browser location.
 *
 * Returning an absolute origin keeps requests stable in browser, jsdom, and
 * Tauri environments. The desktop app always targets the maenifold backend
 * directly rather than the Vite dev origin.
 */
export function getApiBase(): string {
  if (typeof window === 'undefined') {
    return '';
  }

  return import.meta.env.VITE_MCP_BASE_URL ?? DEFAULT_MCP_BASE_URL;
}
