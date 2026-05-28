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
  return extractGatewayPayload(result) as T;
}

export function normalizePipelineView(raw: unknown): PipelineView {
  const value = asRecord(extractGatewayPayload(raw));

  return {
    pipelineId: readString(value.pipelineId),
    status: readString(value.status, "pending"),
    final: readBoolean(value.final),
    running: readBoolean(value.running),
    nodes: readArray(value.nodes).map(normalizePipelineNode),
    edges: readArray(value.edges).map(normalizePipelineEdge),
    ready: readArray(value.ready).map((entry) => readString(entry)),
    active: readArray(value.active).map((entry) => readString(entry)),
    completed: readArray(value.completed).map((entry) => readString(entry)),
    mermaid: readOptionalString(value.mermaid),
    name: readOptionalString(value.name),
    description: readOptionalString(value.description),
    inputSchema: normalizeOptionalJsonValue(value.inputSchema),
    degraded: normalizeOptionalJsonValue(value.degraded),
  };
}

export function normalizePipelineList(raw: unknown): PipelineListResponse {
  const value = asRecord(extractGatewayPayload(raw));

  return {
    pipelines: readArray(value.pipelines).map((entry) => {
      const record = asRecord(entry);

      return {
        pipelineId: readString(record.pipelineId),
        status: readString(record.status, "pending"),
        nodeCount: readNumber(record.nodeCount),
        completedCount: readNumber(record.completedCount),
        runs: readArray(record.runs).map((run) => {
          const runRecord = asRecord(run);
          return {
            nodeId: readString(runRecord.nodeId),
            startedAt: readOptionalString(runRecord.startedAt),
            completedAt: readOptionalString(runRecord.completedAt),
            status: readOptionalString(runRecord.status),
            outcome: readOptionalString(runRecord.outcome),
          };
        }),
        nodes: readArray(record.nodes).map(normalizePipelineNode),
        name: readOptionalString(record.name),
        description: readOptionalString(record.description),
        degraded: normalizeOptionalJsonValue(record.degraded),
      };
    }),
  };
}

export function normalizeTaskList(raw: unknown): PipelineTaskList {
  const value = asRecord(extractGatewayPayload(raw));

  return {
    tasks: readArray(value.tasks).map((entry) => {
      const record = asRecord(entry);

      return {
        taskId: readString(record.taskId),
        status: readString(record.status, "working"),
        pipelineStatus: readString(record.pipelineStatus, "pending"),
      };
    }),
    nextCursor: readOptionalString(value.nextCursor),
  };
}

function extractGatewayPayload(result: unknown): unknown {
  const toolResult = result as GatewayToolResult;
  const text = toolResult.content?.find((entry) => entry.type === "text")?.text;

  if (!text) {
    throw new Error("Gateway response did not include text content");
  }

  if (toolResult.isError || text.startsWith("Error invoking")) {
    throw new Error(text);
  }

  const parsed = JSON.parse(text) as GatewayEnvelope<unknown> | unknown;

  if (isGatewayEnvelope(parsed)) {
    if (parsed.success === false) {
      throw new Error(parsed.error ?? "Gateway request failed");
    }

    return parsed.data ?? parsed;
  }

  return parsed;
}

function isGatewayEnvelope(value: unknown): value is GatewayEnvelope<unknown> {
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

function normalizePipelineNode(value: unknown): PipelineNode {
  const record = asRecord(value);

  return {
    nodeId: readString(record.nodeId),
    sessionId: readString(record.sessionId),
    status: readString(record.status, "pending"),
    attempts: readNumber(record.attempts),
    output: normalizeJsonValue(record.output),
    type: readString(record.type),
    displayLabel: readOptionalString(record.displayLabel),
    startedAt: readOptionalString(record.startedAt),
    completedAt: readOptionalString(record.completedAt),
    retryAfter: readOptionalString(record.retryAfter),
    retryCount: readOptionalNumber(record.retryCount),
    inputTokens: readOptionalNumber(record.inputTokens),
    outputTokens: readOptionalNumber(record.outputTokens),
    inputArtifact: normalizeOptionalJsonValue(record.inputArtifact),
    outputArtifact: normalizeOptionalJsonValue(record.outputArtifact),
    failureReason: readOptionalString(record.failureReason),
    acpSessionId: readOptionalString(record.acpSessionId),
    acpProcessAlive: readOptionalBoolean(record.acpProcessAlive),
    thinkingSessionId: readOptionalString(record.thinkingSessionId),
    toolProvenance: normalizeOptionalJsonValue(record.toolProvenance),
    provenance: normalizeOptionalJsonValue(record.provenance),
    assembled_prompt: readOptionalString(record.assembled_prompt),
    workflow_steps: normalizeOptionalJsonValue(record.workflow_steps),
    thinking_trace: normalizeOptionalJsonValue(record.thinking_trace),
    workflowId: readOptionalString(record.workflowId),
    prompt: readOptionalString(record.prompt),
    toolName: readOptionalString(record.toolName),
    toolArgs: normalizeOptionalJsonValue(record.toolArgs),
    agent: readOptionalString(record.agent),
    acpClient: readOptionalString(record.acpClient),
    topic: readOptionalString(record.topic),
    model: readOptionalString(record.model),
    thoughtLevel: readOptionalString(record.thoughtLevel),
    allowedTools: readOptionalStringArray(record.allowedTools),
    maxTurns: readOptionalNumber(record.maxTurns),
    cwd: readOptionalString(record.cwd),
    timeout: readOptionalNumber(record.timeout),
    agentConfig: normalizeOptionalJsonValue(record.agentConfig),
    inputSchema: normalizeOptionalJsonValue(record.inputSchema),
    outputSchema: normalizeOptionalJsonValue(record.outputSchema),
  };
}

function normalizePipelineEdge(value: unknown): PipelineEdge {
  const record = asRecord(value);

  return {
    source: readString(record.source),
    target: readString(record.target),
    type: readOptionalString(record.type),
    mode: normalizeOptionalJsonValue(record.mode),
    mapping: normalizeOptionalJsonValue(record.mapping),
  };
}

function asRecord(value: unknown): UnknownRecord {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? (value as UnknownRecord)
    : {};
}

function readArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : [];
}

function readString(value: unknown, fallback = ""): string {
  return typeof value === "string" ? value : fallback;
}

function readOptionalString(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

function readBoolean(value: unknown): boolean {
  return typeof value === "boolean" ? value : false;
}

function readOptionalBoolean(value: unknown): boolean | undefined {
  return typeof value === "boolean" ? value : undefined;
}

function readNumber(value: unknown): number {
  return typeof value === "number" && Number.isFinite(value) ? value : 0;
}

function readOptionalNumber(value: unknown): number | undefined {
  return typeof value === "number" && Number.isFinite(value) ? value : undefined;
}

function readOptionalStringArray(value: unknown): string[] | null | undefined {
  if (value == null) {
    return value as null | undefined;
  }

  return Array.isArray(value)
    ? value.filter((entry): entry is string => typeof entry === "string")
    : undefined;
}

function normalizeOptionalJsonValue(value: unknown): JsonValue | undefined {
  return value === undefined ? undefined : normalizeJsonValue(value);
}

function normalizeJsonValue(value: unknown): JsonValue {
  if (
    value === null ||
    typeof value === "string" ||
    (typeof value === "number" && Number.isFinite(value)) ||
    typeof value === "boolean"
  ) {
    return value;
  }

  if (Array.isArray(value)) {
    return value.map((entry) => normalizeJsonValue(entry));
  }

  if (typeof value === "object" && value !== null) {
    return Object.fromEntries(
      Object.entries(value).map(([key, entry]) => [key, normalizeJsonValue(entry)]),
    );
  }

  return null;
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
