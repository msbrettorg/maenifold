import { useApp } from "@modelcontextprotocol/ext-apps/react";
import { createContext, type PropsWithChildren, useMemo } from "react";

import { detectTransport, type Transport } from "./transport";
import { WryBridge } from "./wry";

export type Bridge = {
  transport: Transport;
  invoke: <T>(tool: string, args: unknown) => Promise<T>;
  subscribe: <T>(channel: string, cb: (message: T) => void) => () => void;
};

export const BridgeContext = createContext<Bridge | null>(null);

const NO_TRANSPORT_ERROR =
  "No transport: maenifold-console is not running inside an MCP host or WRY tray viewer";

const noneBridge: Bridge = {
  transport: "none",
  async invoke() {
    throw new Error(NO_TRANSPORT_ERROR);
  },
  subscribe() {
    return () => {};
  },
};

function StaticBridgeProvider({
  bridge,
  children,
}: PropsWithChildren<{ bridge: Bridge }>) {
  return (
    <BridgeContext.Provider value={bridge}>{children}</BridgeContext.Provider>
  );
}

function WryBridgeProvider({ children }: PropsWithChildren) {
  const bridge = useMemo<Bridge>(() => {
    const wryBridge = new WryBridge();

    return {
      transport: "wry",
      invoke: (tool, args) => wryBridge.invoke(tool, args),
      subscribe: (channel, cb) => wryBridge.subscribe(channel, cb),
    };
  }, []);

  return (
    <StaticBridgeProvider bridge={bridge}>{children}</StaticBridgeProvider>
  );
}

function McpAppsBridgeProvider({ children }: PropsWithChildren) {
  const { app, isConnected, error } = useApp({
    appInfo: { name: "maenifold-console", version: "0.0.1" },
    capabilities: {},
  });

  const bridge = useMemo<Bridge>(
    () => ({
      transport: "mcp-apps",
      async invoke<T>(tool: string, args: unknown): Promise<T> {
        if (error) {
          throw error;
        }

        if (!app || !isConnected) {
          throw new Error("MCP Apps bridge is not connected yet");
        }

        return (await app.callServerTool({
          name: tool,
          arguments: (args ?? {}) as Record<string, unknown>,
        })) as T;
      },
      subscribe(channel) {
        console.debug(
          "[bridge] mcp-apps subscribe is a no-op until T-251.14 wires daemon-side notifications",
          channel,
        );

        return () => {};
      },
    }),
    [app, error, isConnected],
  );

  return (
    <StaticBridgeProvider bridge={bridge}>{children}</StaticBridgeProvider>
  );
}

export function BridgeProvider({ children }: PropsWithChildren) {
  const transport = detectTransport();

  if (transport === "mcp-apps") {
    return <McpAppsBridgeProvider>{children}</McpAppsBridgeProvider>;
  }

  if (transport === "wry") {
    return <WryBridgeProvider>{children}</WryBridgeProvider>;
  }

  return (
    <StaticBridgeProvider bridge={noneBridge}>{children}</StaticBridgeProvider>
  );
}
