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

export function handleToolCallImpl(
  session: SessionState,
  update: ToolCallUpdateSnapshot,
  appUi: McpAppUiMetadata | null = null,
): void {
  const toolCallInfo = createToolCallInfo(update, appUi);
  const existing = session.toolCalls.get(update.toolCallId);

  if (existing) {
    applyToolCallSnapshot(existing, toolCallInfo);
    for (const msg of session.messages) {
      const messageToolCall = msg.toolCalls?.find((t) => t.toolCallId === update.toolCallId);
      if (messageToolCall) {
        applyToolCallSnapshot(messageToolCall, toolCallInfo);
        break;
      }
    }
    return;
  }

  // Add tool call to the current assistant message, synthesizing a
  // tool-only assistant turn when the agent emits tool activity before
  // any assistant text.
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

  if (!currentAssistantMsg.toolCalls) {
    currentAssistantMsg.toolCalls = [];
  }
  currentAssistantMsg.toolCalls.push({ ...toolCallInfo });

  // Also keep in session's tool call map for updates
  session.toolCalls.set(update.toolCallId, { ...toolCallInfo });
}

export function handleToolCallUpdateImpl(
  session: SessionState,
  update: ToolCallUpdateSnapshot,
  appUi: McpAppUiMetadata | null = null,
): void {
  const existing = session.toolCalls.get(update.toolCallId);
  if (!existing) return;

  if (update.status) existing.status = update.status;
  if (update.title) existing.title = update.title;
  const nextContent = normalizeToolCallContent(update.content);
  if (nextContent?.length) {
    existing.content = [...(existing.content ?? []), ...nextContent];
  }
  if (update.locations) {
    existing.locations = normalizeToolCallLocations(update.locations);
  }
  if ('rawOutput' in update) {
    existing.rawOutput = normalizeToolCallPayload(update.rawOutput);
  }
  if (isRecord(update._meta)) {
    existing._meta = update._meta;
  }
  if (!existing.appUi && appUi) {
    existing.appUi = cloneAppUiMetadata(appUi);
  }

  // Also update in the message's toolCalls array
  for (const msg of session.messages) {
    if (!msg.toolCalls) continue;
    const tc = msg.toolCalls.find((t) => t.toolCallId === update.toolCallId);
    if (tc) {
      if (update.status) tc.status = update.status;
      if (update.title) tc.title = update.title;
      if (nextContent?.length) {
        tc.content = [...(tc.content ?? []), ...nextContent];
      }
      if (update.locations) {
        tc.locations = normalizeToolCallLocations(update.locations);
      }
      if ('rawOutput' in update) {
        tc.rawOutput = normalizeToolCallPayload(update.rawOutput);
      }
      if (isRecord(update._meta)) {
        tc._meta = update._meta;
      }
      if (!tc.appUi && appUi) {
        tc.appUi = cloneAppUiMetadata(appUi);
      }
    }
  }
}
