export type JsonPrimitive = string | number | boolean | null;
export type JsonValue = JsonPrimitive | JsonObject | JsonValue[];
export type JsonObject = { [key: string]: JsonValue };

export type PipelineNodeTone =
  | "pending"
  | "ready"
  | "active"
  | "done"
  | "failed";

export type PipelineLogLevel = "info" | "warn" | "error";

export type GatewayToolResult = {
  content?: Array<{
    type?: string;
    text?: string;
  }>;
  isError?: boolean;
};

export type PipelineEdge = {
  source: string;
  target: string;
  type?: string | null;
  mode?: JsonValue;
  mapping?: JsonValue;
};

export type PipelineNode = {
  nodeId: string;
  sessionId: string;
  status: string;
  attempts: number;
  output: JsonValue;
  type: string;
  displayLabel?: string;
  startedAt?: string;
  completedAt?: string;
  retryAfter?: string;
  retryCount?: number;
  inputTokens?: number;
  outputTokens?: number;
  inputArtifact?: JsonValue;
  outputArtifact?: JsonValue;
  failureReason?: string;
  acpSessionId?: string;
  acpProcessAlive?: boolean;
  thinkingSessionId?: string;
  toolProvenance?: JsonValue;
  provenance?: JsonValue;
  assembled_prompt?: string | null;
  workflow_steps?: JsonValue;
  thinking_trace?: JsonValue;
  workflowId?: string;
  prompt?: string;
  toolName?: string;
  toolArgs?: JsonValue;
  agent?: string | null;
  acpClient?: string | null;
  topic?: string;
  model?: string | null;
  thoughtLevel?: string | null;
  allowedTools?: string[] | null;
  maxTurns?: number | null;
  cwd?: string | null;
  timeout?: number | null;
  agentConfig?: JsonValue;
  inputSchema?: JsonValue;
  outputSchema?: JsonValue;
};

export type PipelineRun = {
  nodeId: string;
  startedAt?: string | null;
  completedAt?: string | null;
  status?: string | null;
  outcome?: string | null;
};

export type PipelineListEntry = {
  pipelineId: string;
  status: string;
  nodeCount: number;
  completedCount: number;
  runs: PipelineRun[];
  nodes: PipelineNode[];
  name?: string;
  description?: string;
  degraded?: JsonValue;
};

export type PipelineView = {
  pipelineId: string;
  status: string;
  final: boolean;
  running: boolean;
  nodes: PipelineNode[];
  edges: PipelineEdge[];
  ready: string[];
  active: string[];
  completed: string[];
  mermaid?: string;
  name?: string;
  description?: string;
  inputSchema?: JsonValue;
  degraded?: JsonValue;
};

export type PipelineTask = {
  taskId: string;
  status: string;
  pipelineStatus: string;
};

export type PipelineTaskList = {
  tasks: PipelineTask[];
  nextCursor?: string | null;
};

export type PipelineListResponse = {
  pipelines: PipelineListEntry[];
};

export type RunMetadata = {
  activeNodes: number;
  completedNodes: number;
  failedNodes: number;
  elapsedMs: number;
  hasActiveRun: boolean;
};

export type LogEntry = {
  id: string;
  timestamp: string | null;
  level: PipelineLogLevel;
  message: string;
  source: string;
};

type GatewayEnvelope<T> = {
  success?: boolean;
  data?: T;
  error?: string;
};

type UnknownRecord = Record<string, unknown>;

const STATUS_TO_TONE: Record<string, PipelineNodeTone> = {
  pending: "pending",
  ready: "ready",
  active: "active",
  retrying: "active",
  running: "active",
  completed: "done",
  skipped: "done",
  failed: "failed",
  stuck: "failed",
  abandoned: "failed",
  cancelled: "failed",
};

export function parseGatewayToolResult<T>(result: unknown): T {
  const toolResult = result as GatewayToolResult;
  const text = toolResult.content?.find((entry) => entry.type === "text")?.text;

  if (!text) {
    throw new Error("Gateway response did not include text content");
  }

  if (toolResult.isError || text.startsWith("Error invoking")) {
    throw new Error(text);
  }

  const parsed = JSON.parse(text) as GatewayEnvelope<T> | T;

  if (isGatewayEnvelope<T>(parsed)) {
    if (parsed.success === false) {
      throw new Error(parsed.error ?? "Gateway request failed");
    }

    return (parsed.data ?? (parsed as T)) as T;
  }

  return parsed as T;
}

function isGatewayEnvelope<T>(value: GatewayEnvelope<T> | T): value is GatewayEnvelope<T> {
  return (
    typeof value === "object" &&
    value !== null &&
    ("success" in value || "data" in value || "error" in value)
  );
}

export function selectPrimaryPipelineId(
  tasks: PipelineTaskList | null | undefined,
  list: PipelineListResponse | null | undefined,
): string | null {
  const activeTask = tasks?.tasks.find((task) => task.status === "working");

  if (activeTask) {
    return activeTask.taskId;
  }

  const activePipeline = list?.pipelines.find((pipeline) =>
    ["active", "running"].includes(pipeline.status),
  );

  return activePipeline?.pipelineId ?? null;
}

export function getNodeTone(status: string): PipelineNodeTone {
  return STATUS_TO_TONE[status.toLowerCase()] ?? "pending";
}

export function countFailedNodes(nodes: PipelineNode[]): number {
  return nodes.filter((node) => getNodeTone(node.status) === "failed").length;
}

export function formatDuration(ms: number): string {
  if (!Number.isFinite(ms) || ms <= 0) {
    return "00:00";
  }

  const totalSeconds = Math.floor(ms / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  if (hours > 0) {
    return [hours, minutes, seconds].map((part) => String(part).padStart(2, "0")).join(":");
  }

  return [minutes, seconds].map((part) => String(part).padStart(2, "0")).join(":");
}

export function computeRunMetadata(
  pipeline: PipelineView | null,
  now = Date.now(),
): RunMetadata {
  if (!pipeline) {
    return {
      activeNodes: 0,
      completedNodes: 0,
      failedNodes: 0,
      elapsedMs: 0,
      hasActiveRun: false,
    };
  }

  return {
    activeNodes: pipeline.nodes.filter((node) => getNodeTone(node.status) === "active").length,
    completedNodes: pipeline.nodes.filter((node) => getNodeTone(node.status) === "done").length,
    failedNodes: countFailedNodes(pipeline.nodes),
    elapsedMs: getPipelineElapsedMs(pipeline, now),
    hasActiveRun: pipeline.running || pipeline.status === "active",
  };
}

export function getPipelineElapsedMs(
  pipeline: Pick<PipelineView, "pipelineId" | "nodes" | "running">,
  now = Date.now(),
): number {
  const timestamps = pipeline.nodes
    .flatMap((node) => [node.startedAt, node.completedAt])
    .filter((value): value is string => Boolean(value))
    .map((value) => Date.parse(value))
    .filter((value) => Number.isFinite(value));

  const seedTimestamp = parsePipelineTimestamp(pipeline.pipelineId);
  const startAt = timestamps.length > 0 ? Math.min(...timestamps) : seedTimestamp;

  if (!startAt) {
    return 0;
  }

  if (pipeline.running) {
    return Math.max(0, now - startAt);
  }

  const completedAt = timestamps.length > 0 ? Math.max(...timestamps) : now;
  return Math.max(0, completedAt - startAt);
}

export function parsePipelineTimestamp(pipelineId: string): number | null {
  const match = pipelineId.match(/-(\d{13,})$/);
  return match ? Number(match[1]) : null;
}

export function deriveNodeDuration(node: PipelineNode, now = Date.now()): string {
  const startedAt = node.startedAt ? Date.parse(node.startedAt) : null;
  const completedAt = node.completedAt ? Date.parse(node.completedAt) : null;

  if (!startedAt || !Number.isFinite(startedAt)) {
    return "—";
  }

  const end = completedAt && Number.isFinite(completedAt) ? completedAt : now;
  return formatDuration(Math.max(0, end - startedAt));
}

export function getNodeLabel(node: PipelineNode): string {
  return node.displayLabel?.trim() || node.nodeId;
}

export function collectNodeLogs(node: PipelineNode): LogEntry[] {
  const entries: LogEntry[] = [];

  if (node.startedAt) {
    entries.push({
      id: `${node.nodeId}-started`,
      timestamp: node.startedAt,
      level: "info",
      source: "lifecycle",
      message: `Node started (${node.sessionId})`,
    });
  }

  if (node.completedAt) {
    entries.push({
      id: `${node.nodeId}-completed`,
      timestamp: node.completedAt,
      level: getNodeTone(node.status) === "failed" ? "error" : "info",
      source: "lifecycle",
      message: `Node finished with status ${node.status}`,
    });
  }

  if (node.failureReason) {
    entries.push({
      id: `${node.nodeId}-failure`,
      timestamp: node.completedAt ?? node.startedAt ?? null,
      level: "error",
      source: "failure",
      message: node.failureReason,
    });
  }

  entries.push(...collectStructuredEntries(node.nodeId, "workflow", node.workflow_steps));
  entries.push(...collectStructuredEntries(node.nodeId, "thinking", node.thinking_trace));

  return entries.sort((left, right) => {
    const leftTime = left.timestamp ? Date.parse(left.timestamp) : 0;
    const rightTime = right.timestamp ? Date.parse(right.timestamp) : 0;
    return leftTime - rightTime;
  });
}

function collectStructuredEntries(
  nodeId: string,
  source: string,
  value: JsonValue | undefined,
): LogEntry[] {
  if (!value) {
    return [];
  }

  if (!Array.isArray(value)) {
    return [
      {
        id: `${nodeId}-${source}-0`,
        timestamp: null,
        level: "info",
        source,
        message: stringifyJson(value),
      },
    ];
  }

  return value.map((entry, index) => mapStructuredEntry(nodeId, source, entry, index));
}

function mapStructuredEntry(
  nodeId: string,
  source: string,
  value: JsonValue,
  index: number,
): LogEntry {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return {
      id: `${nodeId}-${source}-${index}`,
      timestamp: null,
      level: "info",
      source,
      message: stringifyJson(value),
    };
  }

  const record = value as UnknownRecord;
  const timestamp = pickString(record, [
    "timestamp",
    "time",
    "startedAt",
    "completedAt",
    "createdAt",
  ]);
  const message =
    pickString(record, ["message", "summary", "text", "thought", "step", "title"]) ??
    stringifyJson(value);
  const rawLevel = pickString(record, ["level", "severity", "status"]);

  return {
    id: `${nodeId}-${source}-${index}`,
    timestamp,
    level: toLogLevel(rawLevel),
    source,
    message,
  };
}

function toLogLevel(value: string | null): PipelineLogLevel {
  if (!value) {
    return "info";
  }

  const normalized = value.toLowerCase();

  if (normalized.includes("fail") || normalized.includes("error") || normalized.includes("stuck")) {
    return "error";
  }

  if (normalized.includes("warn") || normalized.includes("retry")) {
    return "warn";
  }

  return "info";
}

function pickString(record: UnknownRecord, keys: string[]): string | null {
  for (const key of keys) {
    const value = record[key];
    if (typeof value === "string" && value.trim()) {
      return value;
    }
  }

  return null;
}

export function stringifyJson(value: JsonValue | undefined): string {
  if (typeof value === "string") {
    return value;
  }

  if (value === undefined) {
    return "";
  }

  return JSON.stringify(value, null, 2);
}

export function filterJsonValue(
  value: JsonValue,
  query: string,
): JsonValue | null {
  const normalizedQuery = query.trim().toLowerCase();

  if (!normalizedQuery) {
    return value;
  }

  if (typeof value !== "object" || value === null) {
    return stringifyJson(value).toLowerCase().includes(normalizedQuery) ? value : null;
  }

  if (Array.isArray(value)) {
    const filtered = value
      .map((item) => filterJsonValue(item, normalizedQuery))
      .filter((item): item is JsonValue => item !== null);
    return filtered.length > 0 ? filtered : null;
  }

  const result: JsonObject = {};

  for (const [key, child] of Object.entries(value)) {
    const filteredChild = filterJsonValue(child, normalizedQuery);
    if (key.toLowerCase().includes(normalizedQuery) || filteredChild !== null) {
      result[key] = filteredChild ?? child;
    }
  }

  return Object.keys(result).length > 0 ? result : null;
}

export function pickWorktreePath(
  pipeline: PipelineView | null,
  node: PipelineNode | null,
): string | null {
  const direct = node?.cwd?.trim();
  if (direct) {
    return direct;
  }

  const fallback = pipeline?.nodes.find((entry) => entry.cwd?.trim())?.cwd?.trim();
  return fallback || null;
}
