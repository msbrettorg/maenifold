import { useContext } from "react";

import { BridgeContext } from "./BridgeProvider";

export function useBridge() {
  const bridge = useContext(BridgeContext);

  if (!bridge) {
    throw new Error("useBridge must be used within a BridgeProvider");
  }

  return bridge;
}
