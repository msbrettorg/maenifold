export const MCP_BASE_URL = (import.meta.env.VITE_MCP_BASE_URL as string | undefined)?.trim() || 'http://127.0.0.1:8080';

// The privileged /api/* surface (config, user-prompt, machine-id, acp/session, health)
// lives on a separate loopback-only listener so the MCP listener (above) can be
// safely exposed to a tailnet via Tailscale Serve without leaking the privileged
// API. See docs/lan-exposure.md and crates/maenifold-core/src/mcp/http.rs.
export const LOCAL_API_BASE_URL =
  (import.meta.env.VITE_LOCAL_API_BASE_URL as string | undefined)?.trim() ||
  'http://127.0.0.1:8081';
