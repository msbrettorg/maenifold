import type { ContentBlock, McpAppUiMetadata, SessionState } from '../lib/types';
import type { ToolCallUpdateSnapshot } from './useStreamHandler-tool-calls';
import {
  cloneContentBlockImpl,
  handleAgentMessageChunkImpl,
  handleAgentThoughtChunkImpl,
  handleUserMessageChunkImpl,
} from './useStreamHandler-content';
import {
  handleToolCallImpl,
  handleToolCallUpdateImpl,
} from './useStreamHandler-tool-calls';

export function cloneContentBlock(block: ContentBlock): ContentBlock {
  return cloneContentBlockImpl(block);
}

export function handleUserMessageChunk(
  session: SessionState,
  update: { content: ContentBlock },
): void {
  handleUserMessageChunkImpl(session, update);
}

export function handleAgentMessageChunk(
  session: SessionState,
  update: { content: ContentBlock },
): void {
  handleAgentMessageChunkImpl(session, update);
}

export function handleAgentThoughtChunk(
  session: SessionState,
  update: { content: ContentBlock },
): void {
  handleAgentThoughtChunkImpl(session, update);
}

export function handleToolCall(
  session: SessionState,
  update: ToolCallUpdateSnapshot,
  appUi: McpAppUiMetadata | null = null,
): void {
  handleToolCallImpl(session, update, appUi);
}

export function handleToolCallUpdate(
  session: SessionState,
  update: ToolCallUpdateSnapshot,
  appUi: McpAppUiMetadata | null = null,
): void {
  handleToolCallUpdateImpl(session, update, appUi);
}
