export const MCP_BASE_URL = (import.meta.env.VITE_MCP_BASE_URL as string | undefined)?.trim() || 'http://127.0.0.1:8080';

// The /api/* surface (config, user-prompt, machine-id, acp/session, health) is
// currently co-served on the MCP listener: crates/maenifold-core/src/mcp/http.rs
// binds a single TcpListener on 127.0.0.1:{port} and serves both /mcp and /api/*
// from it (there is no separate local-api listener or --local-api-port flag).
// So the default API base is the MCP base. The VITE_LOCAL_API_BASE_URL override
// exists only so a future deployment that splits /api/* onto its own loopback
// listener can point the UI at it without a code change.
export const LOCAL_API_BASE_URL =
  (import.meta.env.VITE_LOCAL_API_BASE_URL as string | undefined)?.trim() ||
  MCP_BASE_URL;
