import type {
  McpAppUiMetadata,
  SessionState,
  ToolCallInfo,
} from '../lib/types';
import type {
  ToolCallContent as SdkToolCallContent,
  ToolCallLocation as SdkToolCallLocation,
} from '@agentclientprotocol/sdk';
import { isRecord } from './session-utils';

export type ToolCallUpdateSnapshot = {
  toolCallId: string;
  title?: string | null;
  kind?: string | null;
  status?: ToolCallInfo['status'] | null;
  locations?: Array<SdkToolCallLocation> | null;
  content?: Array<SdkToolCallContent> | null;
  rawInput?: unknown;
  rawOutput?: unknown;
  _meta?: Record<string, unknown> | null;
};

function normalizeToolCallLocations(
  locations: Array<SdkToolCallLocation> | null | undefined,
): ToolCallInfo['locations'] | undefined {
  return locations?.map(({ path, line }) => (line == null ? { path } : { path, line }));
}

function normalizeToolCallContent(
  content: Array<SdkToolCallContent> | null | undefined,
): ToolCallInfo['content'] | undefined {
  return content?.map((item) => ({ ...item }));
}

function normalizeToolCallPayload(payload: unknown): Record<string, unknown> | undefined {
  return isRecord(payload) ? { ...payload } : undefined;
}

function cloneAppUiMetadata(appUi: McpAppUiMetadata | null | undefined): McpAppUiMetadata | null {
  if (!appUi) return null;
  return {
    resourceUri: appUi.resourceUri,
    originatingServer: appUi.originatingServer,
    csp: appUi.csp,
    permissions: appUi.permissions ? [...appUi.permissions] : undefined,
  };
}

function createToolCallInfo(
  update: ToolCallUpdateSnapshot,
  appUi: McpAppUiMetadata | null = null,
): ToolCallInfo {
  return {
    toolCallId: update.toolCallId,
    title: update.title ?? '',
    kind: update.kind ?? 'other',
    status: update.status ?? 'pending',
    appUi: cloneAppUiMetadata(appUi),
    locations: normalizeToolCallLocations(update.locations),
    content: normalizeToolCallContent(update.content),
    rawInput: normalizeToolCallPayload(update.rawInput),
    rawOutput: normalizeToolCallPayload(update.rawOutput),
    _meta: isRecord(update._meta) ? update._meta : null,
  };
}

function applyToolCallSnapshot(target: ToolCallInfo, source: ToolCallInfo, appendContent = false): void {
  target.title = source.title;
  target.kind = source.kind;
  target.status = source.status;
  if (source.locations) target.locations = source.locations;
  if (source.content?.length) {
    target.content = appendContent ? [...(target.content ?? []), ...source.content] : [...source.content];
  }
  if (source.rawInput) target.rawInput = source.rawInput;
  if (source.rawOutput) target.rawOutput = source.rawOutput;
  if (source._meta) target._meta = source._meta;
  if (!target.appUi && source.appUi) target.appUi = cloneAppUiMetadata(source.appUi);
}

function forEachToolCall(
  session: SessionState,
  toolCallId: string,
  apply: (toolCall: ToolCallInfo) => void,
): void {
  for (const message of session.messages) {
    const toolCall = message.toolCalls?.find((entry) => entry.toolCallId === toolCallId);
    if (toolCall) apply(toolCall);
  }
}

export function handleToolCallImpl(
  session: SessionState,
  update: ToolCallUpdateSnapshot,
  appUi: McpAppUiMetadata | null = null,
): void {
  const toolCallInfo = createToolCallInfo(update, appUi);
  const existing = session.toolCalls.get(update.toolCallId);
  if (existing) {
    applyToolCallSnapshot(existing, toolCallInfo);
    forEachToolCall(session, update.toolCallId, (toolCall) => applyToolCallSnapshot(toolCall, toolCallInfo));
    return;
  }

  let currentAssistantMsg = session.messages[session.messages.length - 1];
  if (!currentAssistantMsg || currentAssistantMsg.role !== 'assistant') {
    session.messages.push({
      id: crypto.randomUUID(),
      role: 'assistant',
      content: '',
      timestamp: Date.now(),
      toolCalls: [],
    });
    currentAssistantMsg = session.messages[session.messages.length - 1];
  }

  currentAssistantMsg.toolCalls ??= [];
  currentAssistantMsg.toolCalls.push({ ...toolCallInfo });
  session.toolCalls.set(update.toolCallId, { ...toolCallInfo });
}

export function handleToolCallUpdateImpl(
  session: SessionState,
  update: ToolCallUpdateSnapshot,
  appUi: McpAppUiMetadata | null = null,
): void {
  const existing = session.toolCalls.get(update.toolCallId);
  if (!existing) return;

  const snapshot = createToolCallInfo(
    {
      ...update,
      title: update.title ?? existing.title,
      kind: update.kind ?? existing.kind,
      status: update.status ?? existing.status,
    },
    appUi ?? existing.appUi ?? null,
  );

  applyToolCallSnapshot(existing, snapshot, true);
  forEachToolCall(session, update.toolCallId, (toolCall) => applyToolCallSnapshot(toolCall, snapshot, true));
}
