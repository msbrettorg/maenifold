import { shallowRef, type Ref } from 'vue';
import type { AcpClientBridge } from '../lib/acp-client';

export type BridgeKind = AcpClientBridge;

export function useSessionBridges(): {
  bridges: Ref<Map<string, AcpClientBridge>>;
  ensureBridge(sessionId: string, kind: BridgeKind): AcpClientBridge;
  releaseBridge(sessionId: string): void;
  replaceBridge(sessionId: string, kind: BridgeKind): AcpClientBridge;
  getBridge(sessionId: string): AcpClientBridge | undefined;
  takeBridge(sessionId: string): AcpClientBridge | undefined;
} {
  const bridges = shallowRef<Map<string, AcpClientBridge>>(new Map<string, AcpClientBridge>());

  function getBridge(sessionId: string): AcpClientBridge | undefined {
    return bridges.value.get(sessionId);
  }

  function ensureBridge(sessionId: string, kind: BridgeKind): AcpClientBridge {
    return bridges.value.get(sessionId) ?? replaceBridge(sessionId, kind);
  }

  function releaseBridge(sessionId: string): void {
    bridges.value.delete(sessionId);
  }

  function replaceBridge(sessionId: string, kind: BridgeKind): AcpClientBridge {
    bridges.value.set(sessionId, kind);
    return kind;
  }

  function takeBridge(sessionId: string): AcpClientBridge | undefined {
    const bridge = getBridge(sessionId);
    releaseBridge(sessionId);
    return bridge;
  }

  return { bridges, ensureBridge, releaseBridge, replaceBridge, getBridge, takeBridge };
}
