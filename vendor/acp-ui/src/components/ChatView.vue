<script setup lang="ts">
import { ref, computed, nextTick, onMounted, watch } from 'vue';
import { marked } from 'marked';
import { open } from '@tauri-apps/plugin-dialog';
import { readFile, readTextFile } from '@tauri-apps/plugin-fs';
import { resolve } from '@tauri-apps/api/path';
import { useSessionStore } from '../stores/session';
import { useMermaid } from '../composables/useMermaid';
import { isAbsolutePath } from '../lib/acp-bridge';
import ModePicker from './ModePicker.vue';
import ModelPicker from './ModelPicker.vue';
import ConfigOptionPicker from './ConfigOptionPicker.vue';
import CommandPalette from './CommandPalette.vue';
import PlanView from './PlanView.vue';
import type {
  AvailableCommand,
  ChatMessage,
  ContentBlock,
  ToolCallContentBlock,
  ToolCallInfo,
} from '../lib/types';

type PendingAttachment = {
  id: string;
  kind: 'image' | 'audio';
  label: string;
  block: ContentBlock;
};

const EMBEDDED_CONTEXT_MENTION = /(^|[\s(])@(?:"([^"\n]+)"|([^\s@]+))/g;
const IMAGE_MIME_TYPES: Record<string, string> = {
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.gif': 'image/gif',
  '.webp': 'image/webp',
  '.svg': 'image/svg+xml',
  '.bmp': 'image/bmp',
  '.ico': 'image/x-icon',
  '.tif': 'image/tiff',
  '.tiff': 'image/tiff',
};
const AUDIO_MIME_TYPES: Record<string, string> = {
  '.mp3': 'audio/mpeg',
  '.wav': 'audio/wav',
  '.ogg': 'audio/ogg',
  '.m4a': 'audio/mp4',
  '.aac': 'audio/aac',
  '.flac': 'audio/flac',
  '.opus': 'audio/opus',
  '.weba': 'audio/webm',
};

const sessionStore = useSessionStore();
const inputText = ref('');
const activeCommandHint = ref<string | null>(null);
const messagesContainer = ref<HTMLElement | null>(null);
const commandPaletteRef = ref<InstanceType<typeof CommandPalette> | null>(null);
const pendingAttachments = ref<PendingAttachment[]>([]);
const attachmentError = ref<string | null>(null);
const { renderMermaidBlocks } = useMermaid();

// Track expanded thought sections by message id
const expandedThoughts = ref<Set<string>>(new Set());
const expandedToolCalls = ref<Set<string>>(new Set());

const messages = computed(() => sessionStore.messageList);
const isLoading = computed(() => sessionStore.isLoading);
const currentSession = computed(() => sessionStore.currentSession);
const availableModes = computed(() => sessionStore.availableModes);
const currentModeId = computed(() => sessionStore.currentModeId);
const availableModels = computed(() => sessionStore.availableModels);
const currentModelId = computed(() => sessionStore.currentModelId);
const configOptions = computed(() => sessionStore.configOptions);
const hasConfigOptions = computed(() => sessionStore.hasConfigOptions);
const availableCommands = computed(() => sessionStore.availableCommands);
const currentPlan = computed(() => sessionStore.currentPlan);
const pendingPermissionCount = computed(() => sessionStore.pendingPermissionCount);
const agentPromptCapabilities = computed(() => sessionStore.agentPromptCapabilities);
const supportsImageAttachments = computed(() => agentPromptCapabilities.value.image === true);
const supportsAudioAttachments = computed(() => agentPromptCapabilities.value.audio === true);
const supportsEmbeddedContext = computed(
  () => agentPromptCapabilities.value.embeddedContext === true
);
const imageAttachmentTooltip = computed(() =>
  supportsImageAttachments.value
    ? 'Attach image'
    : 'This agent does not support image attachments.'
);
const audioAttachmentTooltip = computed(() =>
  supportsAudioAttachments.value
    ? 'Attach audio'
    : 'This agent does not support audio attachments.'
);
const embeddedContextTooltip = computed(() =>
  supportsEmbeddedContext.value
    ? 'Type @README.md or @"notes/file name.md" to embed a file.'
    : 'This agent does not support embedded file context.'
);

// Slash command state
const showCommandPalette = computed(() => {
  if (availableCommands.value.length === 0) return false;
  const text = inputText.value;
  // Show palette when input starts with "/" and cursor is after it
  if (!text.startsWith('/')) return false;
  // Don't show if there's a space (command already entered)
  const spaceIndex = text.indexOf(' ');
  return spaceIndex === -1;
});

const commandFilter = computed(() => {
  if (!inputText.value.startsWith('/')) return '';
  return inputText.value.slice(1); // Remove the leading "/"
});

// Auto-scroll to bottom when new messages arrive
watch(messages, async () => {
  await nextTick();
  if (messagesContainer.value) {
    messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight;
    // Only render mermaid when not streaming — incomplete fences get permanently
    // marked as processed and won't re-render when the full definition arrives.
    if (!isLoading.value) {
      await renderMermaidBlocks(messagesContainer.value);
    }
  }
}, { deep: true });

onMounted(async () => {
  await nextTick();
  if (messagesContainer.value) {
    await renderMermaidBlocks(messagesContainer.value);
  }
});

watch(inputText, (text) => {
  if (!text.startsWith('/')) {
    activeCommandHint.value = null;
  }
});

async function handleSend() {
  const text = inputText.value.trim();
  if (!text || isLoading.value) return;

  attachmentError.value = null;
  try {
    const attachments = [
      ...pendingAttachments.value.map((attachment) => attachment.block),
      ...(await buildEmbeddedContextAttachments(text)),
    ];
    await sessionStore.sendPrompt(text, attachments);
    inputText.value = '';
    pendingAttachments.value = [];
  } catch (e) {
    attachmentError.value = e instanceof Error ? e.message : 'Failed to send prompt.';
    console.error('Failed to send prompt:', e);
  }
}

function handleKeyDown(event: KeyboardEvent) {
  // Let CommandPalette handle navigation keys when visible
  if (showCommandPalette.value && commandPaletteRef.value) {
    if (['ArrowDown', 'ArrowUp', 'Enter', 'Tab', 'Escape'].includes(event.key)) {
      commandPaletteRef.value.handleKeyDown(event);
      return;
    }
  }
  
  if (event.key === 'Enter' && !event.shiftKey) {
    event.preventDefault();
    handleSend();
  }
}

function handleCommandSelect(command: AvailableCommand) {
  inputText.value = `/${command.name} `;
  activeCommandHint.value = command.input?.hint ?? null;
}

function handleCommandClose() {
  // Just dismiss, keep the text
}

function handleCancel() {
  sessionStore.cancelOperation();
}

async function handleModeChange(modeId: string) {
  try {
    await sessionStore.setMode(modeId);
  } catch (e) {
    console.error('Failed to change mode:', e);
  }
}

async function handleModelChange(modelId: string) {
  try {
    await sessionStore.setModel(modelId);
  } catch (e) {
    console.error('Failed to change model:', e);
  }
}

async function handleConfigOptionChange(configId: string, value: string) {
  try {
    await sessionStore.setConfigOption(configId, value);
  } catch (e) {
    console.error('Failed to change config option:', e);
  }
}

function isThoughtExpanded(messageId: string): boolean {
  return expandedThoughts.value.has(messageId);
}

async function rerenderExpandedMarkdown(): Promise<void> {
  await nextTick();
  if (messagesContainer.value) {
    await renderMermaidBlocks(messagesContainer.value);
  }
}

async function toggleThought(messageId: string): Promise<void> {
  if (expandedThoughts.value.has(messageId)) {
    expandedThoughts.value.delete(messageId);
  } else {
    expandedThoughts.value.add(messageId);
    await rerenderExpandedMarkdown();
  }
}

function isToolCallExpanded(toolCallId: string): boolean {
  return expandedToolCalls.value.has(toolCallId);
}

async function toggleToolCall(toolCallId: string): Promise<void> {
  if (expandedToolCalls.value.has(toolCallId)) {
    expandedToolCalls.value.delete(toolCallId);
    return;
  }

  expandedToolCalls.value.add(toolCallId);
  await rerenderExpandedMarkdown();
}

function renderMarkdown(content: string): string {
  return marked.parse(content, { async: false }) as string;
}

function hasToolCallDetails(toolCall: ToolCallInfo): boolean {
  return Boolean(
    toolCall.content?.length ||
    toolCall.rawInput ||
    toolCall.rawOutput
  );
}

function getToolCallLocationLabel(toolCall: ToolCallInfo): string | null {
  const firstLocation = toolCall.locations?.[0];
  if (!firstLocation) {
    return null;
  }

  return firstLocation.line != null
    ? `${firstLocation.path}:${firstLocation.line}`
    : firstLocation.path;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

type InlineTextToolCallContentBlock = Extract<ToolCallContentBlock, { type: 'content' }> & {
  content: Extract<ContentBlock, { type: 'text' }>;
};

function isInlineTextToolCallContentBlock(
  block: ToolCallContentBlock
): block is InlineTextToolCallContentBlock {
  if (block.type !== 'content' || !isRecord(block.content)) {
    return false;
  }

  return block.content.type === 'text' && typeof block.content.text === 'string';
}

type DiffLine = {
  kind: 'header' | 'context' | 'add' | 'remove';
  text: string;
};

function isDiffToolCallContentBlock(
  block: ToolCallContentBlock
): block is Extract<ToolCallContentBlock, { type: 'diff' }> {
  return block.type === 'diff';
}

function isTerminalToolCallContentBlock(
  block: ToolCallContentBlock
): block is Extract<ToolCallContentBlock, { type: 'terminal' }> {
  return block.type === 'terminal';
}

function splitDiffLines(text: string | null | undefined): string[] {
  return (text ?? '').split('\n');
}

function buildDiffOperations(oldLines: string[], newLines: string[]): DiffLine[] {
  const lengths = Array.from(
    { length: oldLines.length + 1 },
    () => Array<number>(newLines.length + 1).fill(0)
  );

  for (let oldIndex = oldLines.length - 1; oldIndex >= 0; oldIndex -= 1) {
    for (let newIndex = newLines.length - 1; newIndex >= 0; newIndex -= 1) {
      lengths[oldIndex][newIndex] = oldLines[oldIndex] === newLines[newIndex]
        ? lengths[oldIndex + 1][newIndex + 1] + 1
        : Math.max(lengths[oldIndex + 1][newIndex], lengths[oldIndex][newIndex + 1]);
    }
  }

  const diffLines: DiffLine[] = [];
  let oldIndex = 0;
  let newIndex = 0;

  while (oldIndex < oldLines.length && newIndex < newLines.length) {
    if (oldLines[oldIndex] === newLines[newIndex]) {
      diffLines.push({ kind: 'context', text: ` ${oldLines[oldIndex]}` });
      oldIndex += 1;
      newIndex += 1;
      continue;
    }

    if (lengths[oldIndex + 1][newIndex] >= lengths[oldIndex][newIndex + 1]) {
      diffLines.push({ kind: 'remove', text: `-${oldLines[oldIndex]}` });
      oldIndex += 1;
    } else {
      diffLines.push({ kind: 'add', text: `+${newLines[newIndex]}` });
      newIndex += 1;
    }
  }

  while (oldIndex < oldLines.length) {
    diffLines.push({ kind: 'remove', text: `-${oldLines[oldIndex]}` });
    oldIndex += 1;
  }

  while (newIndex < newLines.length) {
    diffLines.push({ kind: 'add', text: `+${newLines[newIndex]}` });
    newIndex += 1;
  }

  return diffLines;
}

function getUnifiedDiffLines(block: Extract<ToolCallContentBlock, { type: 'diff' }>): DiffLine[] {
  return [
    {
      kind: 'header',
      text: `--- ${block.oldText == null ? '/dev/null' : block.path}`,
    },
    {
      kind: 'header',
      text: `+++ ${block.path}`,
    },
    ...buildDiffOperations(splitDiffLines(block.oldText), splitDiffLines(block.newText)),
  ];
}

function getDiffLineClass(line: DiffLine): string {
  switch (line.kind) {
    case 'add':
      return 'diff-line-add';
    case 'remove':
      return 'diff-line-remove';
    case 'header':
      return 'diff-line-header';
    default:
      return 'diff-line-context';
  }
}

function formatJsonPayload(payload: Record<string, unknown>): string {
  return JSON.stringify(payload, null, 2);
}

function formatUnknownToolContent(block: ToolCallContentBlock): string {
  return JSON.stringify(block, null, 2);
}

function getTerminalStatusLabel(block: Extract<ToolCallContentBlock, { type: 'terminal' }>): string | null {
  const exitStatus = isRecord(block.exitStatus) ? block.exitStatus : null;
  const exitCode =
    typeof exitStatus?.exitCode === 'number' ? exitStatus.exitCode : null;
  const signal =
    typeof exitStatus?.signal === 'string' && exitStatus.signal.length > 0
      ? exitStatus.signal
      : null;

  if (signal) {
    return `signal: ${signal}`;
  }

  if (exitCode != null) {
    return `exit ${exitCode}`;
  }

  if (block.released === true) {
    return 'released';
  }

  return null;
}

function getToolIcon(kind: string): string {
  switch (kind) {
    case 'read': return '📖';
    case 'edit': return '✏️';
    case 'delete': return '🗑️';
    case 'move': return '📦';
    case 'search': return '🔍';
    case 'execute': return '▶️';
    case 'think': return '💭';
    case 'fetch': return '🌐';
    default: return '🔧';
  }
}

function getStatusIcon(status: string): string {
  switch (status) {
    case 'pending': return '⏳';
    case 'in_progress': return '⚙️';
    case 'completed': return '✓';
    case 'failed': return '✗';
    case 'cancelled': return '⊘';
    default: return '';
  }
}

function getMessageToolCalls(message: ChatMessage) {
  return message.toolCalls ?? [];
}

function getMessageBlocks(message: ChatMessage): ContentBlock[] {
  return message.blocks ?? [];
}

function shouldRenderLegacyContent(message: ChatMessage): boolean {
  return Boolean(message.content) && !getMessageBlocks(message).some((block) => block.type === 'text');
}

function getImageSrc(block: ContentBlock): string {
  return block.type === 'image' ? `data:${block.mimeType};base64,${block.data}` : '';
}

function getAudioSrc(block: ContentBlock): string {
  return block.type === 'audio' ? `data:${block.mimeType};base64,${block.data}` : '';
}

function getResourceLinkLabel(block: ContentBlock): string {
  if (block.type !== 'resource_link') {
    return '';
  }

  // Spec: prefer title over name.
  return block.title || block.name || block.uri;
}

function getResourceLinkDescription(block: ContentBlock): string | null {
  if (block.type !== 'resource_link') {
    return null;
  }

  if (typeof block.description !== 'string') {
    return null;
  }

  const trimmed = block.description.trim();
  return trimmed.length > 0 ? trimmed : null;
}

function formatFileSize(bytes: number | bigint): string {
  const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB'];

  // BigInt path (ACP int64)
  if (typeof bytes === 'bigint') {
    if (bytes < 1024n) {
      return `${bytes} B`;
    }

    let unitIndex = 0;
    let value = bytes;
    while (value >= 1024n && unitIndex < units.length - 1) {
      value = value / 1024n;
      unitIndex += 1;
    }

    return `${value} ${units[unitIndex]}`;
  }

  // Number path
  if (!Number.isFinite(bytes) || bytes < 0) {
    return '';
  }

  if (bytes < 1024) {
    return `${Math.round(bytes)} B`;
  }

  let unitIndex = 0;
  let value = bytes;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }

  const rounded = value < 10 ? value.toFixed(1) : Math.round(value).toString();
  return `${rounded} ${units[unitIndex]}`;
}

function getResourceLinkSizeLabel(block: ContentBlock): string | null {
  if (block.type !== 'resource_link') {
    return null;
  }

  if (block.size == null) {
    return null;
  }

  return (typeof block.size === 'number' || typeof block.size === 'bigint')
    ? formatFileSize(block.size)
    : null;
}

function getResourceLinkTooltip(block: ContentBlock): string | null {
  if (block.type !== 'resource_link') {
    return null;
  }

  const description = getResourceLinkDescription(block);
  const size = getResourceLinkSizeLabel(block);
  const parts = [description, size].filter((part): part is string => Boolean(part));

  return parts.length ? parts.join(' • ') : null;
}

function getResourceUri(block: ContentBlock): string {
  return block.type === 'resource' ? block.resource.uri : '';
}

function getResourceBody(block: ContentBlock): string {
  if (block.type !== 'resource') {
    return '';
  }

  return 'text' in block.resource ? block.resource.text : block.resource.blob;
}

function isToolOnlyAssistantMessage(message: ChatMessage): boolean {
  return (
    message.role === 'assistant' &&
    !message.content &&
    !getMessageBlocks(message).length &&
    getMessageToolCalls(message).length > 0
  );
}

function bytesToBase64(bytes: Uint8Array): string {
  let binary = '';
  const chunkSize = 0x8000;
  for (let index = 0; index < bytes.length; index += chunkSize) {
    const chunk = bytes.subarray(index, index + chunkSize);
    binary += String.fromCharCode(...chunk);
  }
  return btoa(binary);
}

function getFileLabel(path: string): string {
  const normalized = path.replace(/\\/g, '/');
  const segments = normalized.split('/');
  return segments[segments.length - 1] || path;
}

function getMimeType(path: string, kind: 'image' | 'audio'): string {
  const extensionIndex = path.lastIndexOf('.');
  const extension = extensionIndex >= 0 ? path.slice(extensionIndex).toLowerCase() : '';
  const knownMimeType =
    kind === 'image' ? IMAGE_MIME_TYPES[extension] : AUDIO_MIME_TYPES[extension];
  if (knownMimeType) {
    return knownMimeType;
  }

  return kind === 'image' ? 'image/png' : 'audio/mpeg';
}

function normalizeDialogSelection(selection: string | string[] | null): string[] {
  if (!selection) {
    return [];
  }

  return Array.isArray(selection) ? selection : [selection];
}

async function addPendingAttachments(kind: 'image' | 'audio'): Promise<void> {
  if (isLoading.value) {
    return;
  }

  if (kind === 'image' && !supportsImageAttachments.value) {
    return;
  }

  if (kind === 'audio' && !supportsAudioAttachments.value) {
    return;
  }

  attachmentError.value = null;

  try {
    const selection = await open({
      multiple: true,
      title: kind === 'image' ? 'Select image attachments' : 'Select audio attachments',
      filters: [
        kind === 'image'
          ? {
              name: 'Images',
              extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg', 'bmp', 'ico', 'tif', 'tiff'],
            }
          : {
              name: 'Audio',
              extensions: ['mp3', 'wav', 'ogg', 'm4a', 'aac', 'flac', 'opus', 'weba'],
            },
      ],
    });

    const nextPaths = normalizeDialogSelection(selection);
    if (!nextPaths.length) {
      return;
    }

    const nextAttachments = await Promise.all(
      nextPaths.map(async (path) => {
        const data = await readFile(path);
        const label = getFileLabel(path);
        const mimeType = getMimeType(path, kind);
        const block: ContentBlock =
          kind === 'image'
            ? {
                type: 'image',
                data: bytesToBase64(data),
                mimeType,
                uri: path,
              }
            : {
                type: 'audio',
                data: bytesToBase64(data),
                mimeType,
              };

        return {
          id: crypto.randomUUID(),
          kind,
          label,
          block,
        } satisfies PendingAttachment;
      })
    );

    pendingAttachments.value = [...pendingAttachments.value, ...nextAttachments];
  } catch (e) {
    attachmentError.value =
      e instanceof Error ? e.message : `Failed to load ${kind} attachment(s).`;
    console.error(`Failed to load ${kind} attachment(s):`, e);
  }
}

function removePendingAttachment(attachmentId: string): void {
  pendingAttachments.value = pendingAttachments.value.filter(
    (attachment) => attachment.id !== attachmentId
  );
}

function isPathLikeMention(candidate: string): boolean {
  return (
    candidate.includes('/') ||
    candidate.includes('\\') ||
    candidate.includes('.') ||
    candidate.startsWith('~')
  );
}

function normalizeMentionPath(candidate: string): string {
  return candidate.trim().replace(/[),.;!?]+$/, '');
}

function insertEmbeddedContextHint(): void {
  if (!supportsEmbeddedContext.value || isLoading.value) {
    return;
  }

  const snippet = '@"path/to/file"';
  inputText.value = inputText.value.trim().length
    ? `${inputText.value} ${snippet}`
    : `${snippet} `;
}

async function buildEmbeddedContextAttachments(text: string): Promise<ContentBlock[]> {
  if (!supportsEmbeddedContext.value) {
    return [];
  }

  const cwd = currentSession.value?.cwd;
  if (!cwd) {
    return [];
  }

  const attachments: ContentBlock[] = [];
  const seenPaths = new Set<string>();

  for (const match of text.matchAll(EMBEDDED_CONTEXT_MENTION)) {
    const rawCandidate = match[2] ?? match[3];
    if (!rawCandidate) {
      continue;
    }

    const candidate = normalizeMentionPath(rawCandidate);
    if (!candidate || !isPathLikeMention(candidate)) {
      continue;
    }

    const resolvedPath = isAbsolutePath(candidate) ? candidate : await resolve(cwd, candidate);
    if (seenPaths.has(resolvedPath)) {
      continue;
    }

    try {
      const content = await readTextFile(resolvedPath);
      attachments.push({
        type: 'resource',
        resource: {
          uri: resolvedPath,
          text: content,
          mimeType: 'text/plain',
        },
      });
      seenPaths.add(resolvedPath);
    } catch (e) {
      const fileLabel = getFileLabel(candidate);
      throw new Error(
        `Failed to embed file context for ${fileLabel}: ${
          e instanceof Error ? e.message : 'Unable to read file'
        }`
      );
    }
  }

  return attachments;
}
</script>

<template>
  <div class="chat-view">
    <div class="chat-header">
      <h2>{{ currentSession?.title || 'Chat' }}</h2>
      <div class="header-right">
        <template v-if="hasConfigOptions">
          <ConfigOptionPicker
            v-for="option in configOptions"
            :key="option.id"
            :option="option"
            @change="handleConfigOptionChange"
          />
        </template>
        <template v-else>
          <ModelPicker 
            v-if="availableModels.length > 0"
            :models="availableModels"
            :current-model-id="currentModelId"
            @change="handleModelChange"
          />
          <ModePicker 
            v-if="availableModes.length > 0"
            :modes="availableModes"
            :current-mode-id="currentModeId"
            @change="handleModeChange"
          />
        </template>
        <span class="agent-name">{{ currentSession?.agentName }}</span>
      </div>
    </div>
    
    <div ref="messagesContainer" class="messages-container">
        <PlanView v-if="currentPlan" :entries="currentPlan" />
        <div 
          v-for="message in messages" 
          :key="message.id"
          :class="[
            'message',
            `message-${message.role}`,
            { 'message-tool-only': isToolOnlyAssistantMessage(message) },
          ]"
        >
        <div class="message-header">
          <span class="role">{{ message.role === 'user' ? 'You' : 'Assistant' }}</span>
        </div>
        
        <!-- Agent thinking section (collapsible) - shown first to explain reasoning -->
        <div v-if="message.thought && message.role === 'assistant'" class="thought-section">
          <button class="thought-toggle" @click="toggleThought(message.id)">
            <span class="thought-icon">💭</span>
            <span class="thought-label">{{ isThoughtExpanded(message.id) ? 'Hide Thinking' : 'Show Thinking' }}</span>
            <span class="thought-chevron">{{ isThoughtExpanded(message.id) ? '▲' : '▼' }}</span>
          </button>
          <div v-if="isThoughtExpanded(message.id)" class="thought-content">
            <div v-html="renderMarkdown(message.thought)" />
          </div>
        </div>
        
        <!-- Tool calls for this message (shown after thinking) -->
        <div v-if="getMessageToolCalls(message).length" class="tool-calls-section">
          <div 
            v-for="tc in getMessageToolCalls(message)" 
            :key="tc.toolCallId"
            :class="['tool-call-inline', `tool-${tc.status}`]"
          >
            <button
              type="button"
              class="tool-call-summary"
              :class="{ 'tool-call-summary-expandable': hasToolCallDetails(tc) }"
              @click="hasToolCallDetails(tc) && toggleToolCall(tc.toolCallId)"
            >
              <span class="tool-icon">{{ getToolIcon(tc.kind) }}</span>
              <span class="tool-name">{{ tc.title }}</span>
              <span v-if="getToolCallLocationLabel(tc)" class="tool-location">
                {{ getToolCallLocationLabel(tc) }}
              </span>
              <span :class="['tool-status', `status-${tc.status}`]">
                {{ getStatusIcon(tc.status) }}
              </span>
              <span
                v-if="hasToolCallDetails(tc)"
                class="tool-chevron"
              >
                {{ isToolCallExpanded(tc.toolCallId) ? '▲' : '▼' }}
              </span>
            </button>

            <div
              v-if="hasToolCallDetails(tc) && isToolCallExpanded(tc.toolCallId)"
              class="tool-call-details"
            >
              <div
                v-if="tc.content?.length"
                class="tool-call-content-list"
              >
                <div
                  v-for="(block, blockIndex) in tc.content"
                  :key="`${tc.toolCallId}-content-${blockIndex}`"
                  class="tool-content-block"
                >
                  <div
                    v-if="isInlineTextToolCallContentBlock(block)"
                    class="tool-content-markdown"
                    v-html="renderMarkdown(block.content.text)"
                  />
                  <div
                    v-else-if="isDiffToolCallContentBlock(block)"
                    class="tool-content-diff"
                  >
                    <div class="tool-content-label">{{ block.path }}</div>
                    <pre class="tool-diff-pre"><code><span
                      v-for="(line, lineIndex) in getUnifiedDiffLines(block)"
                      :key="`${tc.toolCallId}-diff-${blockIndex}-${lineIndex}`"
                      :class="['tool-diff-line', getDiffLineClass(line)]"
                    >{{ line.text }}
</span></code></pre>
                  </div>
                  <div
                    v-else-if="isTerminalToolCallContentBlock(block)"
                    class="tool-content-terminal"
                  >
                    <div class="tool-content-label">
                      Terminal {{ block.terminalId }}
                      <span v-if="getTerminalStatusLabel(block)" class="tool-terminal-status">
                        · {{ getTerminalStatusLabel(block) }}
                      </span>
                    </div>
                    <pre
                      v-if="typeof block.output === 'string' && block.output.length > 0"
                      class="tool-terminal-output"
                    ><code>{{ block.output }}</code></pre>
                    <p v-else class="tool-terminal-placeholder">
                      Live terminal output will appear here once terminal rendering lands.
                    </p>
                    <p
                      v-if="block.truncated === true"
                      class="tool-terminal-meta"
                    >
                      Output truncated by the terminal byte limit.
                    </p>
                  </div>
                  <pre v-else class="tool-content-unknown"><code>{{ formatUnknownToolContent(block) }}</code></pre>
                </div>
              </div>

              <details v-if="tc.rawInput" class="tool-call-payload">
                <summary>Raw input</summary>
                <pre><code>{{ formatJsonPayload(tc.rawInput) }}</code></pre>
              </details>

              <details v-if="tc.rawOutput" class="tool-call-payload">
                <summary>Raw output</summary>
                <pre><code>{{ formatJsonPayload(tc.rawOutput) }}</code></pre>
              </details>
            </div>
          </div>
        </div>
        
        <div
          v-for="(block, index) in getMessageBlocks(message)"
          :key="`${message.id}-block-${index}`"
          class="message-content-block"
        >
          <div
            v-if="block.type === 'text'"
            class="message-content"
            v-html="renderMarkdown(block.text)"
          />
          <img
            v-else-if="block.type === 'image'"
            class="message-image"
            :src="getImageSrc(block)"
            :alt="block.uri ?? 'Message image'"
          />
          <audio
            v-else-if="block.type === 'audio'"
            class="message-audio"
            controls
            :src="getAudioSrc(block)"
          />
          <a
            v-else-if="block.type === 'resource_link'"
            class="resource-link-chip"
            :href="block.uri"
            target="_blank"
            rel="noreferrer noopener"
            :title="getResourceLinkTooltip(block) || undefined"
          >
            <span class="resource-link-chip__content">
              <span class="resource-link-chip__label">{{ getResourceLinkLabel(block) }}</span>
              <span
                v-if="getResourceLinkDescription(block) || getResourceLinkSizeLabel(block)"
                class="resource-link-chip__meta"
              >
                <span
                  v-if="getResourceLinkDescription(block)"
                  class="resource-link-chip__description"
                >
                  {{ getResourceLinkDescription(block) }}
                </span>
                <span v-if="getResourceLinkSizeLabel(block)" class="resource-link-chip__size">
                  {{ getResourceLinkSizeLabel(block) }}
                </span>
              </span>
            </span>
            <span class="resource-link-uri">{{ block.uri }}</span>
          </a>
          <div v-else-if="block.type === 'resource'" class="resource-block">
            <div class="resource-label">{{ getResourceUri(block) }}</div>
            <pre class="resource-content"><code>{{ getResourceBody(block) }}</code></pre>
          </div>
        </div>

        <div
          v-if="shouldRenderLegacyContent(message)"
          class="message-content"
          v-html="renderMarkdown(message.content)"
        />
      </div>
      
      <div v-if="pendingPermissionCount > 0" class="permission-status">
        {{ pendingPermissionCount }}
        permission request{{ pendingPermissionCount === 1 ? '' : 's' }} awaiting review.
      </div>

      <!-- Loading indicator -->
      <div v-if="isLoading" class="loading-indicator">
        <span class="spinner"></span>
        <span>Thinking...</span>
        <button class="cancel-btn" @click="handleCancel">Cancel</button>
      </div>
    </div>
    
    <div class="input-container">
      <CommandPalette
        ref="commandPaletteRef"
        :commands="availableCommands"
        :filter="commandFilter"
        :visible="showCommandPalette"
        @select="handleCommandSelect"
        @close="handleCommandClose"
      />
      <div class="composer">
        <div class="composer-toolbar">
          <span class="composer-control-wrapper" :title="imageAttachmentTooltip">
            <button
              type="button"
              class="attachment-btn"
              :disabled="!supportsImageAttachments || isLoading"
              @click="addPendingAttachments('image')"
            >
              🖼️ Image
            </button>
          </span>
          <span class="composer-control-wrapper" :title="audioAttachmentTooltip">
            <button
              type="button"
              class="attachment-btn"
              :disabled="!supportsAudioAttachments || isLoading"
              @click="addPendingAttachments('audio')"
            >
              🎧 Audio
            </button>
          </span>
          <span class="composer-control-wrapper" :title="embeddedContextTooltip">
            <button
              type="button"
              class="attachment-btn"
              :disabled="!supportsEmbeddedContext || isLoading"
              @click="insertEmbeddedContextHint"
            >
              @ File
            </button>
          </span>
        </div>

        <div v-if="pendingAttachments.length" class="pending-attachments">
          <div
            v-for="attachment in pendingAttachments"
            :key="attachment.id"
            class="pending-attachment-chip"
          >
            <span>{{ attachment.kind === 'image' ? '🖼️' : '🎧' }} {{ attachment.label }}</span>
            <button
              type="button"
              class="pending-attachment-remove"
              :disabled="isLoading"
              @click="removePendingAttachment(attachment.id)"
            >
              ×
            </button>
          </div>
        </div>

        <div
          class="embedded-context-hint"
          :title="embeddedContextTooltip"
          :aria-disabled="!supportsEmbeddedContext"
        >
          {{
            supportsEmbeddedContext
              ? 'Embed file context with @README.md or @"notes/file name.md".'
              : 'Embedded file context is unavailable for this agent.'
          }}
        </div>

        <div v-if="attachmentError" class="attachment-error">{{ attachmentError }}</div>

        <textarea
          v-model="inputText"
          :placeholder="activeCommandHint ?? 'Type your message...'"
          :disabled="isLoading"
          @keydown="handleKeyDown"
          rows="3"
        />
      </div>
      <button 
        class="send-btn"
        :disabled="!inputText.trim() || isLoading"
        @click="handleSend"
      >
        Send
      </button>
    </div>
  </div>
</template>

<style scoped>
.chat-view {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.chat-header {
  padding: 1rem;
  border-bottom: 1px solid var(--border-color, #e0e0e0);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.chat-header h2 {
  margin: 0;
  font-size: 1.1rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.header-right {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 0.75rem;
}

.agent-name {
  font-size: 0.875rem;
  color: var(--text-accent, #0066cc);
}

.messages-container {
  flex: 1;
  overflow-y: auto;
  padding: 1rem;
}

.permission-status {
  margin-top: 1rem;
  padding: 0.75rem 1rem;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: var(--bg-warning, #fff3cd);
  color: var(--text-primary);
}

.message {
  margin-bottom: 1rem;
  padding: 0.75rem;
  border-radius: 8px;
}

.message-user {
  background: var(--bg-user, #e3f2fd);
  margin-left: 2rem;
}

.message-assistant {
  background: var(--bg-assistant, #f5f5f5);
  margin-right: 2rem;
}

.message-header {
  margin-bottom: 0.5rem;
}

.role {
  font-weight: 600;
  font-size: 0.875rem;
  color: var(--text-secondary, #666);
}

/* Tool calls inline styles */
.tool-calls-section {
  margin-bottom: 0.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.tool-call-inline {
  display: flex;
  flex-direction: column;
  border-radius: 4px;
  font-size: 0.8rem;
  background: rgba(0, 0, 0, 0.04);
  border-left: 2px solid var(--border-color);
  overflow: hidden;
}

.tool-call-summary {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  width: 100%;
  padding: 0.375rem 0.625rem;
  border: none;
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
}

.tool-call-summary-expandable {
  cursor: pointer;
}

.tool-call-summary-expandable:hover {
  background: rgba(255, 255, 255, 0.3);
}

.tool-pending {
  border-left-color: #f59e0b;
}

.tool-in_progress {
  border-left-color: #3b82f6;
  background: rgba(59, 130, 246, 0.08);
}

.tool-completed {
  border-left-color: #10b981;
  background: rgba(16, 185, 129, 0.08);
}

.tool-failed {
  border-left-color: #ef4444;
  background: rgba(239, 68, 68, 0.08);
}

.tool-cancelled {
  border-left-color: #6b7280;
  background: rgba(107, 114, 128, 0.08);
}

.tool-icon {
  font-size: 0.875rem;
}

.tool-name {
  font-weight: 500;
  color: var(--text-primary);
}

.tool-location {
  flex: 1;
  color: var(--text-muted);
  font-size: 0.75rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tool-status {
  font-size: 0.75rem;
  font-weight: 600;
}

.tool-chevron {
  color: var(--text-muted, #666);
  font-size: 0.7rem;
}

.tool-call-details {
  padding: 0 0.625rem 0.625rem;
  display: flex;
  flex-direction: column;
  gap: 0.625rem;
}

.tool-call-content-list {
  display: flex;
  flex-direction: column;
  gap: 0.625rem;
}

.tool-content-block {
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.6);
  border: 1px solid rgba(148, 163, 184, 0.25);
  padding: 0.625rem;
}

.tool-content-label {
  margin-bottom: 0.375rem;
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--text-secondary, #555);
}

.tool-content-markdown {
  line-height: 1.5;
  overflow-wrap: break-word;
  word-wrap: break-word;
}

.tool-content-markdown :deep(p) {
  margin: 0.5rem 0;
}

.tool-content-markdown :deep(pre),
.tool-content-unknown,
.tool-diff-pre,
.tool-terminal-output,
.tool-call-payload pre {
  margin: 0;
  padding: 0.75rem;
  border-radius: 6px;
  background: var(--bg-code, #282c34);
  color: var(--text-code, #abb2bf);
  overflow-x: auto;
}

.tool-content-unknown code,
.tool-diff-pre code,
.tool-terminal-output code,
.tool-call-payload code {
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 0.82rem;
}

.tool-diff-line {
  display: block;
  white-space: pre;
}

.diff-line-header {
  color: #93c5fd;
}

.diff-line-context {
  color: inherit;
}

.diff-line-add {
  color: #86efac;
}

.diff-line-remove {
  color: #fca5a5;
}

.tool-terminal-placeholder,
.tool-terminal-meta {
  margin: 0;
  color: var(--text-secondary, #555);
}

.tool-terminal-status {
  font-weight: 500;
}

.tool-call-payload {
  border: 1px solid rgba(148, 163, 184, 0.35);
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.45);
}

.tool-call-payload summary {
  cursor: pointer;
  padding: 0.5rem 0.75rem;
  font-weight: 600;
}

.tool-call-payload pre {
  margin: 0 0.75rem 0.75rem;
}

.status-pending { color: #f59e0b; }
.status-in_progress { color: #3b82f6; }
.status-completed { color: #10b981; }
.status-failed { color: #ef4444; }
.status-cancelled { color: #6b7280; }

.message-content {
  line-height: 1.5;
  overflow-wrap: break-word;
  word-wrap: break-word;
}

.message-content-block + .message-content-block {
  margin-top: 0.75rem;
}

.message-content :deep(p) {
  margin: 0.5rem 0;
}

.message-content :deep(ol),
.message-content :deep(ul) {
  margin: 0.5rem 0;
  padding-left: 1.5rem;
}

.message-content :deep(li) {
  margin: 0.25rem 0;
}

.message-content :deep(pre) {
  background: var(--bg-code, #282c34);
  color: var(--text-code, #abb2bf);
  padding: 0.75rem;
  border-radius: 4px;
  overflow-x: auto;
}

.message-content :deep(code) {
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 0.9rem;
}

.message-content :deep(table) {
  border-collapse: collapse;
  width: 100%;
  margin: 0.75rem 0;
  font-size: 0.875rem;
  overflow-x: auto;
  display: block;
}

.message-content :deep(th),
.message-content :deep(td) {
  border: 1px solid var(--border-color, #3d3d3d);
  padding: 0.5rem 0.75rem;
  text-align: left;
}

.message-content :deep(th) {
  background: var(--bg-hover, #2d2d2d);
  font-weight: 600;
}

.message-content :deep(tr:nth-child(even)) {
  background: var(--bg-main, #1e1e1e);
}

.message-content :deep(tr:hover) {
  background: var(--bg-hover, #2d2d2d);
}

.messages-container :deep(.mermaid-rendered) {
  margin: 0.75rem 0;
}

.messages-container :deep(.mermaid-rendered svg) {
  display: block;
  max-width: 100%;
  border-radius: 6px;
}

.messages-container :deep(pre.mermaid-error) {
  border: 1px solid rgba(239, 68, 68, 0.45);
}

.messages-container :deep(.mermaid-error-message) {
  margin-top: 0.5rem;
  color: #fca5a5;
  font-size: 0.8rem;
}

.message-image {
  display: block;
  max-width: min(100%, 32rem);
  max-height: 24rem;
  border-radius: 6px;
  border: 1px solid var(--border-color, #e0e0e0);
}

.message-audio {
  width: min(100%, 28rem);
}

.resource-link-chip {
  display: inline-flex;
  align-items: flex-start;
  gap: 0.375rem;
  padding: 0.375rem 0.75rem;
  border-radius: 999px;
  background: var(--bg-hover, #f0f4f8);
  border: 1px solid var(--border-color, #d0d7de);
  color: var(--text-accent, #0066cc);
  text-decoration: none;
  font-size: 0.85rem;
  font-weight: 500;
}

.resource-link-chip__content {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
  min-width: 0;
}

.resource-link-chip__label {
  line-height: 1.1;
}

.resource-link-chip__meta {
  display: inline-flex;
  gap: 0.375rem;
  font-size: 0.75rem;
  font-weight: 400;
  color: var(--text-muted, #666);
  line-height: 1.1;
}

.resource-link-chip__description {
  max-width: 16rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.resource-link-chip__size {
  white-space: nowrap;
}

.resource-link-chip:hover {
  text-decoration: underline;
}

.resource-link-uri {
  max-width: 18rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-muted, #666);
  font-weight: 400;
}

.resource-block {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.resource-label {
  font-size: 0.8rem;
  color: var(--text-muted, #666);
  word-break: break-all;
}

.resource-content {
  margin: 0;
  background: var(--bg-code, #282c34);
  color: var(--text-code, #abb2bf);
  padding: 0.75rem;
  border-radius: 4px;
  overflow-x: auto;
}

.loading-indicator {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem;
  color: var(--text-muted, #666);
}

.spinner {
  width: 16px;
  height: 16px;
  border: 2px solid var(--border-color, #ccc);
  border-top-color: var(--text-accent, #0066cc);
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.cancel-btn {
  margin-left: auto;
  padding: 0.25rem 0.5rem;
  border: 1px solid var(--border-color, #ccc);
  border-radius: 4px;
  background: transparent;
  font-size: 0.8rem;
  cursor: pointer;
}

.input-container {
  position: relative;
  display: flex;
  align-items: flex-end;
  gap: 0.5rem;
  padding: 1rem;
  border-top: 1px solid var(--border-color, #e0e0e0);
}

.composer {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.composer-toolbar {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.composer-control-wrapper {
  display: inline-flex;
}

.attachment-btn {
  border: 1px solid var(--border-color, #d0d7de);
  border-radius: 999px;
  background: var(--bg-hover, #f5f7fa);
  color: var(--text-primary, #1f2328);
  padding: 0.35rem 0.75rem;
  font-size: 0.85rem;
  cursor: pointer;
}

.attachment-btn:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.pending-attachments {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.pending-attachment-chip {
  display: inline-flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.25rem 0.625rem;
  border-radius: 999px;
  background: var(--bg-hover, #f0f4f8);
  border: 1px solid var(--border-color, #d0d7de);
  font-size: 0.8rem;
}

.pending-attachment-remove {
  border: none;
  background: transparent;
  color: var(--text-muted, #666);
  cursor: pointer;
  font-size: 1rem;
  line-height: 1;
  padding: 0;
}

.pending-attachment-remove:disabled {
  cursor: not-allowed;
}

.embedded-context-hint {
  font-size: 0.8rem;
  color: var(--text-muted, #666);
}

.embedded-context-hint[aria-disabled='true'] {
  opacity: 0.7;
}

.attachment-error {
  color: #b42318;
  font-size: 0.8rem;
}

textarea {
  flex: 1;
  padding: 0.75rem;
  border: 1px solid var(--border-color, #ccc);
  border-radius: 6px;
  font-size: 1rem;
  font-family: inherit;
  resize: none;
}

textarea:focus {
  outline: none;
  border-color: var(--text-accent, #0066cc);
}

.send-btn {
  padding: 0.75rem 1.5rem;
  background: var(--bg-primary, #0066cc);
  color: white;
  border: none;
  border-radius: 6px;
  font-size: 1rem;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.15s;
}

.send-btn:hover:not(:disabled) {
  background: var(--bg-primary-hover, #0052a3);
}

.send-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* Agent Thinking Section */
.thought-section {
  margin-bottom: 0.75rem;
  border: 1px solid var(--border-color, #e0e0e0);
  border-radius: 8px;
  overflow: hidden;
}

.thought-toggle {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  width: 100%;
  padding: 0.5rem 0.75rem;
  background: var(--bg-hover, #f5f5f5);
  border: none;
  cursor: pointer;
  font-size: 0.85rem;
  color: var(--text-muted, #666);
  text-align: left;
  transition: background 0.15s ease;
}

.thought-toggle:hover {
  background: var(--bg-user, #e3f2fd);
}

.thought-icon {
  font-size: 1rem;
  flex-shrink: 0;
}

.thought-label {
  flex: 1;
  font-weight: 500;
}

.thought-chevron {
  font-size: 0.7rem;
  color: var(--text-muted, #999);
}

.thought-content {
  padding: 0.75rem 1rem 0.75rem 1.25rem;
  background: var(--bg-main, #fafafa);
  border-top: 1px solid var(--border-color, #e0e0e0);
  font-size: 0.9rem;
  color: var(--text-muted, #666);
  font-style: italic;
  line-height: 1.5;
}

.thought-content :deep(p) {
  margin: 0 0 0.5rem 0;
}

.thought-content :deep(p:last-child) {
  margin-bottom: 0;
}

.thought-content :deep(code) {
  background: var(--bg-hover, #f0f0f0);
  padding: 0.125rem 0.25rem;
  border-radius: 3px;
  font-size: 0.85em;
}
</style>
