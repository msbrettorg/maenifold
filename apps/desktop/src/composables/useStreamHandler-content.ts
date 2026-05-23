import type { ContentBlock, SessionState } from '../lib/types';
import { isRecord } from './session-utils';

function warnNonTextContentBlock(block: ContentBlock): void {
  if (block.type !== 'text') {
  }
}

export function cloneContentBlockImpl(block: ContentBlock): ContentBlock {
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
    const existing = message.blocks;
    const lastBlock = existing?.[existing.length - 1];
    if (lastBlock && lastBlock.type === 'text') {
      lastBlock.text += block.text;
      message.blocks = [...existing];
    } else {
      message.blocks = [...(existing ?? []), cloneContentBlockImpl(block)];
    }
    message.content += block.text;
    return;
  }

  message.blocks = [...(message.blocks ?? []), cloneContentBlockImpl(block)];
  warnNonTextContentBlock(block);
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
  message.blocks = [...(message.blocks ?? []), cloneContentBlockImpl(block)];
}

export function handleUserMessageChunkImpl(
  session: SessionState,
  update: { content: ContentBlock },
): void {
  const lastUserMsg = session.messages[session.messages.length - 1];
  if (lastUserMsg && lastUserMsg.role === 'user') {
    appendMessageBlock(lastUserMsg, update.content);
    return;
  }

  warnNonTextContentBlock(update.content);
  session.messages.push({
    id: crypto.randomUUID(),
    role: 'user',
    content: update.content.type === 'text' ? update.content.text : '',
    blocks: [cloneContentBlockImpl(update.content)],
    timestamp: Date.now(),
  });
}

export function handleAgentMessageChunkImpl(
  session: SessionState,
  update: { content: ContentBlock },
): void {
  const lastMsg = session.messages[session.messages.length - 1];
  if (lastMsg && lastMsg.role === 'assistant') {
    appendMessageBlock(lastMsg, update.content);
    return;
  }

  warnNonTextContentBlock(update.content);
  session.messages.push({
    id: crypto.randomUUID(),
    role: 'assistant',
    content: update.content.type === 'text' ? update.content.text : '',
    blocks: [cloneContentBlockImpl(update.content)],
    timestamp: Date.now(),
    toolCalls: [],
  });
}

export function handleAgentThoughtChunkImpl(
  session: SessionState,
  update: { content: ContentBlock },
): void {
  const lastAssistantMsg = session.messages[session.messages.length - 1];
  if (lastAssistantMsg && lastAssistantMsg.role === 'assistant') {
    appendThoughtBlock(lastAssistantMsg, update.content);
    return;
  }

  warnNonTextContentBlock(update.content);
  session.messages.push({
    id: crypto.randomUUID(),
    role: 'assistant',
    content: '',
    thought: update.content.type === 'text' ? update.content.text : '',
    blocks: update.content.type === 'text' ? undefined : [cloneContentBlockImpl(update.content)],
    timestamp: Date.now(),
    toolCalls: [],
  });
}
