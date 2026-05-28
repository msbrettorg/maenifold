export type Transport = "mcp-apps" | "wry" | "none";

export function detectTransport(): Transport {
  if (typeof window === "undefined") {
    return "none";
  }

  if (typeof window.maenifold === "object" && window.maenifold !== null) {
    return "wry";
  }

  if (window.parent !== window) {
    return "mcp-apps";
  }

  return "none";
}
