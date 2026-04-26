import { computed, ref } from 'vue';
import { DEFAULT_GATE_MCP_BASE_URL } from '../lib/constants';
import {
  createGateSnapshot,
  GATE_POLL_INTERVAL_MS,
  type GateSnapshot,
  type GateStatusArgs,
  type GateSyncArgs,
  type GateSyncResult,
  type GateTaskBody,
  type GateTask,
  type GateTaskHistoryEntry,
} from '../lib/gate';
import { unwrapToolData } from './pipeline-mcp-transport';
import { getSharedMcpClient } from './usePipelineMcp';

const GATE_SYNC_TOOLS = ['sync', 'gate-sync'] as const;
const GATE_STATUS_TOOLS = ['status', 'gate-status'] as const;
const GATE_HISTORY_TOOLS = ['history', 'gate-history'] as const;
const GATE_TASK_BODY_TOOLS = ['task_body'] as const;

interface GatePollingState {
  root: string | null;
  handle: ReturnType<typeof setInterval> | null;
}

export function useGateMcp(baseUrl = DEFAULT_GATE_MCP_BASE_URL) {
  const mcp = getSharedMcpClient(baseUrl);
  const pollingError = ref<string | null>(null);
  const error = computed(() => pollingError.value ?? mcp.error.value);
  const polling: GatePollingState = {
    root: null,
    handle: null,
  };

  async function ensureConnected() {
    if (mcp.connected.value) {
      return;
    }

    await mcp.connect();
  }

  function isUnknownToolError(errorValue: unknown) {
    const message = errorValue instanceof Error ? errorValue.message : String(errorValue);
    return /unknown tool|tool not found|unknown gate MCP tool/i.test(message);
  }

  async function callGateTool<T>(toolNames: readonly string[], payload: Record<string, unknown> = {}) {
    await ensureConnected();
    let lastError: unknown = null;

    for (const toolName of toolNames) {
      try {
        const result = await mcp.callTool(toolName, payload);
        return unwrapToolData<T>(result);
      } catch (toolError) {
        lastError = toolError;
        if (!isUnknownToolError(toolError)) {
          throw toolError;
        }
      }
    }

    throw lastError instanceof Error ? lastError : new Error(String(lastError));
  }

  function stopPolling() {
    if (polling.handle !== null) {
      clearInterval(polling.handle);
      polling.handle = null;
    }

    polling.root = null;
    pollingError.value = null;
  }

  function startPolling(root: string, refresh: () => Promise<void> | void, intervalMs = GATE_POLL_INTERVAL_MS) {
    if (!root) {
      stopPolling();
      return;
    }

    if (polling.root === root && polling.handle !== null) {
      return;
    }

    stopPolling();
    polling.root = root;
    polling.handle = setInterval(() => {
      void Promise.resolve(refresh()).catch((pollErrorValue: unknown) => {
        pollingError.value = pollErrorValue instanceof Error
          ? pollErrorValue.message
          : String(pollErrorValue);
      });
    }, intervalMs);
  }

  function sync(args: GateSyncArgs): Promise<GateSyncResult> {
    return callGateTool<GateSyncResult>(GATE_SYNC_TOOLS, args);
  }

  function normalizeTaskStages(task: GateTask): GateTask {
    const filtered = Array.isArray(task.stages) ? task.stages : [];
    return { ...task, stages: filtered };
  }

  async function status(args: GateStatusArgs): Promise<GateTask[]> {
    const tasks = await callGateTool<GateTask[]>(GATE_STATUS_TOOLS, args);
    return Array.isArray(tasks) ? tasks.map(normalizeTaskStages) : tasks;
  }

  function readTaskHistory(taskId: string, root?: string): Promise<GateTaskHistoryEntry[]> {
    return callGateTool<GateTaskHistoryEntry[]>(GATE_HISTORY_TOOLS, {
      taskId,
      ...(root ? { root } : {}),
    });
  }

  function readTaskBody(taskId: string, root?: string): Promise<GateTaskBody> {
    return callGateTool<GateTaskBody>(GATE_TASK_BODY_TOOLS, {
      taskId,
      ...(root ? { root } : {}),
    });
  }

  async function loadSnapshot(root: string): Promise<GateSnapshot> {
    await sync({ root });
    const tasks = await status({ root, includeDone: true });
    return createGateSnapshot(root, tasks);
  }

  async function disconnect() {
    stopPolling();
    await mcp.disconnect();
  }

  return {
    connect: mcp.connect,
    disconnect,
    connected: mcp.connected,
    reconnecting: mcp.reconnecting,
    error,
    loadSnapshot,
    sync,
    status,
    readTaskBody,
    readTaskHistory,
    startPolling,
    stopPolling,
  };
}
