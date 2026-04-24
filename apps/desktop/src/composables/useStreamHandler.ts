// Stream handler — content block utilities, tool call normalization,
// and streaming session-update case processing.
import type {
  ContentBlock,
  McpAppUiMetadata,
  SessionState,
  ToolCallInfo,
} from '../lib/types';
import type {
  ToolCallContent as SdkToolCallContent,
  ToolCallLocation as SdkToolCallLocation,
} from '@agentclientprotocol/sdk';
import { isRecord } from './session-utils';

// ── Tool-call snapshot type ─────────────────────────────────────────
type ToolCallUpdateSnapshot = {
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

// ── Pure normalisation helpers ──────────────────────────────────────

function normalizeToolCallLocations(
  locations: Array<SdkToolCallLocation> | null | undefined,
): ToolCallInfo['locations'] | undefined {
  return locations?.map(({ path, line }) =>
    line == null ? { path } : { path, line },
  );
}

function normalizeToolCallContent(
  content: Array<SdkToolCallContent> | null | undefined,
): ToolCallInfo['content'] | undefined {
  return content?.map((item) => ({ ...item }));
}

function normalizeToolCallPayload(
  payload: unknown,
): Record<string, unknown> | undefined {
  return isRecord(payload) ? { ...payload } : undefined;
}

function cloneAppUiMetadata(appUi: McpAppUiMetadata | null | undefined): McpAppUiMetadata | null {
  if (!appUi) {
    return null;
  }

  return {
    resourceUri: appUi.resourceUri,
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

// ── Content-block helpers ───────────────────────────────────────────

function warnNonTextContentBlock(block: ContentBlock): void {
  if (block.type !== 'text') {
  }
}

export function cloneContentBlock(block: ContentBlock): ContentBlock {
  return {
    ...block,
    ...(block.type === 'resource' && block.resource
      ? { resource: { ...block.resource } }
      : {}),
    ...(block.annotations ? { annotations: { ...block.annotations } } : {}),
    ...(isRecord(block._meta) ? { _meta: { ...block._meta } } : {}),
  };
}

function appendMessageBlock(
  message: SessionState['messages'][number],
  block: ContentBlock,
): void {
  if (block.type === 'text') {
    // Accumulate into the last text block instead of creating a new one per chunk
    const existing = message.blocks;
    const lastBlock = existing?.[existing.length - 1];
    if (lastBlock && lastBlock.type === 'text') {
      lastBlock.text += block.text;
      message.blocks = [...existing!];
    } else {
      message.blocks = [...(existing ?? []), cloneContentBlock(block)];
    }
    message.content += block.text;
  } else {
    message.blocks = [...(message.blocks ?? []), cloneContentBlock(block)];
    warnNonTextContentBlock(block);
  }
}

function appendThoughtBlock(
  message: SessionState['messages'][number],
  block: ContentBlock,
): void {
  if (block.type === 'text') {
    message.thought = (message.thought || '') + block.text;
    return;
  }

  warnNonTextContentBlock(block);
  message.blocks = [...(message.blocks ?? []), cloneContentBlock(block)];
}

// ── Streaming session-update case handlers ──────────────────────────

export function handleUserMessageChunk(
  session: SessionState,
  update: { content: ContentBlock },
): void {
  const lastUserMsg = session.messages[session.messages.length - 1];
  if (lastUserMsg && lastUserMsg.role === 'user') {
    appendMessageBlock(lastUserMsg, update.content);
  } else {
    warnNonTextContentBlock(update.content);
    session.messages.push({
      id: crypto.randomUUID(),
      role: 'user',
      content: update.content.type === 'text' ? update.content.text : '',
      blocks: [cloneContentBlock(update.content)],
      timestamp: Date.now(),
    });
  }
}

export function handleAgentMessageChunk(
  session: SessionState,
  update: { content: ContentBlock },
): void {
  const lastMsg = session.messages[session.messages.length - 1];
  if (lastMsg && lastMsg.role === 'assistant') {
    appendMessageBlock(lastMsg, update.content);
  } else {
    warnNonTextContentBlock(update.content);
    session.messages.push({
      id: crypto.randomUUID(),
      role: 'assistant',
      content: update.content.type === 'text' ? update.content.text : '',
      blocks: [cloneContentBlock(update.content)],
      timestamp: Date.now(),
      toolCalls: [],
    });
  }
}

export function handleAgentThoughtChunk(
  session: SessionState,
  update: { content: ContentBlock },
): void {
  const lastAssistantMsg = session.messages[session.messages.length - 1];
  if (lastAssistantMsg && lastAssistantMsg.role === 'assistant') {
    appendThoughtBlock(lastAssistantMsg, update.content);
  } else {
    warnNonTextContentBlock(update.content);
    session.messages.push({
      id: crypto.randomUUID(),
      role: 'assistant',
      content: '',
      thought: update.content.type === 'text' ? update.content.text : '',
      blocks: update.content.type === 'text' ? undefined : [cloneContentBlock(update.content)],
      timestamp: Date.now(),
      toolCalls: [],
    });
  }
}

export function handleToolCall(
  session: SessionState,
  update: ToolCallUpdateSnapshot,
  appUi: McpAppUiMetadata | null = null,
): void {
  const toolCallInfo = createToolCallInfo(update, appUi);

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

export function handleToolCallUpdate(
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
