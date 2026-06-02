import { computed, ref } from 'vue';
import { defineStore } from 'pinia';
import { executeGateway, readGatewayResource } from '../lib/mcp/gateway.ts';

const SCHEDULE_DOMAIN = 'maenifold.system';
const SCHEDULE_TOOL = 'Schedule';

export type RunStatus = 'success' | 'failed' | 'timeout' | 'skipped';

export interface RunRecord {
  status: RunStatus;
  timestamp: string | null;
  duration_seconds: number;
  error: string | null;
  mcp_action: string | null;
  mcp_parameters: unknown | null;
  mcp_result: unknown | null;
}

export interface ScheduledTask {
  id: string;
  name: string;
  schedule: string;
  schedule_human: string | null;
  agent: string;
  prompt: string;
  workdir: string | null;
  auto_grant_permissions: boolean;
  allowed_tools: string[];
  mcp_action: string | null;
  mcp_parameters: unknown | null;
  timeout_seconds: number;
  enabled: boolean;
  created_at: string;
  updated_at: string;
  last_run: RunRecord | null;
}

type ScheduleTaskInput = Partial<ScheduledTask> & { id: string };

interface RawScheduledTask extends Record<string, unknown> {
  id?: string;
  name?: string;
  description?: string;
  schedule?: string;
  schedule_human?: string | null;
  agent?: string;
  prompt?: string;
  workdir?: string | null;
  auto_grant_permissions?: boolean;
  allowed_tools?: unknown;
  mcp_action?: string | null;
  mcp_parameters?: unknown;
  timeout_seconds?: number;
  enabled?: boolean;
  created_at?: string;
  updated_at?: string;
  last_run?: unknown;
}

interface RawLogEntry extends Record<string, unknown> {
  fileName?: string;
  content?: string;
  status?: string;
  duration_seconds?: number;
  mcp_action?: string | null;
  mcp_parameters?: unknown;
  mcp_result?: unknown;
}

function toErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function asRunStatus(value: unknown): RunStatus {
  return value === 'failed' || value === 'timeout' || value === 'skipped' ? value : 'success';
}

function asStringOrNull(value: unknown): string | null {
  return typeof value === 'string' && value.length > 0 ? value : null;
}

function asStringArray(value: unknown): string[] {
  return Array.isArray(value) ? value.filter((item): item is string => typeof item === 'string') : [];
}

function parseLogTimestamp(fileName: unknown): string | null {
  if (typeof fileName !== 'string') {
    return null;
  }

  const compact = fileName.match(/^(\d{4})(\d{2})(\d{2})T(\d{2})(\d{2})(\d{2})Z/);
  if (compact) {
    const [, year, month, day, hour, minute, second] = compact;
    return `${year}-${month}-${day}T${hour}:${minute}:${second}Z`;
  }

  const iso = fileName.match(/^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z)/);
  return iso ? iso[1] : null;
}

function normalizeRunRecord(value: unknown): RunRecord | null {
  if (!value || typeof value !== 'object') {
    return null;
  }

  const record = value as Record<string, unknown>;
  return {
    status: asRunStatus(record.status),
    timestamp: asStringOrNull(record.timestamp),
    duration_seconds: typeof record.duration_seconds === 'number' ? record.duration_seconds : 0,
    error: asStringOrNull(record.error),
    mcp_action: asStringOrNull(record.mcp_action),
    mcp_parameters: record.mcp_parameters ?? null,
    mcp_result: record.mcp_result ?? null,
  };
}

function normalizeTask(value: RawScheduledTask): ScheduledTask {
  const id = typeof value.id === 'string' ? value.id : '';
  const name =
    typeof value.name === 'string'
      ? value.name
      : typeof value.description === 'string'
        ? value.description
        : id;

  return {
    id,
    name,
    schedule: typeof value.schedule === 'string' ? value.schedule : '',
    schedule_human: asStringOrNull(value.schedule_human),
    agent: typeof value.agent === 'string' ? value.agent : '',
    prompt: typeof value.prompt === 'string' ? value.prompt : '',
    workdir: asStringOrNull(value.workdir),
    auto_grant_permissions: value.auto_grant_permissions === true,
    allowed_tools: asStringArray(value.allowed_tools),
    mcp_action: asStringOrNull(value.mcp_action),
    mcp_parameters: value.mcp_parameters ?? null,
    timeout_seconds: typeof value.timeout_seconds === 'number' ? value.timeout_seconds : 300,
    enabled: value.enabled !== false,
    created_at: typeof value.created_at === 'string' ? value.created_at : '',
    updated_at: typeof value.updated_at === 'string' ? value.updated_at : '',
    last_run: normalizeRunRecord(value.last_run),
  };
}

function toMutationPayload(input: ScheduleTaskInput): Record<string, unknown> {
  const payload: Record<string, unknown> = { id: input.id };

  if (input.name !== undefined) payload.description = input.name;
  if (input.schedule !== undefined) payload.schedule = input.schedule;
  if (input.agent !== undefined) payload.agent = input.agent;
  if (input.prompt !== undefined) payload.prompt = input.prompt;
  if (input.workdir !== undefined) payload.workdir = input.workdir;
  if (input.auto_grant_permissions !== undefined) payload.auto_grant_permissions = input.auto_grant_permissions;
  if (input.allowed_tools !== undefined) payload.allowed_tools = input.allowed_tools;
  if (input.mcp_action !== undefined) payload.mcp_action = input.mcp_action;
  if (input.mcp_parameters !== undefined) payload.mcp_parameters = input.mcp_parameters;
  if (input.timeout_seconds !== undefined) payload.timeout_seconds = input.timeout_seconds;
  if (input.enabled !== undefined) payload.enabled = input.enabled;

  return payload;
}

function logEntryToRunRecord(entry: RawLogEntry): RunRecord {
  const status = asRunStatus(entry.status);
  const content = typeof entry.content === 'string' ? entry.content.trim() : '';

  return {
    status,
    timestamp: parseLogTimestamp(entry.fileName),
    duration_seconds: typeof entry.duration_seconds === 'number' ? entry.duration_seconds : 0,
    error: status === 'success' || content.length === 0 ? null : content,
    mcp_action: asStringOrNull(entry.mcp_action),
    mcp_parameters: entry.mcp_parameters ?? null,
    mcp_result: entry.mcp_result ?? null,
  };
}

export const useSchedulesStore = defineStore('schedules', () => {
  const tasks = ref<ScheduledTask[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const lastRefreshed = ref<Date | null>(null);
  const runningIds = ref<Set<string>>(new Set());
  const historyByTaskId = ref<Record<string, RunRecord[]>>({});

  const taskById = computed(() => new Map(tasks.value.map((task) => [task.id, task])));

  async function loadTasks(): Promise<void> {
    loading.value = true;
    try {
      const result = await executeGateway<RawScheduledTask[]>(SCHEDULE_DOMAIN, SCHEDULE_TOOL, {
        action: 'list',
      });
      tasks.value = Array.isArray(result) ? result.map(normalizeTask) : [];
      lastRefreshed.value = new Date();
      error.value = null;
    } catch (err) {
      error.value = toErrorMessage(err);
    } finally {
      loading.value = false;
    }
  }

  async function createTask(input: Partial<ScheduledTask> & { id: string; schedule: string; agent: string; prompt: string }): Promise<void> {
    await executeGateway(SCHEDULE_DOMAIN, SCHEDULE_TOOL, {
      action: 'create',
      ...toMutationPayload(input),
    });
    await loadTasks();
  }

  async function updateTask(input: Partial<ScheduledTask> & { id: string }): Promise<void> {
    await executeGateway(SCHEDULE_DOMAIN, SCHEDULE_TOOL, {
      action: 'update',
      ...toMutationPayload(input),
    });
    await loadTasks();
  }

  async function deleteTask(id: string): Promise<void> {
    await executeGateway(SCHEDULE_DOMAIN, SCHEDULE_TOOL, { action: 'delete', id });
    await loadTasks();
  }

  async function runTask(id: string): Promise<void> {
    runningIds.value = new Set(runningIds.value).add(id);
    try {
      await executeGateway(SCHEDULE_DOMAIN, SCHEDULE_TOOL, { action: 'run_now', id });
      runningIds.value = new Set([...runningIds.value].filter((value) => value !== id));
      error.value = null;
      await Promise.all([loadTasks(), loadHistory(id)]);
    } catch (err) {
      runningIds.value = new Set([...runningIds.value].filter((value) => value !== id));
      error.value = toErrorMessage(err);
    }
  }

  async function togglePaused(id: string): Promise<void> {
    const task = taskById.value.get(id);
    if (!task) {
      throw new Error(`Unknown schedule '${id}'`);
    }

    await updateTask({ id, enabled: !task.enabled });
  }

  async function loadHistory(id: string): Promise<void> {
    const result = await readGatewayResource<unknown[]>(`schedule://logs/${encodeURIComponent(id)}`);
    const records = Array.isArray(result)
      ? result.map((entry) => logEntryToRunRecord((entry ?? {}) as RawLogEntry))
      : [];
    historyByTaskId.value = {
      ...historyByTaskId.value,
      [id]: records,
    };
  }

  return {
    tasks,
    loading,
    error,
    lastRefreshed,
    runningIds,
    historyByTaskId,
    loadTasks,
    createTask,
    updateTask,
    deleteTask,
    runTask,
    togglePaused,
    loadHistory,
  };
});
