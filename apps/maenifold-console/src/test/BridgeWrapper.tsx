import type { PropsWithChildren } from "react";
import { vi } from "vitest";

import { BridgeContext, type Bridge } from "@/lib/bridge/BridgeProvider";

export function makeMockBridge(overrides: Partial<Bridge> = {}): Bridge {
  return {
    transport: "none",
    invoke: vi.fn().mockResolvedValue({}),
    subscribe: vi.fn().mockReturnValue(() => {}),
    ...overrides,
  };
}

export function BridgeWrapper({
  bridge,
  children,
}: PropsWithChildren<{ bridge: Bridge }>) {
  return <BridgeContext.Provider value={bridge}>{children}</BridgeContext.Provider>;
}
