import { ref, type Ref } from 'vue';
import type { SessionState } from '../lib/types';
import { useMcpClient } from './useMcpClient';
import { warnRawResourceErrorInDev } from '../lib/error-sanitizer';
import {
  buildCopilotReasoningEffortMap,
  isCopilotSession,
  type CopilotReasoningEffortMap,
} from '../stores/session-copilot-models';

export interface ModelWarning {
  key: string;
  message: string;
}

export function useSessionModels(): {
  modelReasoningEfforts: Ref<Map<string, CopilotReasoningEffortMap>>;
  warnings: Ref<readonly ModelWarning[]>;
  ensureSessionModels(localId: string, session: SessionState): Promise<void>;
} {
  const mcp = useMcpClient();
  const modelReasoningEfforts = ref<Map<string, CopilotReasoningEffortMap>>(new Map());
  const warnings = ref<ModelWarning[]>([]);
  const loads = new Map<string, Promise<void>>();
  const warned = new Set<string>();

  function warnOnce(key: string, message: string): void {
    if (warned.has(key)) return;
    warned.add(key);
    warnings.value = [...warnings.value, { key, message }];
    warnRawResourceErrorInDev(message);
  }

  function setSessionReasoningEfforts(localId: string, reasoningEfforts: CopilotReasoningEffortMap): void {
    const next = new Map(modelReasoningEfforts.value);
    next.set(localId, reasoningEfforts);
    modelReasoningEfforts.value = next;
  }

  async function ensureSessionModels(localId: string, session: SessionState): Promise<void> {
    if (!session.isConnected || !isCopilotSession(session)) return;
    if (modelReasoningEfforts.value.has(localId) || loads.has(localId)) return;
    const load = (async () => {
      try {
        if (!mcp.connected.value) await mcp.connect();
        const response = await mcp.gatewayExecute('maenifold.system', 'GetCopilotModels', { refresh: false }) as {
          models?: Array<{
            id?: string;
            capabilities?: { supports?: { reasoning_effort?: unknown } };
          }>;
        };
        setSessionReasoningEfforts(localId, buildCopilotReasoningEffortMap(response));
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        warnOnce(`copilot-models:${localId}`, `[GetCopilotModels] ${message}`);
      } finally {
        loads.delete(localId);
      }
    })();
    loads.set(localId, load);
    await load;
  }

  return { modelReasoningEfforts, warnings, ensureSessionModels };
}
