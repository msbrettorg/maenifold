import { computed, ref } from 'vue';
import {
  createGateSnapshot,
  GATE_POLL_INTERVAL_MS,
  type GateCreateArgs,
  type GateCreateResult,
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

const GATE_DOMAIN = 'maenifold.gate';

interface GatewayCommand {
  name?: unknown;
}

interface GatewayIndexResult {
  commands?: GatewayCommand[];
}

interface GatePollingState {
  root: string | null;
  handle: ReturnType<typeof setInterval> | null;
}

type JsonPrimitive = string | number | boolean | null;
type JsonValue = JsonPrimitive | JsonValue[] | { [property: string]: JsonValue };
type JsonObject = { [property: string]: JsonValue };

export function useGateMcp(baseUrl?: string) {
  const mcp = getSharedMcpClient(baseUrl);
  const pollingError = ref<string | null>(null);
  const error = computed(() => pollingError.value ?? mcp.error.value);
  const polling: GatePollingState = {
    root: null,
    handle: null,
  };
  const validatedGateCommands = new Set<string>();

  mcp.onDisconnect(() => {
    validatedGateCommands.clear();
  });

  async function ensureConnected() {
    if (mcp.connected.value) {
      return;
    }

    await mcp.connect();
  }

  function commandNames(result: GatewayIndexResult) {
    return new Set(
      (result.commands ?? [])
        .map((command) => command.name)
        .filter((name): name is string => typeof name === 'string' && name.length > 0),
    );
  }

  async function ensureGateGateway(command: string) {
    await ensureConnected();
    if (validatedGateCommands.has(command)) return;

    const index = unwrapToolData<GatewayIndexResult>(
      await mcp.callTool('GatewayIndex', { domain: GATE_DOMAIN }),
    );
    const available = commandNames(index);
    if (!available.has(command)) {
      throw new Error(
        `Gate tab requires GatewayIndex domain '${GATE_DOMAIN}' to expose ${command}.`,
      );
    }

    validatedGateCommands.add(command);
  }

  async function callGateTool<T>(command: string, payload: JsonObject = {}) {
    await ensureGateGateway(command);
    const result = await mcp.callTool('GatewayExecute', {
      domain: GATE_DOMAIN,
      command,
      parameters: payload,
    });
    return unwrapToolData<T>(result);
  }

  function gatePayload(payload: object): JsonObject {
    return Object.fromEntries(
      Object.entries(payload).filter((entry): entry is [string, JsonValue] => entry[1] !== undefined),
    );
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
    return callGateTool<GateSyncResult>('gate_Sync', gatePayload(args));
  }

  function createTask(args: GateCreateArgs): Promise<GateCreateResult> {
    return callGateTool<GateCreateResult>('gate_Create', gatePayload(args));
  }

  function normalizeTaskStages(task: GateTask): GateTask {
    const filtered = Array.isArray(task.stages) ? task.stages : [];
    return { ...task, stages: filtered };
  }

  async function status(args: GateStatusArgs): Promise<GateTask[]> {
    const tasks = await callGateTool<GateTask[]>('gate_Status', gatePayload(args));
    return Array.isArray(tasks) ? tasks.map(normalizeTaskStages) : tasks;
  }

  function readTaskHistory(taskId: string, root?: string): Promise<GateTaskHistoryEntry[]> {
    return callGateTool<GateTaskHistoryEntry[]>('gate_History', gatePayload({
      taskId,
      ...(root ? { root } : {}),
    }));
  }

  function readTaskBody(taskId: string, root?: string): Promise<GateTaskBody> {
    return callGateTool<GateTaskBody>('gate_TaskBody', gatePayload({
      taskId,
      ...(root ? { root } : {}),
    }));
  }

  function append(args: { root: string; taskId: string; agent: string; notes: string }) {
    return callGateTool<Record<string, unknown>>('gate_Append', gatePayload(args));
  }

  function pass(args: { root: string; taskId: string; agent: string; notes: string }) {
    return callGateTool<Record<string, unknown>>('gate_Pass', gatePayload(args));
  }

  function fail(args: { root: string; taskId: string; agent: string; notes: string }) {
    return callGateTool<Record<string, unknown>>('gate_Fail', gatePayload(args));
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
    createTask,
    sync,
    status,
    append,
    pass,
    fail,
    readTaskBody,
    readTaskHistory,
    startPolling,
    stopPolling,
  };
}
